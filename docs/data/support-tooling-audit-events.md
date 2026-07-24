# Support-Staff Read Audit Events — Schema Definitions

> Scope: the append-only audit events that record **which platform support
> engineer performed which support-tooling action** — who viewed platform
> diagnostics, who searched/opened a user's account, who revoked a user's
> sessions.
> Audience: data / analytics engineers, platform-admin tooling owners,
> security / GDPR reviewers wiring dashboards or investigations on
> support-staff activity.
> Grounded in code as of branch
> `auto-impl/data-support-data-audit-event-def-2026-0-26b82b74` (from `dev`).

This is a **definitions document**. It specifies the canonical event names,
identity/target dimensions, and per-event property schemas for the
`support_tooling_events` audit trail so that producers (api-server) and
consumers (audit dashboards, security investigations, retention jobs) agree on
one schema. Where a required dimension is not yet on the wire today, this doc
marks it **[gap]** and points at the exact code that would carry it — so the
schema is a target contract, not a claim that every field is already emitted.

It **closes the open decision recorded 2026-05-28** on whether the
support-staff read audit trail should be folded into the `AuditRead`-gated
generic `audit_logs` surface or stand on its own — see Section 2.

## 1. Feature recap (what we are auditing)

The **Support Data** admin feature (Story 10B.5, issue #635) gives platform
SuperAdmins a cross-tenant, RLS-bypassing window into platform health and into
an individual user's account state, plus one mutating action (session
revocation). The relevant code:

| Concern | Code |
|---------|------|
| HTTP handlers + event emit sites | `backend/servers/api-server/src/routes/platform_admin/audit.rs` |
| Route table + capability gates | `backend/servers/api-server/src/routes/platform_admin/mod.rs` (base nest `/api/v1/platform-admin` in `lib.rs`) |
| Event kinds, props structs, writer | `backend/crates/db/src/repositories/platform_admin.rs` (`SupportToolingEventKind`, `*Props`, `log_support_tooling_event`) |
| Table + immutability triggers | `backend/crates/db/migrations/00163_create_support_tooling_events.sql` |
| Audit hardening (FK RESTRICT, RLS) | `backend/crates/db/migrations/00165_support_tooling_events_audit_hardening.sql` |
| Retention (TTL) policy | `backend/crates/db/migrations/00222_support_tooling_events_retention.sql` + `cleanup_old_support_tooling_events` |
| Retention & PII posture (companion) | `docs/data/support-data-retention-privacy.md` |

## 2. The 2026-05-28 decision — separate trail, not the `AuditRead` surface

**Decision: support-staff read audit events are recorded in a dedicated,
append-only `support_tooling_events` table, decoupled from the `AuditRead`
capability gate and from the generic `audit_logs` surface that `AuditRead`
exposes.** Rationale, grounded in the current code:

1. **The capability that *gates the action* is not the capability that
   *reads the trail*.** Each support action is gated by the capability
   appropriate to *that* action (Section 4) — `AuditRead` for the aggregate
   diagnostics page, `UsersRead` for user lookups, `UsersWrite` for session
   revocation. If the audit event were emitted onto the `AuditRead`-gated
   `audit_logs` stream, the "who did support things" trail would be
   inconsistently coupled to one capability while the actions span three.
   Recording to a separate table keeps the *audit-of-support-staff* orthogonal
   to the *capability that authorised each support action*.

2. **`audit_logs` is itself what support staff *read*** (`get_user_activity`
   returns rows from `audit_logs`). Writing support-staff-activity events back
   into the same table an operator is reading would let the observer pollute
   the observed stream and complicate per-tenant activity queries. The two
   concerns are kept in separate tables on purpose (migration 00163 header).

3. **Different retention and referential posture.** `support_tooling_events`
   carries a 730-day retention window (migration 00222) and `ON DELETE
   RESTRICT` FK to `users` (migration 00165) so the trail outlives a deleted
   admin. These are policy choices specific to the support-staff trail and are
   cleaner to express on a dedicated table than as a filtered slice of
   `audit_logs`.

Consequence for consumers: **to answer "who viewed/searched/revoked what in
support tooling", query `support_tooling_events`, not `audit_logs`.** The two
are complementary, not redundant.

## 3. Storage model & core dimensions

One row per audited support action. Columns (migration 00163, mirrored by
`SupportToolingEventRow`):

| Property | Type | Description | Source |
|----------|------|-------------|--------|
| `id` | uuid | Row id (`gen_random_uuid()`). | DB default |
| `event_kind` | enum string | Which support action fired — Section 4. CHECK-constrained in-DB; mirrored by `SupportToolingEventKind`. | producer literal |
| `admin_user_id` | uuid | The platform SuperAdmin who performed the action — the acting identity. FK `users(id)` `ON DELETE RESTRICT` (00165). | `extract_super_admin_token` |
| `props` | jsonb | Event-kind-specific payload — Section 4. Defaults `'{}'`. | serialised `*Props` struct |
| `occurred_at` | timestamptz (UTC) | Event time. | DB default `NOW()` |

**Immutability.** Triggers reject `UPDATE` unconditionally and `DELETE` except
inside the sanctioned retention path (`app.retention_prune` GUC, migration
00222). The table is a tamper-evident log.

**Fire-and-forget.** Every emit site is non-fatal: a failure to persist the
tracking event only `warn!`s and never changes the user-facing response
(`audit.rs`). Consumers must therefore treat the trail as **best-effort
complete**, not transactionally guaranteed.

## 4. Event taxonomy & per-event schemas

Three event kinds are live today. Names are `snake_case`. The **gating
capability** column is the capability on the *triggering route* (per Section 2,
deliberately not uniformly `AuditRead`).

| `event_kind` | Emitted from (route) | Gating capability | Meaning |
|--------------|----------------------|:-----------------:|---------|
| `support_data_viewed` | `GET /api/v1/platform-admin/support-data` (`get_support_data`) | `AuditRead` | Admin opened/refreshed the aggregate Support Data diagnostics page. |
| `support_user_searched` | `GET /api/v1/platform-admin/support/users` (`search_users_for_support`) | `UsersRead` | Admin ran a per-user lookup / free-text search. |
| `support_sessions_revoked` | `POST /api/v1/platform-admin/support/users/{id}/sessions/revoke` (`revoke_user_sessions`) | `UsersWrite` | Admin revoked all active sessions for a target user. |

### 4.1 `support_data_viewed` — props (`SupportDataViewedProps`)

Captures the snapshot the admin was shown, so tooling usage can be correlated
with the platform state at view time.

| Property | Type | Notes |
|----------|------|-------|
| `tenant_count` | int (`i64`) | Platform-wide organisation count at time of view. |
| `fault_total` | int (`i64`) | Total fault count across all orgs at time of view. |

Note: this is a **platform-wide aggregate** view — the diagnostics page shows
counts only, no per-tenant or per-user PII (see
`support-data-retention-privacy.md`). It is therefore *not* scoped to a single
tenant; the "which tenant's diagnostics" question is answered per-tenant only
where a per-tenant diagnostic view exists — see **[gap A]** in Section 6.

### 4.2 `support_user_searched` — props (`SupportUserSearchedProps`)

Records search metadata so repeated lookups on one target are spottable, while
**deliberately omitting the raw query** (it commonly contains an email → PII).

| Property | Type | Notes |
|----------|------|-------|
| `query_length` | int? (`Option<i64>`) | Character count of the free-text query; `null` for an unfiltered listing. The literal string is **never** stored. |
| `status_filter` | string? | Status filter applied, if any. |
| `result_count` | int (`i64`) | Number of results returned. |

### 4.3 `support_sessions_revoked` — props (`SupportSessionsRevokedProps`)

Links the revocation to its target so the event is meaningful without a join.

| Property | Type | Notes |
|----------|------|-------|
| `target_user_id` | uuid | The user whose sessions were revoked. |
| `revoked_count` | int (`i64`) | Number of sessions actually revoked. |

## 5. PII & privacy posture

Consistent with `docs/data/support-data-retention-privacy.md`:

- **Identity is a UUID, never a name/email.** `admin_user_id` and
  `target_user_id` are opaque ids. The admin's email is available at the emit
  site (`extract_super_admin_token` returns it) but is **not** written to the
  event.
- **Raw search strings are excluded** — only `query_length` is kept (§4.2),
  because queries carry emails.
- **No IP / user-agent** is placed on these events (unlike `audit_logs`, which
  does carry them for the *actions being audited*).
- **Append-only + RESTRICT + RLS** (migrations 00163 / 00165) mean the trail is
  tamper-evident and survives admin-account deletion (deletion is refused while
  events exist).
- **Retention: 730 days** (migration 00222), the sanctioned lifetime balancing
  accountability against GDPR storage-limitation (Art. 5(1)(e)).

## 6. On the wire today vs. gaps

The three events in Section 4 are **live**. The following are the **target
contract** extensions this schema defines but that are not yet emitted —
recorded here so a future implementer does not invent a divergent field set.

| # | Gap | Today | To close |
|---|-----|-------|----------|
| A | **Per-user diagnostic *views* are not audited.** Opening a specific user's detail / memberships / sessions / activity (`get_user_for_support`, `get_user_memberships`, `get_user_sessions`, `get_user_activity`) reads PII but emits **no** `support_tooling_events` row — only *search* is logged. So "who *viewed* which user's diagnostics" is only partially captured. | 4 read handlers in `audit.rs` return without an emit call. | Define a `support_user_viewed` kind (props: `target_user_id`, `view_kind ∈ detail\|memberships\|sessions\|activity`) and add the fire-and-forget emit to each handler. Requires extending the migration-00163 `event_kind` CHECK constraint (a new migration) + a `SupportToolingEventKind` variant. |
| B | **No per-tenant diagnostics scope on `support_data_viewed`.** The aggregate page is platform-wide, so there is no `tenant_id`. A future per-tenant diagnostics view would need attribution. | `support_data_viewed` carries only counts. | When/if a per-tenant diagnostics view lands, add `tenant_id: uuid?` to its props (or a distinct `support_tenant_diagnostics_viewed` kind). |
| C | **No `request_id` / correlation key.** Each event stands alone; a single admin session's sequence of actions can only be reconstructed by `(admin_user_id, occurred_at)` ordering. | props have no correlation id. | Thread the request/trace id (already present in tracing spans) into props if session-level reconstruction is needed. |
| D | **Retention job is defined but not scheduled.** `cleanup_old_support_tooling_events` exists; wiring it into the api-server background scheduler is an out-of-scope follow-up (owner pm-backend, noted in migration 00222). | function callable, not invoked periodically. | Add the scheduled call alongside `cleanup_old_traces` / `cleanup_old_health_check_results`. |

## 7. Open questions (verify before relying on this doc)

1. **`support_user_viewed` granularity (gap A).** Should each of the four
   read sub-resources emit its own event, or one `detail` event per user open?
   Per-sub-resource is more precise for investigations but noisier. This doc
   proposes a single kind with a `view_kind` discriminator; confirm before
   building.
2. **Read-view volume.** If gap A is closed, per-user view events could be
   high-volume for an active support team; confirm the 730-day retention and
   the `admin_user_id` / `occurred_at` indexes (migration 00163) are sufficient
   for the intended dashboard queries.
3. **Cross-referencing `audit_logs`.** `get_user_activity` reads `audit_logs`;
   a support-staff investigation may want to *join* "support opened user X" with
   "what X's own audit trail shows". Confirm whether consumers need a documented
   join key beyond `target_user_id`.
