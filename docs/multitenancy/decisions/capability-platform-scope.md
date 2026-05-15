# Decision: Capabilities are platform-scoped (never org-scoped)

| Status | Accepted |
| --- | --- |
| Date | 2026-05-15 |
| Phase | 2 (D2.1 follow-up to N2 wiring) |
| Owners | Backend / multitenancy |
| Affected files | `backend/crates/admin-core/src/capability.rs`, `backend/servers/api-server/src/services/auth_policy.rs`, `backend/servers/api-server/src/routes/admin/capabilities.rs` |

## Context

`capability_grants` rows (migration 00138) carry `user_id`, `capability`,
`granted_by`, `granted_at`, `expires_at`, `revoked_at`, `revoked_by`,
`mfa_required`, and `note`. They do **not** carry `organization_id`.

When wiring `AuthPolicyEnforcer` (Defense N2 / leak #13) into the capability
revoke handler, the question came up:

> Should `capability_grants` learn an `organization_id` column so the
> enforcer's `check_*_revoke(org_id, user_id)` shape applies symmetrically?

Short answer: **no**.

## Decision

Capabilities are **platform-scoped**. They never live inside an org. A
capability grant is a statement of the form

> "User U is permitted to invoke action A across the entire platform,
>  subject to the platform-level guards that wrap the action."

There is no such thing as a per-org capability grant. The org-scoping
happens at a **different** layer:

* The data layer (Postgres RLS + the resolved `tenant_context`) decides
  which rows the action can touch.
* The capability decides whether the action can be invoked at all.

### Concrete consequences

* Granting `Capability::AgenciesWrite` to user U makes U able to write to
  **any** agency the request can reach via RLS — not "agencies in org X".
  An operator who only wants U to write to one agency must scope U via a
  membership in that org plus a role gate, **not** by issuing them a
  per-org capability.
* The `RequireCapability` extractor enforces principal-kind (`Platform`),
  active grant, and recent MFA. Those are the only guards on a capability
  invocation. They run **before** any org context is resolved.
* The `AuthPolicyEnforcer` exposes `check_capability_revoke(actor_user_id)`
  (delegating to `check_platform_action(actor_user_id)`) instead of the
  org-keyed `check_*_revoke(org_id, user_id)` shape used by membership /
  principal-kind paths. The platform-action shape loads the **default**
  auth policy as a liveness check — the platform invariants are already
  enforced by the extractor.
* Future platform-scoped operations (tenant lifecycle, feature-flag
  toggles, billing surfaces) should reuse `check_platform_action` rather
  than invent fictitious `org_id` arguments for the policy.

### What we explicitly do NOT do

* Add `organization_id` to `capability_grants`. That would conflate
  "permission to invoke" with "permission to touch row R", which is RLS's
  job — and would tempt callers to skip RLS in favor of an in-process
  org-id check, which historically is the leak #13 anti-pattern.
* Move the principal-kind / MFA checks out of `RequireCapability` into the
  enforcer. The extractor is the right place for those because it runs
  **before** any handler code, with full request context (IP, user-agent,
  audit row).

## Why this is safe

* Capabilities are gated by `principal_kind == Platform` AND an active
  grant AND recent MFA. A stolen JWT cannot mint a capability because the
  principal-kind is re-derived from the trusted `users` table on every
  request (Phase 2 contract — see `RequestPrincipal`).
* Platform principals are a tiny, audited population. Every grant is
  logged, every revoke is logged, and the no-self-grant rule is enforced
  application-side (`PgCapabilityGrantsRepository::grant`).
* Org isolation for the **data** the capability operates on is enforced by
  RLS in Postgres, not by a per-org capability check. RLS cannot be
  bypassed by mis-resolving the policy under the wrong tenant — that is
  Phase 2's whole point.

## Migration / compatibility

No schema change required. No data migration required. The decision is
documented here so a future contributor reading the enforcer or the
capability handlers does not retrofit `organization_id` onto a row that
was deliberately platform-scoped.

If a future use case really does need a "this user can write to THIS org's
agencies only" semantic, the answer is **a role-bearing membership in
that org**, not a per-org capability grant.
