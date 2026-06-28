//! Accounts, organizations & access routes (EPIC-ACC-01 / UC-ACC-01). OWNED BY: BE-Accounts.
//!
//! Resource-server handlers: extract `AuthUser` + `RlsConnection` (validates the
//! api-server-issued JWT, resolves tenant scope), enforce RBAC via
//! `accounting_core::authz`, then call `db::repositories::acc_accounts`.
//! `nest`ed under `/api/v1/accounts` in `routes/mod.rs`.
//!
//! TENANT MODEL (architecture option C): an ACC "company/agenda" IS an existing
//! `organizations` row; user/accountant access IS org membership. This server
//! ISSUES NO tokens — it validates JWTs issued by api-server and enforces
//! per-company isolation via RLS (`find_company_rls`) + server-side role checks
//! (`accounting_core::authz`). Cross-company access returns nothing (RLS) / 403.
//!
//! ## Router wiring note
//! Foundation's `routes/mod.rs` wires only `GET /companies`,
//! `GET /companies/{id}`, `POST /switch-company` for this epic. The remaining
//! handlers (create company, invite/role/deactivate user, grant/revoke
//! accountant, billing) are fully implemented here and ready to wire; Phase 2
//! adds their routes + registers the new DTOs in `main.rs` (see
//! `contract-notes/be-accounts.md`).

use api_core::extractors::{AuthUser, RlsConnection};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use db::repositories::UserRepository;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use accounting_core::audit;
use accounting_core::authz;

use crate::state::AppState;

// ============================================================================
// Plan / entitlement gate stubs (UC-ACC-01.8). The full subscription system is
// out of MVP scope (architecture defers it); these constants gate creation so
// the seat/company-limit acceptance criteria are enforceable. Phase 2 / a later
// epic swaps these for a real entitlement lookup keyed by the account's plan.
// ============================================================================

/// Max companies a single owner may create on the default MVP plan
/// (UC-ACC-01.2/.8). Exceeding it blocks creation with an upgrade prompt.
const DEFAULT_PLAN_MAX_COMPANIES: i64 = 5;
/// Max active members (incl. pending invites) per company on the default plan
/// (UC-ACC-01.3/.8). Exceeding it blocks invites with an upgrade prompt.
const DEFAULT_PLAN_MAX_MEMBERS_PER_COMPANY: i64 = 10;

// ----------------------------------------------------------------------------
// Role mapping (UC-ACC-01.5). The story's user-facing roles map onto the
// canonical `organization_members.role_type` strings the resource server
// resolves into `common::TenantRole`:
//   * Admin     -> "org_admin"  (TenantRole::OrgAdmin)  — manage members/settings
//   * Standard  -> "manager"    (TenantRole::Manager)   — issue/edit documents
//   * Read-only -> "read_only"  (resolves to Guest today; read+export intent)
// NOTE (contract): accounting authz READ_MIN_ROLE == MUTATE_MIN_ROLE == Manager,
// so a true below-Manager read-only/export-only tier needs a dedicated ACC RBAC
// mechanism. Filed in contract-notes/be-accounts.md. For the MVP an Owner who
// wants an accountant to actually READ documents grants "manager".
// ----------------------------------------------------------------------------

/// The role strings UC-ACC-01.5 exposes; validated before persisting so a
/// caller cannot inject an arbitrary `role_type`.
fn normalize_role(input: &str) -> Option<&'static str> {
    match input.trim().to_ascii_lowercase().as_str() {
        "admin" | "org_admin" => Some("org_admin"),
        "standard" | "manager" => Some("manager"),
        "read-only" | "read_only" | "readonly" => Some("read_only"),
        _ => None,
    }
}

// ============================================================================
// DTOs — the three already registered in main.rs (CompanyAccessDto,
// MyCompaniesResponse, SwitchCompanyRequest) keep their names/shapes; the rest
// are NEW and registered by Phase 2 (contract-note filed).
// ============================================================================

/// One company the current principal can operate (UC-ACC-01.2/.3).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CompanyAccessDto {
    pub organization_id: Uuid,
    pub name: String,
    pub role: String,
}

/// Response listing the principal's companies.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MyCompaniesResponse {
    pub companies: Vec<CompanyAccessDto>,
    /// The currently active company resolved from the JWT/session, if any.
    pub active_company_id: Option<Uuid>,
}

/// Request to switch the active company (UC-ACC-01.3).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SwitchCompanyRequest {
    pub organization_id: Uuid,
}

/// Request to create a new company/agenda (UC-ACC-01.1/.2). NEW DTO.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateCompanyRequest {
    /// Display name of the company (agenda).
    pub name: String,
    /// Optional URL slug; auto-derived from `name` when omitted.
    pub slug: Option<String>,
    /// Billing/contact email for the company.
    pub contact_email: String,
}

/// Request to invite a user into the active company with a role
/// (UC-ACC-01.4/.5). NEW DTO.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InviteUserRequest {
    /// Email of the user to invite. Must already correspond to an account.
    pub email: String,
    /// One of: "admin" | "standard" | "read-only" (UC-ACC-01.5).
    pub role: String,
}

/// Request to change a member's role (UC-ACC-01.5). NEW DTO.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SetMemberRoleRequest {
    pub user_id: Uuid,
    /// One of: "admin" | "standard" | "read-only".
    pub role: String,
}

/// Request to grant an accountant cross-company access (UC-ACC-01.7). NEW DTO.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GrantAccountantRequest {
    /// Email of the accountant's account.
    pub email: String,
    /// Optional role override; defaults to "read-only" (read+export).
    pub role: Option<String>,
}

/// Response after a membership mutation (invite / role / grant). NEW DTO.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MembershipMutationResponse {
    pub membership_id: Uuid,
    pub organization_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
}

/// Billing/plan + usage-vs-limits view (UC-ACC-01.8). MVP stub backed by the
/// default-plan constants + live usage counts. NEW DTO.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BillingOverviewResponse {
    /// Plan identifier (MVP: always the default plan).
    pub plan: String,
    /// Companies the principal owns vs the plan's company limit.
    pub companies_used: i64,
    pub companies_limit: i64,
    /// Active+pending members in the active company vs the per-company limit.
    pub members_used: i64,
    pub members_limit: i64,
}

// ============================================================================
// Wired handlers (in Foundation's routes/mod.rs).
// ============================================================================

/// List the companies (agendas) the authenticated user may operate
/// (UC-ACC-01.2/.3/.7). Spans every company the principal actively belongs to;
/// a revoked accountant / deactivated user drops off immediately (the repo
/// query requires an active membership). `active_company_id` echoes the tenant
/// the current JWT is scoped to (the principal's "last active company").
#[utoipa::path(
    get,
    path = "/api/v1/accounts/companies",
    responses(
        (status = 200, description = "Companies the principal can operate", body = MyCompaniesResponse),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Accounts"
)]
pub async fn list_my_companies(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<MyCompaniesResponse>, StatusCode> {
    // UC-ACC-01.2/.3/.7 — cross-org read keyed by the authenticated user id.
    // `organization_members` FORCEs RLS and its SELECT policy keys on
    // `app.current_user_id`, so we open a connection with the USER context set
    // (org-less: a fresh account may have no active company yet) and read the
    // caller's OWN memberships across all companies.
    let mut conn = acquire_user_ctx(&state, auth.user_id).await?;

    let companies = state
        .accounts_repo
        .list_companies_for_user(&mut *conn, auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!("list_companies_for_user failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        });
    release_user_ctx(conn).await;
    let companies = companies?;

    let companies = companies
        .into_iter()
        .map(|c| CompanyAccessDto {
            organization_id: c.organization_id,
            name: c.name,
            role: c.role,
        })
        .collect();

    Ok(Json(MyCompaniesResponse {
        companies,
        // The "active company" is whatever the current JWT/session is scoped to
        // (UC-ACC-01.13 last-company resume). `None` when the token carries no
        // tenant (e.g. a fresh account before first company selection).
        active_company_id: auth.tenant_id,
    }))
}

/// Confirm the principal may switch to a company (UC-ACC-01.3). The actual
/// active-company selection is reflected in the next JWT/session issued by
/// api-server; this validates access and returns 204 on success, 403 otherwise.
/// This is the cross-company isolation gate (NFR-ACC-01): a switch to a company
/// the principal does not actively belong to is denied.
#[utoipa::path(
    post,
    path = "/api/v1/accounts/switch-company",
    request_body = SwitchCompanyRequest,
    responses(
        (status = 204, description = "Access confirmed"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "No access to that company")
    ),
    tag = "Accounts"
)]
pub async fn switch_company(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<SwitchCompanyRequest>,
) -> Result<StatusCode, StatusCode> {
    // UC-ACC-01.3 — verify ACTIVE membership before confirming the switch.
    // User-context connection: the membership SELECT policy keys on
    // `app.current_user_id`; the principal's active company may differ from the
    // requested one (that's the whole point of a switch), so we set USER (not
    // org) context.
    let mut conn = acquire_user_ctx(&state, auth.user_id).await?;
    let allowed = state
        .accounts_repo
        .user_has_company_access(&mut *conn, auth.user_id, req.organization_id)
        .await
        .map_err(|e| {
            tracing::error!("user_has_company_access failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        });
    release_user_ctx(conn).await;
    let allowed = allowed?;

    if allowed {
        Ok(StatusCode::NO_CONTENT)
    } else {
        // No bleed: an unrelated company is indistinguishable from a forbidden
        // one — both 403, non-enumerating.
        Err(StatusCode::FORBIDDEN)
    }
}

/// Get the profile of a company by id within the principal's scope
/// (UC-ACC-01.3). RLS-scoped: the connection is bound to the active company, so
/// a cross-company id returns 404 (the row is invisible) rather than leaking its
/// existence (UC-ACC-01.2 isolation).
#[utoipa::path(
    get,
    path = "/api/v1/accounts/companies/{id}",
    params(("id" = Uuid, Path, description = "Organization (company) id")),
    responses(
        (status = 200, description = "Company profile", body = CompanyAccessDto),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found")
    ),
    tag = "Accounts"
)]
pub async fn get_company(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> Result<Json<CompanyAccessDto>, StatusCode> {
    // UC-ACC-01.3 — reading accounting-company data requires at least the
    // accounting read role, enforced server-side (UC-ACC-16.2).
    authz::require_role(rls.role(), authz::READ_MIN_ROLE).map_err(|_| StatusCode::FORBIDDEN)?;

    let user_id = rls.user_id();

    // UC-ACC-01.2 isolation — `organizations` has NO RLS (it IS the tenant), so
    // a bare find_company_rls would leak ANY company's existence by id. Gate on
    // the principal's ACTIVE membership in `id` first (membership SELECT policy
    // keys on the user context the RlsConnection carries). A company the
    // principal does not belong to is reported as 404 (no enumeration).
    let role = state
        .accounts_repo
        .role_in_company(&mut **rls, user_id, id)
        .await
        .map_err(|e| {
            tracing::error!("role_in_company failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    let Some(role) = role else {
        rls.release().await;
        return Err(StatusCode::NOT_FOUND);
    };

    let org = state
        .accounts_repo
        .find_company_rls(&mut **rls, id)
        .await
        .map_err(|e| {
            tracing::error!("find_company_rls failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    rls.release().await;

    let org = org.ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(CompanyAccessDto {
        organization_id: org.id,
        name: org.name,
        role,
    }))
}

// ============================================================================
// Additional handlers (implemented + ready; Phase 2 wires their routes).
// ============================================================================

/// Create a new company/agenda and make the caller its Owner/Admin
/// (UC-ACC-01.1/.2). Enforces the plan company-count limit (UC-ACC-01.8) before
/// creation, derives/validates the slug, then provisions org + owner membership
/// in one transaction and writes an audit entry. Returns the new company.
#[utoipa::path(
    post,
    path = "/api/v1/accounts/companies",
    request_body = CreateCompanyRequest,
    responses(
        (status = 201, description = "Company created", body = CompanyAccessDto),
        (status = 400, description = "Invalid input"),
        (status = 401, description = "Unauthorized"),
        (status = 402, description = "Plan company limit reached — upgrade required"),
        (status = 409, description = "Slug already taken")
    ),
    tag = "Accounts"
)]
pub async fn create_company(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateCompanyRequest>,
) -> Result<(StatusCode, Json<CompanyAccessDto>), StatusCode> {
    // UC-ACC-01.1 — basic validation (non-enumerating, clear).
    let name = req.name.trim();
    let contact_email = req.contact_email.trim();
    if name.is_empty() || name.len() > 255 || contact_email.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Slug: validate the caller's, else derive from name. (Cheap, no DB.)
    let slug = match req.slug.as_deref().map(str::trim) {
        Some(s) if !s.is_empty() => {
            if !is_valid_slug(s) {
                return Err(StatusCode::BAD_REQUEST);
            }
            s.to_string()
        }
        _ => {
            let derived = slugify(name);
            if derived.is_empty() {
                return Err(StatusCode::BAD_REQUEST);
            }
            derived
        }
    };

    // One connection carries the USER context for the limit check, then gets
    // the new org's context set inside create_company_with_owner so both the
    // membership INSERT and the audit INSERT pass their RLS WITH CHECK.
    let mut conn = acquire_user_ctx(&state, auth.user_id).await?;

    // UC-ACC-01.2/.8 — plan company-count gate BEFORE creation.
    let owned = match state
        .accounts_repo
        .count_owned_companies(&mut *conn, auth.user_id)
        .await
    {
        Ok(n) => n,
        Err(e) => {
            tracing::error!("count_owned_companies failed: {e}");
            release_user_ctx(conn).await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    if owned >= DEFAULT_PLAN_MAX_COMPANIES {
        release_user_ctx(conn).await;
        // 402 Payment Required → the upgrade-prompt signal.
        return Err(StatusCode::PAYMENT_REQUIRED);
    }

    // UC-ACC-01.1/.2 — provision org + owner membership atomically (the helper
    // repoints the org RLS context to the new company between the two INSERTs).
    let provisioned = state
        .accounts_repo
        .create_company_with_owner(&mut conn, auth.user_id, name, &slug, contact_email)
        .await;
    let provisioned = match provisioned {
        Ok(p) => p,
        Err(e) => {
            release_user_ctx(conn).await;
            // A slug collision surfaces as a unique-violation; map to 409.
            if let sqlx::Error::Database(db_err) = &e {
                if db_err.is_unique_violation() {
                    return Err(StatusCode::CONFLICT);
                }
            }
            tracing::error!("create_company_with_owner failed: {e}");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let org = provisioned.organization;

    // UC-ACC-01.10 / 16.7 — audit the creation on the SAME connection (its org
    // context now points at the new company, so the acc_audit_log INSERT WITH
    // CHECK `tenant_id = get_current_org_id()` passes). Best-effort: a failed
    // audit must not lose the just-created company, so we log + warn.
    if let Ok(entry) = audit::record(
        org.id,
        Some(auth.user_id),
        "create",
        "company",
        Some(org.id),
        serde_json::json!({ "name": org.name, "slug": org.slug }),
    ) {
        if let Err(e) = state
            .platform_repo
            .record_audit_rls(&mut *conn, entry)
            .await
        {
            tracing::warn!("audit record for company create failed (non-fatal): {e}");
        }
    }

    release_user_ctx(conn).await;

    Ok((
        StatusCode::CREATED,
        Json(CompanyAccessDto {
            organization_id: org.id,
            name: org.name,
            role: "org_admin".to_string(),
        }),
    ))
}

/// Invite a user (by email) into the ACTIVE company with a role
/// (UC-ACC-01.4/.5). Admin-only (manage members). Enforces the per-company seat
/// limit (UC-ACC-01.8), resolves the email to an account, upserts a pending
/// membership, and audits. Re-inviting a previously deactivated user
/// re-activates the invite while preserving their authored documents
/// (UC-ACC-01.6).
#[utoipa::path(
    post,
    path = "/api/v1/accounts/members/invite",
    request_body = InviteUserRequest,
    responses(
        (status = 200, description = "Invitation created", body = MembershipMutationResponse),
        (status = 400, description = "Invalid role"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden (admin required)"),
        (status = 402, description = "Seat limit reached — upgrade required"),
        (status = 404, description = "No account for that email")
    ),
    tag = "Accounts"
)]
pub async fn invite_user(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(req): Json<InviteUserRequest>,
) -> Result<Json<MembershipMutationResponse>, StatusCode> {
    // UC-ACC-01.4/.5 — managing members is an admin action (server-enforced).
    if !authz::is_admin(rls.role()) {
        rls.release().await;
        return Err(StatusCode::FORBIDDEN);
    }

    let role = match normalize_role(&req.role) {
        Some(r) => r,
        None => {
            rls.release().await;
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    // The RlsConnection is scoped to the ACTIVE company (org = tenant_id);
    // that's exactly the company we invite into, so the membership UPDATE/INSERT
    // and the audit INSERT both satisfy `organization_id/tenant_id =
    // get_current_org_id()`. Keep it open through the whole mutation.
    let organization_id = rls.tenant_id();
    let actor_id = rls.user_id();

    // Resolve the invitee account by email FIRST (users is RLS-exempt; the repo
    // pool read is fine). Invitations target existing accounts in the MVP; a
    // "create-on-invite" flow is future work.
    let user_repo = UserRepository::new(state.db.clone());
    let invitee = match user_repo.find_by_email(req.email.trim()).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            rls.release().await;
            return Err(StatusCode::NOT_FOUND);
        }
        Err(e) => {
            tracing::error!("find_by_email failed: {e}");
            rls.release().await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // UC-ACC-01.3/.8 — seat-limit gate before creating the invite.
    let members = match state
        .accounts_repo
        .count_active_members(&mut **rls, organization_id)
        .await
    {
        Ok(n) => n,
        Err(e) => {
            tracing::error!("count_active_members failed: {e}");
            rls.release().await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    if members >= DEFAULT_PLAN_MAX_MEMBERS_PER_COMPANY {
        rls.release().await;
        return Err(StatusCode::PAYMENT_REQUIRED);
    }

    let membership_id = match state
        .accounts_repo
        .upsert_membership(&mut **rls, organization_id, invitee.id, role, actor_id)
        .await
    {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("upsert_membership failed: {e}");
            rls.release().await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // UC-ACC-01.10 — audit on the SAME RLS connection (same org context).
    audit_membership(
        &state,
        &mut **rls,
        organization_id,
        actor_id,
        "invite",
        invitee.id,
        role,
    )
    .await;

    rls.release().await;
    Ok(Json(MembershipMutationResponse {
        membership_id,
        organization_id,
        user_id: invitee.id,
        role: role.to_string(),
    }))
}

/// Change a member's role within the active company (UC-ACC-01.5). Admin-only.
#[utoipa::path(
    post,
    path = "/api/v1/accounts/members/role",
    request_body = SetMemberRoleRequest,
    responses(
        (status = 200, description = "Role updated", body = MembershipMutationResponse),
        (status = 400, description = "Invalid role"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden (admin required)"),
        (status = 404, description = "Member not found")
    ),
    tag = "Accounts"
)]
pub async fn set_member_role(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(req): Json<SetMemberRoleRequest>,
) -> Result<Json<MembershipMutationResponse>, StatusCode> {
    if !authz::is_admin(rls.role()) {
        rls.release().await;
        return Err(StatusCode::FORBIDDEN);
    }
    let role = match normalize_role(&req.role) {
        Some(r) => r,
        None => {
            rls.release().await;
            return Err(StatusCode::BAD_REQUEST);
        }
    };
    let organization_id = rls.tenant_id();
    let actor_id = rls.user_id();

    let membership_id = match state
        .accounts_repo
        .set_member_role(&mut **rls, organization_id, req.user_id, role)
        .await
    {
        Ok(Some(id)) => id,
        Ok(None) => {
            rls.release().await;
            return Err(StatusCode::NOT_FOUND);
        }
        Err(e) => {
            tracing::error!("set_member_role failed: {e}");
            rls.release().await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    audit_membership(
        &state,
        &mut **rls,
        organization_id,
        actor_id,
        "update_role",
        req.user_id,
        role,
    )
    .await;

    rls.release().await;
    Ok(Json(MembershipMutationResponse {
        membership_id,
        organization_id,
        user_id: req.user_id,
        role: role.to_string(),
    }))
}

/// Deactivate a member of the active company (UC-ACC-01.6). Admin-only. Revokes
/// access immediately (membership flips to 'removed', failing every subsequent
/// request's membership check) while their authored documents remain intact
/// (those reference the user id, not the membership row).
#[utoipa::path(
    delete,
    path = "/api/v1/accounts/members/{user_id}",
    params(("user_id" = Uuid, Path, description = "User to deactivate")),
    responses(
        (status = 204, description = "Member deactivated"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden (admin required)"),
        (status = 404, description = "Member not found")
    ),
    tag = "Accounts"
)]
pub async fn deactivate_member(
    Path(user_id): Path<Uuid>,
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> Result<StatusCode, StatusCode> {
    if !authz::is_admin(rls.role()) {
        rls.release().await;
        return Err(StatusCode::FORBIDDEN);
    }
    let organization_id = rls.tenant_id();
    let actor_id = rls.user_id();

    // UC-ACC-01.6 — an admin must not lock themselves out / orphan the company;
    // disallow self-deactivation here (the last-admin invariant is enforced by
    // the higher-level account-deletion flow, EPIC-ACC-16).
    if user_id == actor_id {
        rls.release().await;
        return Err(StatusCode::BAD_REQUEST);
    }

    let removed = match state
        .accounts_repo
        .deactivate_member(&mut **rls, organization_id, user_id)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("deactivate_member failed: {e}");
            rls.release().await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    if !removed {
        rls.release().await;
        return Err(StatusCode::NOT_FOUND);
    }

    audit_membership(
        &state,
        &mut **rls,
        organization_id,
        actor_id,
        "deactivate",
        user_id,
        "removed",
    )
    .await;
    rls.release().await;
    Ok(StatusCode::NO_CONTENT)
}

/// Grant an accountant ongoing access to the active company (UC-ACC-01.7).
/// Admin-only. The accountant is a cross-company principal; this creates a
/// per-company membership edge with a read+export role (default read-only). The
/// accountant can then switch into this company alongside their other clients.
#[utoipa::path(
    post,
    path = "/api/v1/accounts/accountant-access",
    request_body = GrantAccountantRequest,
    responses(
        (status = 200, description = "Access granted", body = MembershipMutationResponse),
        (status = 400, description = "Invalid role"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden (admin required)"),
        (status = 404, description = "No account for that email")
    ),
    tag = "Accounts"
)]
pub async fn grant_accountant_access(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(req): Json<GrantAccountantRequest>,
) -> Result<Json<MembershipMutationResponse>, StatusCode> {
    if !authz::is_admin(rls.role()) {
        rls.release().await;
        return Err(StatusCode::FORBIDDEN);
    }
    // UC-ACC-01.7 — default accountant role is read-only (read+export).
    let role = match normalize_role(req.role.as_deref().unwrap_or("read-only")) {
        Some(r) => r,
        None => {
            rls.release().await;
            return Err(StatusCode::BAD_REQUEST);
        }
    };
    let organization_id = rls.tenant_id();
    let actor_id = rls.user_id();

    let user_repo = UserRepository::new(state.db.clone());
    let accountant = match user_repo.find_by_email(req.email.trim()).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            rls.release().await;
            return Err(StatusCode::NOT_FOUND);
        }
        Err(e) => {
            tracing::error!("find_by_email failed: {e}");
            rls.release().await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let membership_id = match state
        .accounts_repo
        .grant_accountant_access(&mut **rls, organization_id, accountant.id, role, actor_id)
        .await
    {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("grant_accountant_access failed: {e}");
            rls.release().await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    audit_membership(
        &state,
        &mut **rls,
        organization_id,
        actor_id,
        "grant_accountant",
        accountant.id,
        role,
    )
    .await;

    rls.release().await;
    Ok(Json(MembershipMutationResponse {
        membership_id,
        organization_id,
        user_id: accountant.id,
        role: role.to_string(),
    }))
}

/// Revoke an accountant's access to the ACTIVE company only (UC-ACC-01.7).
/// Admin-only. Immediate and isolated: the accountant keeps access to every
/// other company that still grants it.
#[utoipa::path(
    delete,
    path = "/api/v1/accounts/accountant-access/{user_id}",
    params(("user_id" = Uuid, Path, description = "Accountant user id")),
    responses(
        (status = 204, description = "Access revoked"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden (admin required)"),
        (status = 404, description = "No such grant")
    ),
    tag = "Accounts"
)]
pub async fn revoke_accountant_access(
    Path(user_id): Path<Uuid>,
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> Result<StatusCode, StatusCode> {
    if !authz::is_admin(rls.role()) {
        rls.release().await;
        return Err(StatusCode::FORBIDDEN);
    }
    let organization_id = rls.tenant_id();
    let actor_id = rls.user_id();

    let revoked = match state
        .accounts_repo
        .revoke_accountant_access(&mut **rls, organization_id, user_id)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("revoke_accountant_access failed: {e}");
            rls.release().await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    if !revoked {
        rls.release().await;
        return Err(StatusCode::NOT_FOUND);
    }

    audit_membership(
        &state,
        &mut **rls,
        organization_id,
        actor_id,
        "revoke_accountant",
        user_id,
        "removed",
    )
    .await;
    rls.release().await;
    Ok(StatusCode::NO_CONTENT)
}

/// Billing/plan overview with usage-vs-limits (UC-ACC-01.8). MVP stub: reports
/// the default plan + live usage (companies owned, members in active company)
/// against the gate constants the create/invite handlers enforce, so the
/// displayed limits match the enforced limits.
#[utoipa::path(
    get,
    path = "/api/v1/accounts/billing",
    responses(
        (status = 200, description = "Plan + usage", body = BillingOverviewResponse),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Accounts"
)]
pub async fn get_billing(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<BillingOverviewResponse>, StatusCode> {
    // Owned-companies count keys on the USER context (the caller's own
    // org_admin memberships). Set both org (active company, for the member
    // count below) AND user context so the single connection serves both reads;
    // the member count needs org context because the membership SELECT policy
    // would otherwise hide the other members' rows.
    let mut conn = acquire_ctx(&state, auth.tenant_id, auth.user_id).await?;

    let companies_used = state
        .accounts_repo
        .count_owned_companies(&mut *conn, auth.user_id)
        .await;
    let companies_used = match companies_used {
        Ok(n) => n,
        Err(e) => {
            tracing::error!("count_owned_companies failed: {e}");
            release_user_ctx(conn).await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Members count is scoped to the active company (the tenant in the JWT).
    let members_used = match auth.tenant_id {
        Some(org) => match state
            .accounts_repo
            .count_active_members(&mut *conn, org)
            .await
        {
            Ok(n) => n,
            Err(e) => {
                tracing::error!("count_active_members failed: {e}");
                release_user_ctx(conn).await;
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        },
        None => 0,
    };

    release_user_ctx(conn).await;

    Ok(Json(BillingOverviewResponse {
        plan: "default".to_string(),
        companies_used,
        companies_limit: DEFAULT_PLAN_MAX_COMPANIES,
        members_used,
        members_limit: DEFAULT_PLAN_MAX_MEMBERS_PER_COMPANY,
    }))
}

// ============================================================================
// Helpers (private to this owned file).
// ============================================================================

/// Acquire a pooled connection with the USER RLS context set (org UNSET) for
/// the `AuthUser`-only handlers. Needed because `organization_members` FORCEs
/// RLS and its SELECT policy keys on `app.current_user_id`; without the context
/// a bare connection silently returns 0 rows. Org-less by design: a fresh
/// account may not have selected a company yet (UC-ACC-01.1/.2/.3). Mirrors
/// what `RlsConnection` does internally, but for an org-less read.
async fn acquire_user_ctx(
    state: &AppState,
    user_id: Uuid,
) -> Result<sqlx::pool::PoolConnection<sqlx::Postgres>, StatusCode> {
    acquire_ctx(state, None, user_id).await
}

/// Acquire a pooled connection with org (optional) + user RLS context set.
async fn acquire_ctx(
    state: &AppState,
    org_id: Option<Uuid>,
    user_id: Uuid,
) -> Result<sqlx::pool::PoolConnection<sqlx::Postgres>, StatusCode> {
    let mut conn = state.db.acquire().await.map_err(|e| {
        tracing::error!("failed to acquire db connection: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    // resource server: never super-admin via this path (false).
    db::set_request_context(&mut *conn, org_id, Some(user_id), false)
        .await
        .map_err(|e| {
            tracing::error!("failed to set RLS context: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(conn)
}

/// Clear the RLS context and return a manually-acquired connection to the pool
/// (the counterpart of [`acquire_ctx`]). Prevents context bleed between
/// requests (the same discipline `RlsConnection::release` enforces). Failures
/// to clear are logged, not propagated.
async fn release_user_ctx(mut conn: sqlx::pool::PoolConnection<sqlx::Postgres>) {
    if let Err(e) = db::clear_request_context(&mut *conn).await {
        tracing::warn!("failed to clear RLS context on release: {e}");
    }
    // conn drops here, returning to the pool with context cleared.
}

/// Best-effort audit of a membership mutation (UC-ACC-01.10 / 16.7) on the
/// SAME connection (whose org context = the company), so the append-only
/// `acc_audit_log` INSERT satisfies `tenant_id = get_current_org_id()`.
/// Failures are logged, never propagated — the mutation already committed and
/// an audit-write hiccup must not 500 the caller.
async fn audit_membership<'e, E>(
    state: &AppState,
    executor: E,
    organization_id: Uuid,
    actor_id: Uuid,
    action: &str,
    target_user: Uuid,
    role: &str,
) where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    match audit::record(
        organization_id,
        Some(actor_id),
        action,
        "membership",
        Some(target_user),
        serde_json::json!({ "user_id": target_user, "role": role }),
    ) {
        Ok(entry) => {
            if let Err(e) = state.platform_repo.record_audit_rls(executor, entry).await {
                tracing::warn!("audit record ({action}) failed (non-fatal): {e}");
            }
        }
        Err(e) => tracing::warn!("audit build ({action}) failed (non-fatal): {e}"),
    }
}

/// Derive a URL-friendly slug from a company name (lowercase ASCII alnum,
/// runs of other chars collapsed to a single hyphen, trimmed). Inlined because
/// api-server's `generate_slug` is not importable from this crate.
fn slugify(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Validate a caller-supplied slug (lowercase alnum + single hyphens, no
/// leading/trailing/consecutive hyphens). Mirrors api-server's `validate_slug`.
fn is_valid_slug(slug: &str) -> bool {
    if slug.len() < 2 || slug.len() > 100 {
        return false;
    }
    if !slug
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return false;
    }
    if slug.starts_with('-') || slug.ends_with('-') || slug.contains("--") {
        return false;
    }
    true
}

// ============================================================================
// Unit tests (BODIES ONLY — Verify runs them in Phase 2; coders never build).
// Pure helpers are tested here; handler/repo behaviour is covered by Phase 2
// integration tests against a real DB (RLS isolation is the release gate).
// ============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_role_maps_story_roles() {
        // UC-ACC-01.5 — the three user-facing roles map to canonical role_types.
        assert_eq!(normalize_role("admin"), Some("org_admin"));
        assert_eq!(normalize_role("Admin"), Some("org_admin"));
        assert_eq!(normalize_role("standard"), Some("manager"));
        assert_eq!(normalize_role("read-only"), Some("read_only"));
        assert_eq!(normalize_role("read_only"), Some("read_only"));
        assert_eq!(normalize_role(" readonly "), Some("read_only"));
    }

    #[test]
    fn normalize_role_rejects_unknown() {
        // A caller cannot inject an arbitrary role_type.
        assert_eq!(normalize_role("superuser"), None);
        assert_eq!(normalize_role(""), None);
        assert_eq!(normalize_role("org_admin "), Some("org_admin"));
    }

    #[test]
    fn slugify_derives_clean_slugs() {
        assert_eq!(slugify("Acme s.r.o."), "acme-s-r-o");
        assert_eq!(slugify("  Hello  World  "), "hello-world");
        assert_eq!(slugify("Žabka 123"), "abka-123"); // non-ascii dropped
        assert_eq!(slugify("---"), "");
    }

    #[test]
    fn slug_validation_matches_rules() {
        assert!(is_valid_slug("acme-123"));
        assert!(!is_valid_slug("a")); // too short
        assert!(!is_valid_slug("-acme")); // leading hyphen
        assert!(!is_valid_slug("acme-")); // trailing hyphen
        assert!(!is_valid_slug("ac--me")); // consecutive hyphens
        assert!(!is_valid_slug("Acme")); // uppercase
        assert!(!is_valid_slug("acme_co")); // underscore not allowed
    }

    #[test]
    fn plan_limits_are_sane() {
        // UC-ACC-01.8 — the displayed limits the billing handler reports are the
        // same constants the create/invite gates enforce.
        const { assert!(DEFAULT_PLAN_MAX_COMPANIES >= 1) };
        const { assert!(DEFAULT_PLAN_MAX_MEMBERS_PER_COMPANY >= 1) };
    }
}
