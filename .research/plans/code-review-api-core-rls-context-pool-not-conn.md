# code-review-api-core-rls-context-pool-not-conn

**Vector:** bug
**Score:** 5
**Source:** tier1d-dev-review segment=api-core (2026-08-08, 08-09)
**Confidence:** medium

## Hypothesis
`backend/crates/api-core/src/middleware/rls_context.rs` contains a deprecated middleware pair (`rls_context_middleware`, `require_rls_context`) whose module header explicitly warns "fundamentally flawed and should NOT be used" — yet the module is still `pub mod rls_context;` and glob re-exported via `pub use rls_context::*;` in `middleware/mod.rs`. Neither symbol carries `#[deprecated]`. The functions call `set_request_context(pool.as_ref(), ...)` on the pool, so the RLS `SET` runs on a connection that pgpool immediately returns; the handler later borrows a different connection where the GUC is unset — silently leaking cross-tenant. Additionally both derive tenant from the client-supplied `X-Tenant-ID` header with no authorization check that the principal belongs to that org. Currently latent (no route wires this middleware), but the docstring on `require_rls_context` says "use this for routes that MUST have tenant isolation", so a future author adopting it in good faith gets a silent isolation bypass. Fix: delete the module (canonical path is the `RlsConnection` extractor), or add `#[deprecated]` + drop the glob re-export.

## Evidence
- `backend/crates/api-core/src/middleware/rls_context.rs:1-37` — header comment: "# DEPRECATED", "Setting RLS on the pool doesn't affect individual connections", "PostgreSQL session settings are connection-scoped, not pool-scoped".
- `backend/crates/api-core/src/middleware/rls_context.rs:185` — `pub async fn require_rls_context(...)` — docstring says "use for routes that MUST have tenant isolation", but the body still calls `set_request_context(pool.as_ref(), Some(tenant_id), Some(user_id), is_super_admin)` (line 232) on the pool.
- `backend/crates/api-core/src/middleware/rls_context.rs:87-91` and `:207-218` — parse `X-Tenant-ID` from the request header and pass it straight to `set_request_context` with no verification the principal belongs to that org.
- `backend/crates/api-core/src/middleware/mod.rs:7,16` — `pub mod rls_context;` + `pub use rls_context::*;` re-export both broken symbols to consumers.
- `backend/crates/db/src/tenant_context.rs:18` — `set_request_context<E: Executor>` — pool executor variant is inherently transient because pgpool checks a connection in/out for each statement.

## Files
- `backend/crates/api-core/src/middleware/rls_context.rs`
- `backend/crates/api-core/src/middleware/mod.rs`
- `backend/crates/db/src/tenant_context.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [x] C7 — Code-review reception (security-adjacent change to shared middleware surface)

**Mode: cloud-ok**

Mode: cloud-ok

## Repro steps
1. `grep -rn 'require_rls_context\|rls_context_middleware' backend/servers backend/crates` — currently returns only the definition site plus the `pub use` — no live callers.
2. Read `backend/crates/api-core/src/middleware/rls_context.rs:1-37` — the module header itself says the code is unsafe to use.
3. Read `require_rls_context` at :185 — the docstring says "use for tenant isolation"; the body does not deliver it.
   **Expected:** either the symbol is gone or it carries `#[deprecated]` + compile-time nudge and is not glob-exported.
   **Actual:** both broken symbols are part of api-core's public surface with no guardrail.

## Suggested approach
1. Prefer full removal: delete `backend/crates/api-core/src/middleware/rls_context.rs`; drop `pub mod rls_context;` and `pub use rls_context::*;` from `middleware/mod.rs`. Update any docs that mention the module (rg for `require_rls_context` under `docs/` and `backend/CLAUDE.md`).
2. If the user prefers a soft-deprecation path (to keep the module name grep-able for one release cycle), instead: add `#[deprecated(note = "Use the RlsConnection extractor — this middleware sets RLS on the pool, not on the handler's connection")]` to both `rls_context_middleware` and `require_rls_context`, and switch `pub use rls_context::*;` to `pub use rls_context::{rls_context_middleware, require_rls_context};` so the export list is explicit (no future function silently graduates to the public surface).
3. Add a compile-fail doc-test (or a `#[cfg(any())]` guarded stub) that asserts calling `require_rls_context` produces a deprecation warning when built with `-D warnings`.
4. Update `backend/crates/api-core/src/middleware/mod.rs:12,16` to explicitly enumerate exports (drop glob).
5. Add a regression test in `backend/crates/api-core/tests/middleware.rs` (or an existing test file) that constructs a router with `require_rls_context` layered and asserts the RLS GUC is NOT visible on a subsequently-borrowed pool connection — pins the "middleware does not deliver isolation" fact in code so the doc claim can never drift again without CI catching it.
6. Run `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p api-core -p db`.
7. Update `docs/repo-map.md` if it points at the deprecated symbols.

## Alternatives considered
- **Fix the middleware to hold a dedicated connection through the handler** — rejected because the `RlsConnection` extractor already does this correctly; two competing patterns increase footgun surface. The middleware pattern also can't force the handler to use the same connection without changing the whole request lifecycle.
- **Leave as-is and only remove the docstring's "MUST" claim** — rejected because the symbol name (`require_*`) still reads authoritative; a future author skims the name, not the docstring, and adopts a fail-open guard.

## Root-cause trace
1. Symptom: A route that wires `require_rls_context` as its sole tenant guard leaks across tenants — RLS GUCs are set on a pool connection immediately returned, not on the handler's borrowed connection.
2. ← `require_rls_context` at `rls_context.rs:232` calls `set_request_context(pool.as_ref(), ...)` — pool executor variant.
3. ← `set_request_context<E: Executor>` at `db/src/tenant_context.rs:18` accepts any executor; the pool impl transiently checks a connection in/out for the `SET`.
4. ← Handler later borrows a different connection via extractors / repo helpers; that connection has no `SET`, so RLS falls back to the default (typically no-tenant) — cross-tenant read/write becomes possible.
5. Origin: introduced when the pool-based middleware pattern predated the `RlsConnection` extractor; refactor left the deprecated code exported instead of removing it.

## Test plan
- [ ] `backend/crates/api-core/tests/rls_context_deprecated.rs` — asserts calling `require_rls_context` on an Axum test router does NOT persist the RLS GUC on a subsequent pool checkout (or, if we take the deletion path, that the symbol no longer exists — one `cargo check` failure test via `trybuild`).
- [ ] `backend/crates/db/tests/tenant_context_scope.rs` — regression: `set_request_context(pool, ...)` then `pool.acquire()` → `current_setting('app.tenant_id', true)` returns NULL, pinning the connection-scope invariant.
- [ ] Command: `cd backend && cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p api-core -p db`

## Out of scope
- Any route migration to `RlsConnection` — no route currently wires the deprecated middleware, so there is nothing to migrate.
- Changes to `set_request_context` itself — it is the correct primitive; only its abuse (pool executor variant used for middleware) is the defect here.

## After-merge
- Move this file to `plans/_archive/code-review-api-core-rls-context-pool-not-conn.md`
- Mark the matching `backlog.json` row as `status: "done"`
