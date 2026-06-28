//! Repository for ACC accounts, organizations & access (EPIC-ACC-01). OWNED BY: BE-Accounts.
//!
//! RLS-aware, generic-executor pattern (see
//! [`crate::repositories::accounting::AccountingRepository`] and
//! [`crate::repositories::organization_member::OrganizationMemberRepository`]).
//!
//! TENANT MODEL: an ACC "company/agenda" IS an existing `organizations` row;
//! accountant/user access IS org membership (`organization_members`). This repo
//! reads/writes the existing org/membership tables rather than introducing a
//! parallel identity system (architecture defers that). It ISSUES NO tokens —
//! api-server remains the issuer.
//!
//! ## RLS context (CRITICAL — both `organization_members` and the org/audit
//! tables FORCE row-level security)
//! Every method takes a generic `executor` whose RLS context the CALLER has
//! already set (via `RlsConnection` or `db::set_request_context`). The relevant
//! policies (migrations 00006 / 00196):
//!   * `organization_members` SELECT → only rows where
//!     `user_id = app.current_user_id` (the caller's OWN memberships). So
//!     [`list_companies_for_user`] / [`user_has_company_access`] /
//!     [`count_owned_companies`] need the USER context set (org optional).
//!   * `organization_members` INSERT/UPDATE → require
//!     `organization_id = get_current_org_id()`. So [`upsert_membership`],
//!     [`set_member_role`], [`deactivate_member`], [`grant_accountant_access`],
//!     [`revoke_accountant_access`] need the ORG context = the TARGET company
//!     (the active company on an `RlsConnection`).
//!   * `organizations` has NO RLS (it IS the tenant) — reads/writes there work
//!     on any executor.
//!
//! A bare pool connection (no context) silently returns 0 rows / fails the
//! INSERT WITH CHECK — hence the generic-executor contract.
//!
//! BE-Accounts implements the bodies in THIS FILE only.

use crate::models::organization::Organization;
use crate::DbPool;
use sqlx::{Executor, Postgres};
use uuid::Uuid;

/// Companies (organizations) the current principal may operate, with the
/// principal's role per company. Used for the company-switcher (UC-ACC-01.2/.3).
#[derive(Debug, Clone)]
pub struct AccCompanyAccess {
    pub organization_id: Uuid,
    pub name: String,
    pub role: String,
}

/// Result of provisioning a new company: the new organization plus the owner
/// membership row id (UC-ACC-01.1/.2). Per-repo helper (not a shared model) so
/// it needs no Foundation model edit.
#[derive(Debug, Clone)]
pub struct AccProvisionedCompany {
    pub organization: Organization,
    pub membership_id: Uuid,
}

/// Repository for ACC account/company access (EPIC-ACC-01).
#[derive(Debug, Clone)]
pub struct AccAccountsRepository {
    pub pool: DbPool,
}

impl AccAccountsRepository {
    /// Create a new accounts repository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// List the companies the principal may operate (UC-ACC-01.2/.3/.7).
    /// Reads the caller's OWN active memberships — the `organization_members`
    /// SELECT policy keys on `app.current_user_id`, so the executor must carry
    /// the USER context (set via `RlsConnection` or `set_request_context`).
    /// Excludes removed memberships and non-active companies, so a revoked
    /// accountant / deactivated user drops off immediately.
    pub async fn list_companies_for_user<'e, E>(
        &self,
        executor: E,
        user_id: Uuid,
    ) -> Result<Vec<AccCompanyAccess>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // UC-ACC-01.2/.3/.7 — one row per company the principal may operate.
        // RLS already restricts `om` to the caller's own rows; the explicit
        // user_id predicate is defense in depth and lets a super-admin context
        // still scope to the requested user.
        let rows = sqlx::query_as::<_, (Uuid, String, String)>(
            r#"
            SELECT o.id, o.name, om.role_type
            FROM organization_members om
            INNER JOIN organizations o ON o.id = om.organization_id
            WHERE om.user_id = $1
              AND om.status = 'active'
              AND o.status = 'active'
            ORDER BY o.name ASC
            "#,
        )
        .bind(user_id)
        .fetch_all(executor)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(organization_id, name, role)| AccCompanyAccess {
                organization_id,
                name,
                role,
            })
            .collect())
    }

    /// Verify the principal is an ACTIVE member of the given company
    /// (UC-ACC-01.3/.7). Cross-company isolation gate (NFR-ACC-01): `false` for
    /// any company the user does not actively belong to. Requires the USER
    /// context on the executor (`organization_members` SELECT policy).
    pub async fn user_has_company_access<'e, E>(
        &self,
        executor: E,
        user_id: Uuid,
        organization_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // UC-ACC-01.3/.7 — membership must be ACTIVE; a 'removed'/'suspended'
        // row (deactivated user / revoked accountant) does NOT grant access,
        // which is what makes revocation immediate.
        let exists = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM organization_members om
            INNER JOIN organizations o ON o.id = om.organization_id
            WHERE om.user_id = $1
              AND om.organization_id = $2
              AND om.status = 'active'
              AND o.status = 'active'
            "#,
        )
        .bind(user_id)
        .bind(organization_id)
        .fetch_one(executor)
        .await?;

        Ok(exists > 0)
    }

    /// Resolve the principal's `role_type` in a company (UC-ACC-01.5). `None`
    /// if no active membership. Requires the USER context on the executor.
    pub async fn role_in_company<'e, E>(
        &self,
        executor: E,
        user_id: Uuid,
        organization_id: Uuid,
    ) -> Result<Option<String>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let role = sqlx::query_scalar::<_, String>(
            r#"
            SELECT role_type FROM organization_members
            WHERE user_id = $1 AND organization_id = $2 AND status = 'active'
            "#,
        )
        .bind(user_id)
        .bind(organization_id)
        .fetch_optional(executor)
        .await?;

        Ok(role)
    }

    /// Fetch a company (organization) by id (UC-ACC-01.3). `organizations` has
    /// NO RLS (it IS the tenant); the route layer guards cross-company reads via
    /// [`user_has_company_access`] before calling this. Excludes soft-deleted.
    pub async fn find_company_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<Organization>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, Organization>(
            "SELECT * FROM organizations WHERE id = $1 AND status != 'deleted'",
        )
        .bind(id)
        .fetch_optional(executor)
        .await
    }

    /// Count the active companies the user OWNS (`org_admin`). Backs the plan
    /// company-count gate (UC-ACC-01.2/.8). Requires the USER context.
    pub async fn count_owned_companies<'e, E>(
        &self,
        executor: E,
        user_id: Uuid,
    ) -> Result<i64, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // UC-ACC-01.8 — only owner/admin memberships consume the
        // company-creation entitlement.
        let n = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM organization_members om
            INNER JOIN organizations o ON o.id = om.organization_id
            WHERE om.user_id = $1
              AND om.role_type = 'org_admin'
              AND om.status = 'active'
              AND o.status = 'active'
            "#,
        )
        .bind(user_id)
        .fetch_one(executor)
        .await?;

        Ok(n)
    }

    /// Provision a new company and make `owner_user_id` its `org_admin` in ONE
    /// transaction (UC-ACC-01.1/.2). `conn` MUST carry the owner's USER context
    /// AND have org context set to the freshly created org for the membership
    /// INSERT (the route sets the org GUC to `org.id` between the two INSERTs —
    /// see the route handler). `organizations` has no RLS, so its INSERT needs
    /// no prior context; the `organization_members` INSERT WITH CHECK requires
    /// `organization_id = get_current_org_id()`.
    ///
    /// `slug` must be pre-validated/uniquified; a collision surfaces as a
    /// unique-violation `sqlx::Error` the route maps to 409.
    pub async fn create_company_with_owner(
        &self,
        conn: &mut sqlx::PgConnection,
        owner_user_id: Uuid,
        name: &str,
        slug: &str,
        contact_email: &str,
    ) -> Result<AccProvisionedCompany, sqlx::Error> {
        // UC-ACC-01.2 — the company IS an organizations row (the tenant
        // boundary). The AFTER INSERT trigger seeds the default role set.
        let organization = sqlx::query_as::<_, Organization>(
            r#"
            INSERT INTO organizations (name, slug, contact_email)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
        )
        .bind(name)
        .bind(slug)
        .bind(contact_email)
        .fetch_one(&mut *conn)
        .await?;

        // Point the org RLS context at the just-created company so the
        // membership INSERT passes its WITH CHECK (organization_id =
        // get_current_org_id()). Keeps the owner's user context intact.
        crate::tenant_context::set_request_context(
            &mut *conn,
            Some(organization.id),
            Some(owner_user_id),
            false,
        )
        .await?;

        // UC-ACC-01.1 — the creator becomes the Owner/Admin (role_type
        // 'org_admin' → TenantRole::OrgAdmin), ACTIVE + joined now.
        let membership_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO organization_members
                (organization_id, user_id, role_type, status, joined_at)
            VALUES ($1, $2, 'org_admin', 'active', NOW())
            RETURNING id
            "#,
        )
        .bind(organization.id)
        .bind(owner_user_id)
        .fetch_one(&mut *conn)
        .await?;

        Ok(AccProvisionedCompany {
            organization,
            membership_id,
        })
    }

    /// Count active+pending members of a company (UC-ACC-01.3/.8 seat limit).
    /// Requires the executor's org context = this company (or super-admin),
    /// since the SELECT policy hides other-user rows otherwise; called from an
    /// `RlsConnection` scoped to the company.
    pub async fn count_active_members<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
    ) -> Result<i64, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let n = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) FROM organization_members
            WHERE organization_id = $1 AND status IN ('active', 'pending')
            "#,
        )
        .bind(organization_id)
        .fetch_one(executor)
        .await?;

        Ok(n)
    }

    /// Invite a user into a company with a role; upsert their membership
    /// (UC-ACC-01.4/.5). Re-inviting a previously removed teammate re-activates
    /// the invite (status → pending) while preserving their id — and therefore
    /// their authored documents (UC-ACC-01.6). `UNIQUE (organization_id,
    /// user_id)` makes this idempotent. Requires the executor's org context =
    /// `organization_id` (INSERT/UPDATE WITH CHECK).
    pub async fn upsert_membership<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        user_id: Uuid,
        role_type: &str,
        invited_by: Uuid,
    ) -> Result<Uuid, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // UC-ACC-01.4/.5 — scoped membership with an explicit role.
        let id = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO organization_members
                (organization_id, user_id, role_type, status, invited_by, invited_at)
            VALUES ($1, $2, $3, 'pending', $4, NOW())
            ON CONFLICT (organization_id, user_id) DO UPDATE
            SET role_type = EXCLUDED.role_type,
                status = 'pending',
                invited_by = EXCLUDED.invited_by,
                invited_at = NOW(),
                joined_at = NULL,
                updated_at = NOW()
            RETURNING id
            "#,
        )
        .bind(organization_id)
        .bind(user_id)
        .bind(role_type)
        .bind(invited_by)
        .fetch_one(executor)
        .await?;

        Ok(id)
    }

    /// Change an existing member's role (UC-ACC-01.5). Affects an active or
    /// pending membership; returns the touched membership id, or `None` if the
    /// user is not a member. Requires org context = `organization_id`.
    pub async fn set_member_role<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        user_id: Uuid,
        role_type: &str,
    ) -> Result<Option<Uuid>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // UC-ACC-01.5 — server-enforced RBAC; the new role_type changes the
        // TenantRole the resource server resolves on the member's next request.
        let id = sqlx::query_scalar::<_, Uuid>(
            r#"
            UPDATE organization_members
            SET role_type = $3, updated_at = NOW()
            WHERE organization_id = $1 AND user_id = $2 AND status != 'removed'
            RETURNING id
            "#,
        )
        .bind(organization_id)
        .bind(user_id)
        .bind(role_type)
        .fetch_optional(executor)
        .await?;

        Ok(id)
    }

    /// Deactivate a member: revoke access immediately, PRESERVE authored
    /// documents (UC-ACC-01.6). Soft delete (`status = 'removed'`) — documents
    /// reference `user_id`, never the membership row, so they stay intact.
    /// Returns `true` if a member was deactivated. Requires org context =
    /// `organization_id` (UPDATE policy).
    pub async fn deactivate_member<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // UC-ACC-01.6 — access revoked the moment status flips: the resource
        // server re-validates membership (status = 'active') every request, so
        // this revokes live sessions too.
        let res = sqlx::query(
            r#"
            UPDATE organization_members
            SET status = 'removed', updated_at = NOW()
            WHERE organization_id = $1 AND user_id = $2 AND status != 'removed'
            "#,
        )
        .bind(organization_id)
        .bind(user_id)
        .execute(executor)
        .await?;

        Ok(res.rows_affected() > 0)
    }

    /// Grant an accountant ongoing access to a company (UC-ACC-01.7). The
    /// accountant is a cross-company principal; the grant is a per-company
    /// membership EDGE with a read+export role (default `read_only`), ACTIVE
    /// immediately. Idempotent on the unique pair; re-granting after a revoke
    /// re-activates the same edge. Requires org context = `organization_id`.
    pub async fn grant_accountant_access<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        accountant_user_id: Uuid,
        role_type: &str,
        granted_by: Uuid,
    ) -> Result<Uuid, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // UC-ACC-01.7 — per-company edge; the accountant then sees this company
        // in their multi-client switcher list (list_companies_for_user).
        let id = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO organization_members
                (organization_id, user_id, role_type, status, invited_by, invited_at, joined_at)
            VALUES ($1, $2, $3, 'active', $4, NOW(), NOW())
            ON CONFLICT (organization_id, user_id) DO UPDATE
            SET role_type = EXCLUDED.role_type,
                status = 'active',
                invited_by = EXCLUDED.invited_by,
                joined_at = NOW(),
                updated_at = NOW()
            RETURNING id
            "#,
        )
        .bind(organization_id)
        .bind(accountant_user_id)
        .bind(role_type)
        .bind(granted_by)
        .fetch_one(executor)
        .await?;

        Ok(id)
    }

    /// Revoke accountant access to ONE company (UC-ACC-01.7). Soft-removes the
    /// per-company edge; isolates to the single company (the accountant keeps
    /// every other company that still grants it). Returns `true` if revoked.
    /// Distinct from [`deactivate_member`] for audit-action clarity. Requires
    /// org context = `organization_id`.
    pub async fn revoke_accountant_access<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        accountant_user_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // UC-ACC-01.7 — immediate (membership must be 'active' for any access)
        // and isolated to this company only.
        let res = sqlx::query(
            r#"
            UPDATE organization_members
            SET status = 'removed', updated_at = NOW()
            WHERE organization_id = $1 AND user_id = $2 AND status != 'removed'
            "#,
        )
        .bind(organization_id)
        .bind(accountant_user_id)
        .execute(executor)
        .await?;

        Ok(res.rows_affected() > 0)
    }
}
