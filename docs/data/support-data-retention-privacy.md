# Support Data — Retention & Privacy Posture

> Scope: the read-only GET endpoints behind the Support Data admin page (issue #635).
> Audience: platform support engineers, data-protection / GDPR reviewers.
> Grounded in code as of branch `auto-impl/pm-data-support-data-retention-privacy`.

## Overview

The Support Data feature gives platform-level support engineers a read-only
window into platform health and into an individual user's account state, plus a
single mutating action (session revocation). It is implemented in:

- Routes: `backend/servers/api-server/src/routes/platform_admin.rs`
- Repository / SQL: `backend/crates/db/src/repositories/platform_admin.rs`

All endpoints are cross-tenant (they bypass Postgres RLS) and are gated behind
the platform SuperAdmin role plus an admin capability.

## What is read

### `GET /api/v1/platform-admin/support-data`

Aggregated, platform-wide diagnostics (no per-user PII). Handler
`get_support_data`; repository `get_support_data` runs five counting queries
inside a single `REPEATABLE READ` transaction (consistent snapshot, see #628):

| Field | Source table / query |
|-------|----------------------|
| `total_orgs` | `COUNT(*) FROM organizations` |
| `total_users` / `active_users` / `pending_users` / `suspended_users` | `COUNT(*) FROM users GROUP BY status` |
| `active_sessions` | `COUNT(*) FROM refresh_tokens WHERE revoked_at IS NULL AND expires_at > NOW()` |
| `total_faults` | `COUNT(*) FROM faults` |
| `fault_by_status` | `COUNT(*) FROM faults GROUP BY status` |

This response is **counts only** — no names, emails, or row-level data.

### `GET /api/v1/platform-admin/support/users/{id}` (and `/memberships`, `/sessions`, `/activity`)

These return row-level data for one user and **do contain PII**:

- **User detail** (`get_user_for_support`) — `SupportUserInfo`: `id`, `email`,
  `display_name`, `first_name`, `last_name`, `status`, `email_verified`,
  `created_at`, `updated_at`, `last_login_at` (from `users`).
- **Memberships** (`get_user_memberships`) — `organization_id`,
  `organization_name`, `role_name`, `joined_at` (from `organization_members`).
- **Sessions** (`get_user_sessions`) — from `refresh_tokens`: `id`,
  `created_at`, `expires_at`, `last_used_at`, **`user_agent`**, **`ip_address`**.
  Filtered to `revoked_at IS NULL AND expires_at > NOW()` (active sessions only).
- **Activity log** (`get_user_activity_log`) — from `audit_logs`: `id`,
  `action`, `resource_type`, `resource_id`, `details` (JSONB), `created_at`.
  Limited via the `limit` query param (default 50, clamped 1–500).

### Mutating action (for completeness)

`POST /api/v1/platform-admin/support/users/{id}/sessions/revoke`
(`revoke_user_sessions`) sets `revoked_at = NOW()` on all of the user's active
refresh tokens. Not a read, but it operates on the same session data.

## Access control

| Endpoint | Capability (`require_capability`) | Additional gate |
|----------|-----------------------------------|-----------------|
| `support-data` | `AuditRead` | SuperAdmin role enforced in-handler |
| `support/users/{id}/sessions` | `UsersRead` | SuperAdmin role enforced in-handler |
| `support/users/{id}/sessions/revoke` | `UsersWrite` | SuperAdmin role enforced in-handler |
| `support/users/{id}/activity` | `AuditRead` | SuperAdmin role enforced in-handler |

Every handler calls `extract_super_admin_token` (in `platform_admin.rs`), which
validates the bearer access token and rejects the request with `403` unless the
token carries the SuperAdmin role (`has_super_admin_role`). Capabilities are
defined in `backend/crates/admin-core/src/capability.rs` (`AuditRead`,
`UsersRead`, `UsersWrite`). So access requires **both** the SuperAdmin role and
the listed capability — a defence-in-depth pairing.

`audit_logs` itself is additionally RLS-protected (migration
`00025_create_audit_logs.sql`): the `audit_logs_super_admin` policy restricts
direct table access to super admins, and users may read only their own rows.

## Access is itself audited

Each Support Data read/action emits an append-only analytics event via
`log_support_tooling_event` into `support_tooling_events` (migration
`00163_create_support_tooling_events.sql`):

- `support_data_viewed` — props: `tenant_count`, `fault_total`.
- `support_user_searched` — props: `query_length`, `status_filter`,
  `result_count`. The **raw search string is deliberately NOT stored** (it
  commonly contains emails / PII) — only its character length.
- `support_sessions_revoked` — props: `target_user_id`, `revoked_count`.

These events are fire-and-forget (a tracking failure never fails the user-facing
response) and the table is **immutable**: DB triggers reject `UPDATE`/`DELETE`,
and `admin_user_id` ties each event to the acting admin. This gives a tamper-
resistant record of *who looked at what, when*.

## Retention posture

| Data set | Table | Retention behaviour |
|----------|-------|---------------------|
| Sessions | `refresh_tokens` | Token lifetime is **7 days** (`refresh_token_lifetime`, `services/jwt.rs`); access tokens are 15 min. A scheduled cleanup deletes rows where `expires_at < NOW() OR revoked_at < NOW() - INTERVAL '7 days'` — `SessionRepository::cleanup_expired_tokens`, invoked by the background scheduler `cleanup_sessions` (default tick every 60s, `services/scheduler.rs`). So expired/revoked sessions are purged within ~7 days. |
| Activity log | `audit_logs` | **Append-only, no automated retention/expiry found.** Rows persist indefinitely; `user_id` is `ON DELETE SET NULL` so the entry survives user deletion (anonymised). Used for compliance (Epic 9 / Story 9.6). |
| Support-tooling events | `support_tooling_events` | **Append-only, immutable; 730-day (24-month) retention (migration `00222`).** Aged events are pruned by the DB-native `cleanup_old_support_tooling_events(retention_days DEFAULT 730)` function via a sanctioned, GUC-gated delete path — every other UPDATE/DELETE stays rejected. `admin_user_id` is `ON DELETE RESTRICT` (migration `00165`) so the trail cannot be lost via admin-account deletion; RLS restricts the table to super-admins. See [Retention policy](#retention-policy-support_tooling_events) below. |
| Aggregate counts | n/a (computed) | Not stored; recomputed per request. |

## Retention policy (`support_tooling_events`)

Published to satisfy GDPR storage limitation (Art. 5(1)(e)) for the admin
support-tooling audit trail. Defined in migration
`00222_support_tooling_events_retention.sql`.

- **Retention period:** **730 days (24 months).** Long enough to support
  security / accountability investigations into who accessed support data;
  bounded so events are not retained indefinitely. The period is a function
  parameter (`retention_days`, default 730) so operations can override it
  per-invocation without a schema migration — mirroring `cleanup_old_traces`
  (migration `00079`) and `cleanup_old_health_check_results` (migration `00074`).
- **Legal basis for retention:** legitimate interest in a tamper-evident record
  of privileged support access (security & accountability); the 24-month bound
  is the storage-limitation control.
- **Mechanism:** DB-native `cleanup_old_support_tooling_events(retention_days)`
  deletes rows whose `occurred_at` is older than the window and returns the
  deleted count. It is exposed to Rust via
  `PlatformAdminRepository::cleanup_old_support_tooling_events`, following the
  same repository-method + `SELECT cleanup_old_*()` pattern the tracing and
  health-monitoring retention jobs already use.
- **Immutability preserved:** the table's append-only triggers (migration
  `00163`) reject all UPDATE, and reject DELETE **except** inside the sanctioned
  retention path. The retention function opens that path by setting the
  transaction-local `app.retention_prune` GUC (same `app.*` namespace as
  `app.org_id` / `app.is_super_admin`); it also asserts the super-admin RLS
  context so the FORCE-RLS policy (migration `00165`) permits the delete when the
  api-server runs it. Both GUCs are reset before the function returns, so context
  never bleeds onto the pooled connection. Any DELETE outside this path is still
  rejected — the tamper-evidence guarantee for recent events is intact.
- **Scheduling (follow-up, out of scope for pm-data):** invoking the prune on a
  cadence belongs in the api-server background scheduler
  (`services/scheduler.rs`, alongside `cleanup_sessions`) or an external
  scheduler such as pg_cron. That wiring is a separate pm-backend task; this
  change establishes the retention *definition* and its call surface.

## PII / GDPR considerations

What the per-user endpoints expose that is personal data:

- **Direct identifiers:** email, first/last name, display name (`users`).
- **Network / device data:** `ip_address` and `user_agent` on each session
  (`refresh_tokens`) — IP is personal data under GDPR.
- **Behavioural data:** the activity log (`audit_logs.action` + `details`
  JSONB) is a record of the user's actions; `details` may carry request context.

Mitigations already in place:

- Cross-tenant reads require SuperAdmin **and** an explicit capability.
- Every access is recorded in an immutable `support_tooling_events` trail.
- The free-text search term (likely email) is not persisted to analytics.
- Session listing is limited to *active* tokens, and expired/revoked tokens are
  garbage-collected within ~7 days, limiting the IP/UA exposure window.
- `audit_logs` is RLS-locked and supports user self-service reads for GDPR
  transparency.

Recommendations / gaps:

1. **`support_tooling_events` retention — ADDRESSED (migration `00222`).** A
   730-day retention policy with a DB-native prune is now defined and documented
   (see [Retention policy](#retention-policy-support_tooling_events)). **`audit_logs`
   still has no documented retention limit** — GDPR storage-limitation
   (Art. 5(1)(e)) expects a defined retention period; recommend defining and
   enforcing one (e.g. the same scheduled-prune pattern) and documenting the
   legal basis for indefinite audit retention if that is intentional.
2. **IP / user-agent in `details` JSONB** of `audit_logs` is unstructured — a
   data-subject erasure (Art. 17) would need to account for PII that may be
   embedded there, not just the `user_id` SET NULL.
3. The `support_user_searched` props omit the query string but **do** store
   `status_filter` and `result_count`; confirm these are not sensitive in
   combination.

## Open questions (verify before relying on this doc)

1. **`is_revoked` vs `revoked_at` column mismatch — RESOLVED.** The only DDL for
   `refresh_tokens` (migration `00002_create_refresh_tokens.sql`) and the model
   `crates/db/src/models/refresh_token.rs` use `revoked_at TIMESTAMPTZ` (nullable)
   and have no `updated_at` column. The Support Data queries (`get_user_sessions`,
   `revoke_user_sessions`, `get_support_data`) previously referenced a
   non-existent `is_revoked` boolean (and `updated_at`); because these are runtime
   `query`/`query_as` calls (not compile-time-checked), the bug escaped
   `cargo check` and would have raised `column "is_revoked" does not exist` at
   request time. The queries now use `revoked_at IS NULL` / `SET revoked_at = NOW()`
   to match the schema, covered by
   `crates/db/tests/support_data_session_columns_tests.rs`.
2. **`audit_logs` retention** — is indefinite retention an intentional
   compliance decision, or a missing cleanup job? (`support_tooling_events` now
   has a defined 730-day policy via migration `00222`; `audit_logs` does not.
   See recommendation 1.)
3. **`details` JSONB contents** — what PII actually lands in `audit_logs.details`
   in practice was not exhaustively traced; the audit producers should be
   reviewed before treating it as PII-free.
