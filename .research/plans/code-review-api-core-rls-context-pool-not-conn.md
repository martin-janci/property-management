# code-review-api-core-rls-context-pool-not-conn

**Vector:** bug
**Score:** 3
**Source:** tier1d review `.research/signals/2026-08-09-api-core-tier1d-2.json`
**Confidence:** medium

## Hypothesis
`api-core::middleware::rls_context::require_rls_context` is documented as a stricter tenant-isolation guard, but the RLS `set_request_context` call it issues runs against a transient pooled connection that is returned to the pool before the handler acquires its own connection — so no RLS GUCs are actually in effect for the handler's queries. The module's own header already flags this as deprecated, yet the whole module is still glob re-exported and the misleading docstring remains. A future route author who mounts this middleware trusting the "MUST have tenant isolation" contract gets a compiling guard that silently leaks across tenants. The smallest fix is to (a) stop re-exporting the deprecated symbols, (b) annotate them `#[deprecated]` with an explicit pointer to the correct `RlsConnection` extractor path, and (c) add a compile-time test that catches any future call site.

## Evidence
- `backend/crates/api-core/src/middleware/rls_context.rs:1-37` — module header explicitly says "Setting RLS on the pool doesn't affect individual connections" and marks the whole approach deprecated
- `backend/crates/api-core/src/middleware/rls_context.rs:185-251` — `require_rls_context`'s docstring (lines 181-184) still claims it enforces tenant isolation
- `backend/crates/db/src/tenant_context.rs:18` — `set_request_context<E: Executor>` runs its `set_config` on the executor-provided connection and returns it; when called with `pool.as_ref()` the connection is released immediately and the next `pool.acquire()` gets a fresh connection with no session GUCs
- `backend/crates/api-core/src/middleware/mod.rs:16` — `pub use rls_context::*;` re-exports the broken functions to consumers
- Server grep shows no live mount today, so severity is a LATENT bypass (confidence medium, not high) — but the API surface is live-ready for a future author to trip on

## Files
- `backend/crates/api-core/src/middleware/rls_context.rs`
- `backend/crates/api-core/src/middleware/mod.rs`
- `backend/crates/db/src/tenant_context.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug vector)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Grep the workspace: `grep -rn "require_rls_context\|rls_context_middleware" backend/servers/` — confirm zero call sites today. This is a LATENT bypass, so there is no runtime symptom to reproduce; the failing-on-main test asserts the contract-violation directly.
2. Write an integration test in `backend/crates/api-core/tests/rls_context_deprecation.rs` that mounts a toy Axum route through `require_rls_context`, issues two requests as different tenants against a real Postgres pool with an RLS policy on a test table, and asserts that each request only sees its own tenant's rows. On main today, both requests either see all rows (RLS not applied to the handler's connection) or panic because no GUC is set — either outcome is the failing assertion.
3. Expected after the fix: the test is either removed (module deleted outright) or the module is `#[deprecated]`-annotated and `#[compile_error]`-gated at the re-export site so the test file simply cannot exist. Either terminal state proves the contract is unavailable to mislead future authors.

## Suggested approach
1. Verify the "no live call sites" claim with a fresh grep across both servers: `grep -rn "require_rls_context\|rls_context_middleware\|RlsContext\b" backend/servers/ backend/crates/`. If ANY live call site exists, expand the plan — this stops being a docs/re-export fix and becomes a "migrate the call site to `RlsConnection`" refactor first.
2. Remove the glob re-export in `backend/crates/api-core/src/middleware/mod.rs:16` and replace with an explicit list of the symbols that are still needed elsewhere in the crate (probably just `RlsContext` if it's used by extractors — audit before removing).
3. Add `#[deprecated(since = "…", note = "Broken by design — RLS GUCs set on a pooled connection are lost on release. Use `api_core::extractors::RlsConnection` instead. See rls_context.rs module header.")]` on the three public functions in `rls_context.rs` (`rls_context_middleware`, `require_rls_context`, and any other pub `fn` that touches `set_request_context`).
4. Delete the docstring on `require_rls_context` (`rls_context.rs:181-184`) that says "Use this for routes that MUST have tenant isolation" and replace it with a one-line pointer to `RlsConnection`.
5. Land the integration test from *Repro steps* — it fails on main today (the guard leaks) and passes after the fix (compile-error / deprecation warning blocks any new call site; the runtime behavior of the deprecated functions is unchanged, so the assertion is really "the API surface is no longer usable without a loud opt-in").
6. Update `docs/api/README.md` (or the closest equivalent operator-facing doc) if `require_rls_context` is mentioned anywhere as a recommended pattern — grep first.

## Alternatives considered
- **Delete `rls_context.rs` entirely** — rejected because the module contains `RlsContext` as a struct that may be referenced by extractors elsewhere; a wholesale delete risks a compile break in a wider blast radius than the docs/re-export patch. Do this in a follow-up once #[deprecated] warnings surface all consumers.
- **Add a runtime panic to `require_rls_context`** — rejected because a runtime panic would be a footgun (a future dev mounts it in staging, ships to prod, panics on first request); compile-time deprecation is louder and safer than a delayed crash.

## Root-cause trace
1. Symptom: `require_rls_context` returns a handler-wrapper that carries no active RLS GUCs into the handler's SQL calls — cross-tenant queries succeed unfiltered.
2. ← `require_rls_context` at `backend/crates/api-core/src/middleware/rls_context.rs:232` calls `set_request_context(pool.as_ref(), Some(tenant_id), Some(user_id), is_super_admin)` and then invokes `next.run(request)` at `:245` without holding onto the connection.
3. ← `set_request_context<E: Executor>` at `backend/crates/db/src/tenant_context.rs:18` accepts a pool as an executor, checks out a connection, runs `SET LOCAL rls.tenant_id = …`, and returns; the pool immediately reclaims the connection and its session-scoped GUCs.
4. Origin: the middleware was authored before the team learned that PostgreSQL session settings are connection-scoped, not pool-scoped. The module header (lines 1-37) captures that discovery but the docstring on `require_rls_context` was never revised, and the `pub use` in `middleware/mod.rs` was never narrowed.

## Test plan
- [ ] `backend/crates/api-core/tests/rls_context_deprecation.rs` — new integration test that fails on main (asserts a cross-tenant leak through `require_rls_context` against a real RLS-enabled Postgres) and passes after the fix (either the module is gone or the test now compiles against the `#[deprecated]` surface with `#![deny(deprecated)]` and refuses to build)
- [ ] Regression: keep the existing `RlsConnection`-extractor tests green — the fix must not touch any working RLS path
- [ ] Exact command: `cd backend && cargo test -p api-core rls_context_deprecation`

## Out of scope
- Migrating any actual route that IS using the deprecated middleware today (grep says none; if that changes, spin a separate plan).
- Refactoring `RlsConnection` itself.
- Adding a broader lint pass across the middleware crate.

## After-merge
- Move this file to `plans/_archive/code-review-api-core-rls-context-pool-not-conn.md`
- Mark the matching `backlog.json` row as `status: "done"`
