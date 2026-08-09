# code-review-api-core-rls-context-pool-not-conn

**Vector:** bug
**Score:** 3
**Source:** tier1d dispatcher signal 2026-08-09 (segment api-core, ppt-dev-review rust expert)
**Confidence:** medium

## Hypothesis
`api-core::middleware::rls_context::require_rls_context` is documented (rls_context.rs:181-184) as the strict guard "Use this for routes that MUST have tenant isolation", but it sets the RLS session GUCs on a transient pool connection instead of the connection the handler later checks out. Handlers past this middleware run their queries on a different pooled connection where `app.tenant_id` / `app.user_id` are unset, so PostgreSQL RLS policies see NULL context and either fall open or deny — either way the middleware's promised isolation is not applied. The module header (rls_context.rs:1-37) already diagnoses the exact defect and marks the module deprecated, yet no route today mounts it, and it is still `pub use rls_context::*;` re-exported from `middleware/mod.rs:16` — a latent trap for the next author who trusts the surviving docstring. Fix: delete the deprecated module (correct path is `extractors::rls_connection::RlsConnection`, which sets the GUCs on the very connection it hands the handler) and stop re-exporting it, so a future route cannot accidentally opt in to no-op tenant isolation.

## Evidence
- `backend/crates/api-core/src/middleware/rls_context.rs:185-251` — `require_rls_context` calls `db::tenant_context::set_request_context(pool.as_ref(), Some(tenant_id), Some(user_id), is_super_admin)` (line 232), then `next.run(request).await` (line 245). The `set_request_context<E: Executor>` implementation at `backend/crates/db/src/tenant_context.rs:18` issues `SET LOCAL`/`set_config` on whatever executor it is passed — when that executor is `&Pool`, sqlx borrows a transient conn just for the SET and returns it. The handler behind `next.run(...)` then acquires a **different** pooled conn for its queries.
- `backend/crates/api-core/src/middleware/rls_context.rs:1-37` — the module's own header states "Setting RLS on the pool doesn't affect individual connections … PostgreSQL session settings are connection-scoped" and marks the module DEPRECATED, yet the file lacks `#[deprecated]` and `require_rls_context`'s docstring at lines 181-184 still recommends it.
- `backend/crates/api-core/src/middleware/mod.rs:16` — `pub use rls_context::*;` glob-re-exports the broken `require_rls_context` / `rls_context_middleware`, keeping them as first-class API of `api-core::middleware` despite the deprecation notice.
- `backend/crates/api-core/src/extractors/rls_connection.rs` — the correct pattern: `RlsConnection` extractor acquires a dedicated conn, applies `set_config` on that same conn, and hands it to the handler; queries run on a connection whose GUCs are actually set.
- No servers mount the middleware today (grep of `backend/servers/` under `require_rls_context` / `rls_context_middleware` returns no hits), so the current risk is a latent bypass — a future author following the docstring gets a compiling, plausible-looking guard that provides zero isolation.

## Files
- `backend/crates/api-core/src/middleware/rls_context.rs`
- `backend/crates/api-core/src/middleware/mod.rs`
- `backend/crates/api-core/src/extractors/rls_connection.rs`
- `backend/crates/db/src/tenant_context.rs`

## Dependencies
<none>

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

Mode: cloud-ok

## Repro steps
1. Mount `require_rls_context` on any tenant-scoped GET route (e.g. list buildings), then in an integration test authenticate as tenant A and query tenant B's building id — expected: 403/404 (RLS filters row); actual (today, if the route were wired): the query hits the pool for a fresh conn whose `app.tenant_id` GUC is unset, so `set_config('app.tenant_id', ..., true)` was never applied to the executing conn — RLS policies see a NULL tenant context and the query returns whatever the base RLS policy allows on NULL (usually 0 rows *by luck*, not by guard). Any policy that permits NULL context, or code paths that skip RLS for admin, would leak cross-tenant.
2. Contrast with `RlsConnection`: the extractor acquires the conn, applies `set_config` on that same handle, and passes the handle into the handler — the same test returns the expected 403/404.

## Suggested approach
1. Delete `backend/crates/api-core/src/middleware/rls_context.rs` entirely (module has been marked deprecated since its own header, no live call sites in `backend/servers/`).
2. Remove `pub mod rls_context;` + `pub use rls_context::*;` from `backend/crates/api-core/src/middleware/mod.rs`.
3. Grep the workspace one more time for `require_rls_context`, `rls_context_middleware`, and `set_request_context` on a `Pool` executor; anywhere the last pattern still exists (should be zero after step 1) rewrite to acquire a conn first and pass `&mut *conn`.
4. Add a compile-only regression test in `backend/crates/api-core/tests/`: assert that `require_rls_context` symbol no longer resolves (e.g. `compile_fail` doc-test or a `#[cfg(any())]` sentinel) so its re-introduction fails CI.
5. Add a runtime regression test using `RlsConnection` that authenticates as tenant A and asserts that a cross-tenant read returns 0 rows / 404, guarding the *correct* path so future refactors don't regress.
6. Update the module docstring on `extractors/rls_connection.rs` to explicitly recommend it as the canonical tenant-isolation entry point (so search discovers the right API).
7. Grep any `.research/plans/` or `docs/` references to `require_rls_context` and update them.

## Alternatives considered
- **Fix `require_rls_context` in place (make it acquire a conn from the pool, set GUCs, then attach the conn to request extensions for the handler)** — rejected because the middleware pattern requires the handler to *know* to pull the conn from extensions rather than the pool; `RlsConnection` already does the ergonomic thing without a hidden contract, and the deprecated header explicitly says the correct path is the extractor.
- **Add `#[deprecated]` + `compile_error!` guards without deleting the module** — rejected because the surviving docstring is the trap; a still-exported function named `require_rls_context` invites future authors even with a deprecation warning. Deletion + hard test is the only guarantee the footgun cannot be re-armed by a rebase or a copy-paste.

## Root-cause trace
1. Symptom: (latent) `require_rls_context` returns 200 with unset tenant GUCs on the handler's connection — no error, no log, silent bypass.
2. ← `require_rls_context` at `backend/crates/api-core/src/middleware/rls_context.rs:232` passes `pool.as_ref()` (i.e. `&Pool<Postgres>`) to `set_request_context`, so the SET runs on a transient pool checkout and is dropped when that checkout returns to the pool.
3. ← `set_request_context<E: Executor>` at `backend/crates/db/src/tenant_context.rs:18` accepts any executor; when the caller hands it a pool, PostgreSQL scopes the setting to that transient connection only — the handler's downstream pool acquire gets a different conn with no GUC set.
4. Origin: the module's own header (rls_context.rs:1-37) documents the discovery of this defect and the migration to `RlsConnection`, but the module was left in place with a docstring on `require_rls_context` that recommends it — deprecation was descriptive-only, not enforced, and the `pub use` re-export at `middleware/mod.rs:16` keeps the trap loaded.

## Test plan
- [ ] Compile-only guard: assert `use api_core::middleware::require_rls_context;` no longer resolves (add `tests/ui/no_require_rls_context.rs` with `compile_fail` doc-test or a `trybuild` case).
- [ ] Positive integration test in `backend/crates/api-core/tests/` (or wherever `RlsConnection` is currently covered): tenant A queries tenant B's row through a route protected by `RlsConnection`, assert 0 rows / 404, so the correct path stays honest.
- [ ] `cargo test -p api-core` locally (or `cargo test -p api-core --lib -p api-core --tests`); backend CI (`backend.yml`) must be green.

## Out of scope
- Rewriting or auditing other middlewares under `api-core::middleware/`.
- Any change to `RlsConnection` semantics (only recommend it in docs).
- Migrating existing routes to `RlsConnection` where they already use their own pattern; only the removal + the docs update belong in this PR.

## After-merge
- Move this file to `plans/_archive/code-review-api-core-rls-context-pool-not-conn.md`
- Mark the matching `backlog.json` row as `status: "done"`
