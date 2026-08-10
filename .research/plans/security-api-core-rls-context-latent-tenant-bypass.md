# security-api-core-rls-context-latent-tenant-bypass

**Vector:** security
**Score:** 3
**Source:** signal `code-review-api-core-rls-context-pool-not-conn` (Tier-1d dispatcher generator, 2026-08-09)
**Confidence:** medium

## Hypothesis
`backend/crates/api-core/src/middleware/rls_context.rs` still exports a `require_rls_context` axum middleware whose docstring reads "use this for routes that MUST have tenant isolation", but the middleware sets the RLS context on a pooled connection it immediately returns to the pool, while the downstream handler acquires a different pooled connection where the RLS GUCs are unset. The module header already admits the pattern is broken and marks the file DEPRECATED, yet the module lacks a `#[deprecated]` attribute and is glob-re-exported (`pub use rls_context::*;` in `middleware/mod.rs:16`) — so a future route author following the guidance gets a compiling, plausible-looking tenant-isolation guard that leaks across tenants with no error. Delete the module (or hard-guard it) so no one can adopt the trap, while keeping the correct `RlsConnection` extractor path as the only surviving API.

## Evidence
- `backend/crates/api-core/src/middleware/rls_context.rs:1-37` — module header self-diagnoses the defect ("Setting RLS on the pool doesn't affect individual connections"; "PostgreSQL session settings are connection-scoped, not pool-scoped") and states "should NOT be used"
- `backend/crates/api-core/src/middleware/rls_context.rs:181-251` — `require_rls_context` still exists, its docstring still says "use this for routes that MUST have tenant isolation", and it calls `db::tenant_context::set_request_context(pool.as_ref(), ...)` (line 232) on a pool executor
- `backend/crates/api-core/src/middleware/mod.rs:16` — `pub use rls_context::*;` glob re-exports `require_rls_context` (and the sibling `rls_context_middleware`) as `api_core::middleware::require_rls_context`, i.e. the trap is publicly reachable API surface
- `backend/crates/api-core/src/extractors/rls_connection.rs` — the correct pattern: `RlsConnection` acquires a dedicated connection and sets RLS on THAT connection before handing it to the handler, so the GUCs and the query share a session
- Grep of `backend/servers/` for callers finds no mount site today, so severity is latent (medium confidence) rather than a live production bypass — but the module is compiling, exported, and documented for adoption

## Files
- `backend/crates/api-core/src/middleware/rls_context.rs`
- `backend/crates/api-core/src/middleware/mod.rs`
- `backend/crates/api-core/src/extractors/rls_connection.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug/security — trace which module paths re-export the trap)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):** `Mode: cloud-ok`

## Repro steps
1. Author a new route that mounts `require_rls_context` (following the docstring): `router.layer(axum::middleware::from_fn_with_state(pool.clone(), require_rls_context))`, then run an integration test that inserts rows for tenants A and B and requests them as tenant A.
2. Expected: only tenant A rows come back. Actual: both A and B rows come back — the RLS GUC set inside the middleware is bound to a pool connection that is returned before the handler runs, and the handler's own pool checkout sees no `app.current_tenant_id` set.
3. Equivalent assertion without a live DB: unit-test that the middleware sets `set_config('app.current_tenant_id', ...)` on the same `PgConnection` the handler later executes on — the check fails today.

## Suggested approach
1. Add `#[deprecated(note = "Use the RlsConnection extractor instead — this middleware sets RLS on a pool connection that isn't the one the handler runs on.")]` to `require_rls_context`, `rls_context_middleware`, and any other public item in `rls_context.rs`.
2. Replace `pub use rls_context::*;` in `middleware/mod.rs:16` with an explicit import list that omits the broken symbols (or drop the re-export entirely) so `api_core::middleware::require_rls_context` no longer resolves.
3. Delete `rls_context.rs` (and `pub mod rls_context;` in `middleware/mod.rs:7`) once step 2 confirms no consumer imports it — the file's own header already declares it dead, and the correct pattern (`RlsConnection`) lives in `extractors/`.
4. Add a doc-comment on `RlsConnection` in `extractors/rls_connection.rs` pointing readers to it as the sole tenant-isolation entry point (short paragraph, no code changes to the extractor itself).
5. Grep the workspace (`backend/` only) for `require_rls_context`, `rls_context_middleware`, `api_core::middleware::rls_context` to prove no consumer remains before the delete; if any exist, migrate them to `RlsConnection` in the same PR.
6. Run `cargo check -p api-core -p api-server -p reality-server` — a compile failure is the trigger to migrate a caller, not to revert.

## Alternatives considered
- **Fix the middleware in place (make it set RLS on the same connection the handler uses)** — rejected because axum middleware runs before extractors, so the middleware cannot know which connection the handler will check out; the whole architectural shape is wrong, which is why the file's own header calls it deprecated. Repairing it in place would re-introduce the same trap under a different name.
- **Leave the module and add a runtime `panic!("do not use")`** — rejected because compilation would still succeed, docs still advertise the API, and the trap only fires on the first request in production. Compile-time deletion (or `#[deprecated]` + explicit-re-export removal) fails at build time, which is when we want the failure.

## Root-cause trace
1. Symptom: a handler mounted behind `require_rls_context` returns rows for tenants other than the request's tenant.
2. ← `next.run(request).await` at `backend/crates/api-core/src/middleware/rls_context.rs:245` runs the handler AFTER the middleware's `set_request_context` call has already released its pool connection.
3. ← `db::tenant_context::set_request_context(pool.as_ref(), ...)` at `rls_context.rs:232` runs `SET app.current_tenant_id = ...` on a `&DbPool` executor — sqlx checks out a transient connection, runs the SET, and returns it to the pool immediately (session GUCs stay on that connection only).
4. ← The handler then extracts its own `sqlx::PgConnection` from the pool for its query — a different connection with no `app.current_tenant_id` set → RLS policies see NULL → policies that guard against NULL leak rows, and policies that don't gate on tenant_id at all pass everything.
5. Origin: this file was added deliberately as a first-pass RLS middleware, then superseded by the `RlsConnection` extractor once the pool-vs-connection distinction was understood — but the original module was never removed and its `#[deprecated]` never applied, so the docstring on `require_rls_context` still lures adoption.

## Test plan
- [ ] `cargo build -p api-core --all-targets 2>&1 | grep -E 'error\[.*\]: unresolved import.*rls_context|warning:.*deprecated'` — asserts either the delete path (unresolved-import errors on any surviving consumer) OR the `#[deprecated]` path (a deprecation warning fires on any call site).
- [ ] Grep regression: `rg -n 'require_rls_context|rls_context_middleware' backend/` produces zero non-test hits after the change.
- [ ] `cargo test -p api-core middleware::rls_context` — no residual test module referencing the deleted symbols; if the delete removes the `#[cfg(test)] mod tests` too, this is a "no test named X" pass, not a failure.
- [ ] Local run command: `cargo check -p api-core -p api-server -p reality-server && cargo clippy -p api-core --all-targets -- -D warnings`

## Out of scope
- Auditing every backend route to ensure it uses `RlsConnection` (that's a separate sweep — this plan only removes the trap).
- Refactoring or hardening `RlsConnection` itself (it is the correct pattern; leave it alone).
- The sibling `code-review-api-core-authz-mw-guest-fail-open` (`require_permission` unwrap_or(Guest) fail-open in `authorization.rs`) — separate defect, separate file, separate plan.
- Backporting `#[deprecated]` to already-terminated modules unrelated to RLS.

## After-merge
- Move this file to `plans/_archive/security-api-core-rls-context-latent-tenant-bypass.md`
- Mark `code-review-api-core-rls-context-pool-not-conn` in `backlog.json` as `status: "done"`
