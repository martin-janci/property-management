# code-review-api-core-rls-context-pool-not-conn

**Vector:** security
**Score:** 3
**Source:** rotating-expert-review api-core segment 2026-08-09; tier1d-dispatcher-generator
**Confidence:** medium

## Hypothesis
`require_rls_context` in `backend/crates/api-core/src/middleware/rls_context.rs:185-251` is documented as the strict tenant-isolation middleware ("Use this for routes that MUST have tenant isolation"), but its `set_request_context` call runs `SELECT set_request_context(...)` on a transiently checked-out pool connection that is returned to the pool immediately — the handler then acquires a *different* pooled connection where the RLS GUCs are unset. The whole module already carries a DEPRECATED header noting this exact defect, yet the module is still glob re-exported (`pub use rls_context::*;` in `middleware/mod.rs:16`). Result: any future route author who follows the visible docstring gets a compiling, plausible-looking guard that silently leaks across tenants. Fix by deleting the deprecated middleware (or at minimum stop the glob re-export + add `#[deprecated]`) so the only supported path is the `RlsConnection` extractor, which sets RLS on the same dedicated connection it hands the handler.

## Evidence
- `backend/crates/api-core/src/middleware/rls_context.rs:1-37` — module header self-diagnoses the flaw ("Setting RLS on the pool doesn't affect individual connections"; "PostgreSQL session settings are connection-scoped, not pool-scoped") and marks the module DEPRECATED, yet the file itself carries no `#[deprecated]` attribute.
- `backend/crates/api-core/src/middleware/rls_context.rs:181-245` — `require_rls_context` docstring reads "This is a stricter version that requires both authentication AND tenant context. Use this for routes that MUST have tenant isolation." The trap: same file's header says do NOT use, function-level docstring says use for tenant isolation.
- `backend/crates/api-core/src/middleware/mod.rs:16` — `pub use rls_context::*;` glob re-exports `require_rls_context` and `rls_context_middleware` to the whole workspace.
- `backend/crates/db/src/tenant_context.rs:18` — `set_request_context<E: Executor>` runs its `SELECT set_request_context(...)` on the executor it is handed. When passed `pool.as_ref()` (as `require_rls_context` does at line 232), SQLx checks out a connection, issues the query, and returns the connection to the pool. The RLS GUCs stay pinned to that connection, not to the handler's later checkout.
- `backend/crates/api-core/src/extractors/rls_connection.rs:85+` — the correct path is already implemented: it acquires a dedicated connection, sets RLS on it, and hands the same `&mut PgConnection` to the handler.

## Files
- `backend/crates/api-core/src/middleware/rls_context.rs`
- `backend/crates/api-core/src/middleware/mod.rs`
- `backend/crates/db/src/tenant_context.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug with security implications)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. `grep -rn "require_rls_context\|rls_context_middleware" backend/servers/` → verify no route currently mounts either (baseline: 0 hits in servers/ crates).
2. Author a compile-time regression: add a test binary or `#[compile_error!]` in `middleware/mod.rs` that would break if `require_rls_context` is re-exported. Alternatively, a `#[deprecated(note="…")]` on both functions plus `#![deny(deprecated)]` scoped to the module's public tests.
3. Expected: after the fix, any workspace crate that references `api_core::middleware::require_rls_context` fails to compile with a deprecation error. Actual today: no such gate — the trap is available.

## Suggested approach
1. Add `#[deprecated(since = "…", note = "…")]` to both `pub async fn require_rls_context` (rls_context.rs:185) and `pub async fn rls_context_middleware` (rls_context.rs:70).
2. Remove the glob re-export at `backend/crates/api-core/src/middleware/mod.rs:16` — replace with `pub use rls_connection_middleware::…` (or nothing, if no consumer needs anything from that module).
3. Add a comment above the `require_rls_context` docstring warning that the function name is a lie retained only for BC and that all documented behavior lives in `crate::extractors::RlsConnection`.
4. Optional stronger step: delete `rls_context.rs` entirely if no consumer still imports either symbol (verify with `cargo check --workspace`).
5. Add a `#[test]` in `api-core` that asserts the correct path (`RlsConnection` extractor) sets RLS on the connection the handler receives — a small integration test that inserts a row under a tenant, extracts `RlsConnection`, and queries the row back.

## Alternatives considered
- **Fix the middleware so `set_request_context` runs on the same connection the handler will use** — rejected because Axum middleware receives `Request<Body>` and `Next`, not a `PgConnection`; there is no way for middleware to hand the handler the specific connection it initialized. The `RlsConnection` extractor is the correct axis and already exists — duplicating the guarantee inside a middleware would fragment the abstraction and re-create the trap.
- **Leave the module in place with only a docs update** — rejected because the module header *already* documents the flaw (has for a while) and consumers still get a compiling, exported symbol whose function-level docstring contradicts the module docstring. Docs alone do not prevent the trap; `#[deprecated]` + de-glob-exporting does.

## Root-cause trace
1. Symptom: a hypothetical route mounts `require_rls_context` as middleware and its handler queries the DB. Queries observe no tenant filter (RLS unset) → cross-tenant data exposure.
2. ← `require_rls_context` at `backend/crates/api-core/src/middleware/rls_context.rs:232` calls `set_request_context(pool.as_ref(), …)`.
3. ← `set_request_context<E: Executor>` at `backend/crates/db/src/tenant_context.rs:18` executes on a transiently-checked-out pool connection; that connection returns to the pool before the handler runs.
4. ← Handler acquires a *different* pooled connection (`RequestExt` / a manual `pool.acquire()`); PostgreSQL session GUCs on the first connection do not follow.
5. Origin: this is a pre-existing design that the module header itself already flags as deprecated. The `RlsConnection` extractor (`backend/crates/api-core/src/extractors/rls_connection.rs`) was added later as the correct path but the deprecated middleware was left visibly re-exported.

## Test plan
- [ ] Unit / compile: with `#[deprecated]` on both middleware functions and `#![deny(deprecated)]` in a small dedicated test crate (or `cargo check --workspace -- -W deprecated -D warnings` scoped narrowly), verify no workspace consumer references either symbol without triggering the deprecation.
- [ ] Regression: add an `api-core` integration test that mounts a route with the `RlsConnection` extractor, inserts a row for tenant A, then queries as tenant B via a second call — expect zero rows returned.
- [ ] Command: `cargo test -p api-core` and `cargo check --workspace`.

## Out of scope
- Rewriting existing routes to use `RlsConnection` — this plan only sets the trap-removing guardrails; a follow-up plan can migrate any remaining consumers of the deprecated middleware if they are ever found.
- Changing `set_request_context`'s API — the extractor already uses it correctly; middleware misuse is the defect.

## After-merge
- Move this file to `plans/_archive/code-review-api-core-rls-context-pool-not-conn.md`
- Mark the matching `backlog.json` row as `status: "done"`
