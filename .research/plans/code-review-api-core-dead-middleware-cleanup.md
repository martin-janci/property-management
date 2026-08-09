# code-review-api-core-dead-middleware-cleanup

**Vector:** security
**Score:** 5
**Source:** Tier1d dispatcher-generator 2026-08-08 (3 sibling api-core signals: rls-context-deprecated-exported, authz-mw-guest-fail-open, tenant-filter-noop-isolation)
**Confidence:** medium

## Hypothesis
`backend/crates/api-core/src/middleware/` exposes three middleware layers (`rls_context`, `authorization`, `tenant_filter`) that are dead in the current wiring but still glob re-exported from `middleware/mod.rs`. Each one is broken in a specific way — `rls_context` trusts the client-supplied `X-Tenant-ID` and mutates the pool instead of the borrowed connection; `authorization::require_permission` defaults an absent role to `Guest` (level 10) which satisfies the `AUTHENTICATED` gate at level 1, making it fail-open; `tenant_filter` is a pass-through that only checks header shape. A future route author reaching for these authoritative-looking helpers would silently break tenant isolation or auth. Remove them (preferred) or drop them from the `pub use` and gate behind `#[cfg(test)]` + `#[deprecated]` so they cannot be wired by accident. Live isolation stays via `host_tenant.rs` + the `RlsConnection` extractor.

## Evidence
- `backend/crates/api-core/src/middleware/rls_context.rs:1-37` — module header documents it as `# DEPRECATED` (`fundamentally flawed and should NOT be used`); still `pub mod rls_context;` (middleware/mod.rs:7) and `pub use rls_context::*;` (mod.rs:16). Fns derive tenant scope from the client-supplied `X-Tenant-ID` (rls_context.rs:87-91, :207-218) and call `set_request_context(pool.as_ref(), …)` on the pool (rls_context.rs:110-116, :232-238), so the GUC lands on a random connection while the handler queries a different one.
- `backend/crates/api-core/src/middleware/authorization.rs:176-180` — `require_permission` reads role via `.unwrap_or(TenantRole::Guest)`; `TenantRole::Guest.level() == 10` (common/src/tenant.rs:125) satisfies `permissions::AUTHENTICATED = Permission::min_level(1, ...)` (authorization.rs:139), so `require_authenticated` (authorization.rs:236-241) passes for a caller with no principal at all. Still `pub mod` + glob-re-exported (mod.rs:3,12).
- `backend/crates/api-core/src/middleware/tenant_filter.rs:14-42` — named/documented as tenant-isolation guard but only checks `X-Tenant-ID` parses as a UUID; real checks are an inline TODO. Still `pub mod` + glob-re-exported (mod.rs:9,18).
- `grep -rn 'rls_context_middleware|require_rls_context|require_permission|require_authenticated|tenant_filter' backend/servers backend/crates` — zero `.layer(...)` wirings outside the module itself; `host_tenant.rs:9-11` explicitly notes it superseded the deprecated `rls_context` middleware.

## Files
- `backend/crates/api-core/src/middleware/rls_context.rs`
- `backend/crates/api-core/src/middleware/authorization.rs`
- `backend/crates/api-core/src/middleware/tenant_filter.rs`
- `backend/crates/api-core/src/middleware/mod.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug/security removal — verify the greps are exhaustive before deleting)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [x] C7 — Code-review reception (security-touching PR)

**Execution mode (auto-derived from the ticks):** neither C4 nor C5 is ticked.

Mode: cloud-ok

## Repro steps
1. Check out `dev`. `grep -rn 'use api_core::middleware::\*\|rls_context::\|require_permission\|require_authenticated\|tenant_filter' backend/` returns zero wirings today.
2. Simulate the trap by adding a fresh route in a scratch handler that wires `axum::middleware::from_fn(api_core::middleware::require_authenticated)` (no auth extractor before it). Send an unauthenticated request; the handler runs (fail-open). This is the failure mode the plan removes.
3. Expected after the change: `require_authenticated` and siblings are unreachable from `use api_core::middleware::*;` (either deleted, `#[cfg(test)]`-gated, or `#[deprecated(note = "removed — use host_tenant + RlsConnection")]`).

## Suggested approach
1. Re-run the greps in *Evidence* on the current `dev` HEAD to confirm zero live wirings in `backend/servers` and `backend/crates`. Add tests to a new `backend/crates/api-core/tests/no_dead_middleware_reachable.rs` that asserts the symbols are no longer part of `api_core::middleware::*` (compile-fail via `trybuild`, or a plain doctest that fails to name the removed items).
2. Delete `rls_context.rs`, `authorization.rs`, and `tenant_filter.rs`; drop their `pub mod`/`pub use` from `middleware/mod.rs`. If any doc-comment in the crate references the deleted items, remove/redirect it to `host_tenant.rs` + `RlsConnection`.
3. If the review pushes back on deletion, downgrade to: keep the files, remove `pub use` and change `pub mod` → `#[cfg(test)] mod`, and mark each fn `#[deprecated(note = "…")]`. Explicitly document in `middleware/mod.rs` that the live path is `host_tenant` + `RlsConnection`.
4. Run `cargo test -p api-core` and `cargo test -p api-server` to catch anything that transitively imported the removed symbols.
5. Grep the whole workspace one more time for the removed symbol names; ensure nothing (docs, examples, sqlx offline data) references them.

## Alternatives considered
- **Keep files but only remove `pub use` from mod.rs** — rejected because `pub mod` alone still lets a downstream write `api_core::middleware::rls_context::require_rls_context(...)`. The fail-open surface stays reachable; the fix has to remove `pub mod` too.
- **Fix the middleware in place (verify principal, use connection-scoped context, add real tenant check)** — rejected because the correct implementation already exists (`host_tenant.rs` + `RlsConnection`) and the deprecated modules were explicitly superseded. Re-implementing means maintaining two parallel isolation paths.

## Root-cause trace
1. Symptom: `api_core::middleware::require_authenticated` (and siblings) are reachable via glob re-export and would fail-open if wired.
2. ← Immediate cause at `middleware/mod.rs:3,7,9,12,16,18` — `pub mod` + `pub use *` keeps deprecated modules in the public surface.
3. ← Upstream cause at `middleware/authorization.rs:176-180` — `.unwrap_or(TenantRole::Guest)` treats absence as Guest instead of denying; sibling files carry the same design (trust client header, no principal check, pool-scoped GUC).
4. Origin: original api-core middleware scaffold predating the `host_tenant` + `RlsConnection` migration. `host_tenant.rs:9-11` documents the supersession but the dead files were never cleaned up.

## Test plan
- [ ] `backend/crates/api-core/tests/no_dead_middleware_reachable.rs` — a compile-fail (trybuild) or `#[deny(unused_imports)]` test that fails on `use api_core::middleware::{rls_context, authorization, tenant_filter};` after the change.
- [ ] `backend/crates/api-core/tests/…` — if we downgrade to the deprecation path, add a `#[deprecated]` warning assertion.
- [ ] `cargo test -p api-core && cargo test -p api-server` — everything still compiles/passes (nothing in servers depended on the removed symbols).
- [ ] `cargo clippy -p api-core -- -D warnings` — no stray dead-code warnings from partial removal.

## Out of scope
- Refactoring `host_tenant.rs` or `RlsConnection` (the live path).
- Adding new authorization or tenant-filter middleware.
- Any change to route wiring in `backend/servers/*`.

## After-merge
- Move this file to `plans/_archive/code-review-api-core-dead-middleware-cleanup.md`
- Mark the matching `backlog.json` row as `status: "done"`
