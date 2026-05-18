# Tenant Operability Runbook (Phase 5.5)

This runbook covers the operability surface introduced in Phase 5.5: per-tenant
**export**, **purge**, and **restore**, plus the supporting safety nets
(soft-delete + RLS filter, per-tenant rate limit, per-tenant metering, and the
tenant-prefixed Redis wrapper). Read this before exercising any of the
`/api/v1/admin/tenants/*` endpoints in production.

The roadmap entry that authorizes this work is `docs/multitenancy/ROADMAP.md`,
section *Phase 5.5 — Tenant Lifecycle & Operability*. The brainstorming session
references are leak #15, #16, #17, #18, #19, #20, and #21.

---

## Components and where they live

| Surface | Path |
|---|---|
| Soft-delete column | `backend/crates/db/migrations/00139_organizations_soft_delete.sql` |
| RLS soft-delete filter on every tenant policy | `backend/crates/db/migrations/00140_rls_soft_delete_filter.sql` |
| Manifest extractor + RLS gate | `backend/scripts/check-rls-coverage.sh --emit-manifest <PATH>` |
| Manifest snapshot (committed) | `backend/manifests/tenant-data-manifest.json` |
| Soft-delete migration generator | `backend/scripts/generate-soft-delete-rls-migration.sh` |
| Per-tenant rate limit + metering | `backend/crates/api-core/src/middleware/tenant_ops.rs` |
| Tenanted Redis wrapper | `backend/crates/api-core/src/cache/tenanted_redis.rs` |
| no-raw-redis CI gate | `backend/scripts/lints/no-raw-redis.sh` |
| Export / purge / restore primitives | `backend/crates/tenant-ops/` |
| Admin HTTP routes | `backend/servers/api-server/src/routes/admin_tenant_lifecycle.rs` |

The admin route mounts at:

```
POST /api/v1/admin/tenants/{id}/export
POST /api/v1/admin/tenants/{id}/purge
POST /api/v1/admin/tenants/restore   (multipart upload of a previously-exported tarball)
```

All three require `is_platform_admin()` today. Phase 5's `admin-core` will
swap that for a true capability + MFA gate (`Capability::TenantExport`,
`TenantPurge`, `TenantRestore`); the route signatures stay stable.

---

## Per-tenant logical export (defense #18)

```
POST /api/v1/admin/tenants/{org_id}/export
{ "out_dir": "/var/ppt/exports" }
```

Output: a `.tar.gz` at `out_dir/tenant-<org_id>-<UTC stamp>.tar.gz` containing

* `metadata.json` — exported_at, manifest_version, manifest_git_sha
* `manifest.json` — verbatim copy of the manifest used to drive the export
* `tables/organizations.ndjson` — the org row itself
* `tables/<table>.ndjson` — one NDJSON file per directly-org-scoped table

The export runs in super-admin RLS context so soft-deleted tenants are also
exportable. Encryption is the responsibility of the storage layer (KMS-encrypted
S3 / encrypted EBS); the runbook assumption is that the export directory is
itself an encrypted volume. *Do not* publish exports unencrypted.

### Backup configuration (defense #18)

* Production exports MUST land in a bucket with SSE-KMS enabled and bucket
  policy restricting reads to the operator role.
* Per-tenant exports are scheduled nightly via the platform job runner. The
  retention policy is 30 days hot + 365 days cold (Glacier).
* A weekly restore-rehearsal job picks one random export, restores it into a
  throw-away org id in the staging cluster, asserts row counts via the
  `manifest.json` embedded in the tarball, and tears the org down again.
  Failed rehearsals page the on-call engineer.

---

## Per-tenant purge (defense #17)

```
POST /api/v1/admin/tenants/{org_id}/purge
```

The plan is read from `backend/manifests/tenant-data-manifest.json`. Every
direct-org-scoped table gets `DELETE WHERE <org_col> = $1`, in *reverse*
manifest order. Child tables are cleared via cascade FKs already in the schema.
The organization row is dropped last.

CI guarantees the manifest is current: `check-rls-coverage.sh --strict`
(invoked in PRs) regenerates and diffs the manifest. A new tenant table
without a manifest entry fails the gate, which means the purge plan can
never silently miss a table.

Operationally:

1. Run an export FIRST (defense #18 — purge is irreversible at the DB layer).
2. Confirm with a second human (4-eye policy for production purges).
3. Run the purge and capture the `PurgeReport`. The report's
   `s3_keys_to_delete` is empty in the current build — the storage sweep
   runs out-of-band via `integrations::storage::delete_prefix("t/<org_id>/")`.
4. Verify the org row is gone: `SELECT * FROM organizations WHERE id = $1`
   in super-admin context returns zero rows.

---

## Soft-delete + restore (defense #16)

Soft-delete is the default tooling path. `organizations.deleted_at IS NOT NULL`
hides the org and ALL its data from any tenant context via the
`get_current_org_not_deleted()` helper baked into every tenant policy in
migration 00140. Super-admin context still sees the row.

Restore is **never in place**. The restore endpoint mints a fresh
`organization_id` and rewrites every imported row to point at the new id. The
slug is suffixed (`<slug>-restored-<6-char-hex>`) to avoid the UNIQUE
collision. Operators MUST then communicate the new slug / domain to the
restored tenant.

```
POST /api/v1/admin/tenants/restore   (multipart, single file part = tarball)
```

---

## Per-tenant rate limit (defense #15)

A `TenantRateLimiterSet` lives in `api-core` and is intended to be wired into
the keystone middleware at the `SEAM(leak#15)` markers in
`host_tenant_middleware`. Default: 600 req/min per tenant. Per-tenant
overrides can be installed via `TenantRateLimiterSet::set_override(org, rpm)`
(typically loaded at boot from Phase 5's `tenant_settings.rate_limit_rpm`).

Operationally: a flooding tenant gets `429 Too Many Requests` without
affecting any other tenant's latency. A dashboard graph of
`requests_total{org_id="..."}` overlaid on `429_total{org_id="..."}` flags
abusive traffic.

---

## Per-tenant metering (defense #19)

Every wired request emits, with an `org_id` label:

* `requests_total` — counter
* `request_bytes_total` — counter (response body bytes)

These feed the abuse-detection dashboard and the billing pipeline. Cardinality
risk is bounded — at most one label value per platform tenant, and the
metering is silent for the platform-host (no resolved tenant).

---

## Tenanted Redis (defense #20)

Every Redis read/write must go through `api_core::cache::TenantedRedis`. The
wrapper prefixes every key with `t:<org_id>:` so two tenants writing the same
logical key cannot collide. Direct `redis::Client`/`redis::Cmd`/`use redis::`
outside the allowlisted files (`integrations/src/redis.rs` and the wrapper
itself) fails the CI gate `backend/scripts/lints/no-raw-redis.sh`.

Operationally: a missed prefix is impossible by construction *if* the lint
gate is enabled in CI. Add it to the workflow that runs on every PR.

---

## MFA enforcement for platform principals (defense #21)

The current admin routes in this phase use `is_platform_admin()` only. Phase 5
finalizes MFA enforcement: every capability-gated platform action requires a
recent MFA challenge (`mfa_verified_at` within N minutes). Until that lands,
operators with platform principals MUST:

* Log into the admin console only from a hardware-MFA-protected device.
* Never share session cookies / JWTs.
* Rotate the platform password on any compromise rumor.
* Never run `purge` from a session that wasn't authenticated within the last
  15 minutes (manual discipline; automated check arrives in Phase 5).

---

## Break-glass procedure

1. Page the on-call engineer; declare incident severity.
2. Acquire a fresh platform principal session on the engineer's hardware-MFA
   device.
3. Perform the change with a second on-call witnessing.
4. Capture the audit log entries (Phase 5's `SupportActivityLog`) and post
   them in the incident channel within 1 hour.
5. File a post-mortem within 5 business days.

---

## GDPR purge SLA

* Acknowledgement: within 24 hours of receipt.
* Verification of right-to-erasure: within 5 business days.
* Execution (export-then-purge): within 30 calendar days of verified request.
* Confirmation to data subject: within 24 hours of purge completion.

The `PurgeReport` is the legal record. Archive it (KMS-encrypted) for the
retention period required by your jurisdiction.

---

## Restore-rehearsal cadence

* **Daily**: random sample of one export per region, restore-then-drop in
  the staging cluster. Page on failure.
* **Weekly**: full-tenant restore for the largest active tenant, end-to-end,
  including a smoke test against the restored org's `tenant-config`.
* **Quarterly**: cross-region restore drill (export from region A, restore in
  region B). Confirms the export format is region-agnostic.

If a rehearsal fails, the export pipeline is broken — treat as P1.

---

## Capability scope (D2.1)

Capability grants live in `capability_grants` and are deliberately
**platform-scoped** — there is no `organization_id` column on the row.
Granting `Capability::AgenciesWrite` makes the bearer able to write to
**any** agency the request can reach via RLS; org-scoping is a job for
the data layer (RLS + the resolved tenant context), not the capability.
The full rationale, including why we did NOT add `organization_id` to
the grant row when wiring `AuthPolicyEnforcer::check_capability_revoke`,
lives in
[`decisions/capability-platform-scope.md`](decisions/capability-platform-scope.md).

---

## CI gates

### WebSocket Tenant Enforcement (Phase 6 / leak #5 & #20 risk)

Script: `backend/scripts/check-ws-tenant-enforcement.sh`

**Why it exists.** WebSocket upgrade handlers in Axum sit outside the normal
HTTP response middleware chain.  A developer adding a WS route (e.g. for
real-time notifications) might mount it on a bare `Router` that never passes
through `host_tenant_middleware`, silently leaking cross-tenant events
(brainstorming leak #5 — unauthenticated socket handshake — and leak #20 —
tenanted-Redis key collision via unscoped pub/sub).

The gate runs `ripgrep` against `backend/servers/` looking for any of:

* `axum::extract::ws::WebSocketUpgrade`
* `WebSocketUpgrade`
* `tokio_tungstenite::accept_async`

For each match it checks that `host_tenant_middleware` (or `from_fn_with_state`
pairing it) appears in the same file **or** anywhere else in the same crate's
`src/` tree.  If no co-located reference is found, the gate fails.

The gate exits 0 when no WS code exists at all (the state at Phase 6 launch)
so it is safe to enable immediately in CI without any pre-existing violations.

**Convention for adding a compliant WS route.**  Every WebSocket router MUST
apply the middleware before handing off to the upgrade extractor:

```rust
let ws_router = Router::new()
    .route("/ws/notifications", get(ws_handler))
    .route_layer(from_fn_with_state(state.clone(), host_tenant_middleware));
```

The middleware resolves the tenant from the `Host` header (or `X-Tenant-Slug`
in dev mode) and injects `TenantContext` before the WS handshake completes.
Do **not** call `WebSocketUpgrade::on_upgrade` before `TenantContext` is
available in the request extensions.

**CI wiring.** The step `WebSocket Tenant Enforcement Check` runs immediately
after `RLS Enforcement Check` in the `check` job of `.github/workflows/backend.yml`,
with `working-directory: backend` and the `--strict` flag.

## Open items deferred from Phase 5.5

* True capability + MFA gating on the admin routes lands in Phase 5
  (`admin-core::Capability::{TenantExport,TenantPurge,TenantRestore}`).
* The actual rate-limit middleware wiring (currently only `SEAM` markers in
  `host_tenant_middleware` + the standalone `TenantRateLimiterSet`) lands in
  the same Phase 5 sweep that adds the metering instrument to `next.run`.
* S3 prefix sweep on purge: `integrations::storage::delete_prefix("t/<org>/")`.
  The `PurgeReport.s3_keys_to_delete` field is reserved but currently empty;
  the prefix sweep runs out-of-band today.
* Background `TenantedRedis` job-side wiring — only the wrapper exists; the
  audit of every `RedisClient::*` call site is Phase 5 work.
