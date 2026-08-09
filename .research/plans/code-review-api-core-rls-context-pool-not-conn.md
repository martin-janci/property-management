# code-review-api-core-rls-context-pool-not-conn

**Vector:** bug
**Score:** 3
**Source:** Tier1d dev-review 2026-08-09 (segment `api-core`, backlog id `code-review-api-core-rls-context-pool-not-conn`)
**Confidence:** medium

## Hypothesis
`api-core::middleware::rls_context` exports `require_rls_context` with a docstring stating "Use this for routes that MUST have tenant isolation", but it sets the RLS `set_config` GUCs on a **pool-borrowed** connection that is dropped before the handler runs — the handler then acquires a **different** pooled connection where the GUCs are unset, so RLS is silently NOT applied. The module header already diagnoses this (marked deprecated in prose), yet the two broken functions plus `rls_context_middleware` are still glob re-exported via `pub use rls_context::*;` and are not `#[deprecated]`. The correct extractor `RlsConnection` (sets the GUCs on the same dedicated connection it hands the handler) exists at `extractors/rls_connection.rs`. Smallest safe change: mark the module `#[deprecated]`, stop glob-re-exporting its two broken middlewares, add a compile-time doctest / `compile_error!`-in-example asserting the trap, and leave a migration pointer to `RlsConnection` in the header. Zero live call sites today, so this is a latent-trap cleanup, not a live fix — but leaving a plausible-looking middleware named `require_rls_context` that provides no isolation is a footgun the next author will step on.

## Evidence
- `backend/crates/api-core/src/middleware/rls_context.rs:181-251` — `require_rls_context` docstring: "a stricter version ... Use this for routes that MUST have tenant isolation". Line 232 calls `db::tenant_context::set_request_context(pool.as_ref(), Some(tenant_id), ...)` on `pool.as_ref()` (a `&PgPool`), then line 245 `next.run(request).await`. The handler's own conn checkout is unrelated to the transient conn used for `set_config`.
- `backend/crates/db/src/tenant_context.rs:18` — `set_request_context<E: Executor>` accepts any executor; when handed a `&PgPool` the pool checks out a transient conn, issues the `set_config` SETs, and returns it. Connection-scoped, not pool-scoped.
- `backend/crates/api-core/src/middleware/rls_context.rs:1-37` — module header explicitly says "Setting RLS on the pool doesn't affect individual connections" and marks the module DEPRECATED in prose (without the `#[deprecated]` attribute).
- `backend/crates/api-core/src/middleware/mod.rs:7,16` — `pub mod rls_context;` and `pub use rls_context::*;` glob re-export both broken middlewares to consumers.
- `backend/crates/api-core/src/extractors/rls_connection.rs:85` — `RlsConnection` is the correct extractor: it sets the GUCs on the same dedicated `PgConnection` it yields to the handler.
- No live call site: `grep -rn "require_rls_context\|rls_context_middleware" backend/servers/` returns 0 hits. This is a latent trap.

## Files
- `backend/crates/api-core/src/middleware/rls_context.rs`
- `backend/crates/api-core/src/middleware/mod.rs`
- `backend/crates/api-core/src/extractors/rls_connection.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. `grep -rn "require_rls_context\|rls_context_middleware" backend/` — confirm the module + two functions are exported and unused (0 call sites under `backend/servers/`).
2. Add a temporary test that mounts `require_rls_context` on a router, runs a query in the handler, and asserts `SHOW app.current_tenant_id;` returns the empty default (proving the GUC was NOT set on the handler's conn). Expected: today the test passes (proves the trap); after the fix the module is gone / deprecated and the test compile-errors on `require_rls_context`.

## Suggested approach
1. Add `#[deprecated(since = "…", note = "use RlsConnection extractor — this middleware sets RLS on a transient pool conn, not the handler's")]` to `require_rls_context`, `rls_context_middleware`, and (if applicable) the module itself.
2. Replace `pub use rls_context::*;` in `middleware/mod.rs` with an explicit deny list — do NOT re-export `require_rls_context` / `rls_context_middleware`. Keep the module accessible as `rls_context::…` for the header's audit trail.
3. In `middleware/rls_context.rs` header, add a `## Migration` block pointing at `extractors::rls_connection::RlsConnection` with a 6-line "before / after" snippet.
4. Add a doctest (or a `#[should_panic]`-style compile-fail test in the crate's `trybuild` if one exists) demonstrating that the correct pattern is `RlsConnection` extractor + handler query.
5. Grep the docs/ tree and any generated OpenAPI notes for stale references to `require_rls_context` and correct them.
6. Confirm `cargo clippy --workspace --all-targets -- -D warnings` is clean (the `#[deprecated]` triggers `deprecated` warnings only when *used* — since nothing uses it, clippy stays green).

## Alternatives considered
- **Delete the module outright** — rejected because the header text is the primary artifact documenting *why* the naive "set RLS on pool" pattern is wrong; keeping it as `#[deprecated]` preserves the audit trail for the next author who's tempted to try the same shape.
- **Leave it as-is (prose deprecation only)** — rejected because a glob re-export + docstring saying "use this for routes that MUST have tenant isolation" is a live trap; the next route author will grep for `require_rls_context`, wire it up, and ship a silent tenant-isolation bypass.

## Root-cause trace
1. Symptom: `require_rls_context` middleware runs without error but the handler's queries do not see the tenant/user GUCs — RLS is not enforced.
2. ← `require_rls_context` (rls_context.rs:232) calls `set_request_context(pool.as_ref(), …)` — the `&PgPool` executor checks out a transient conn, issues `set_config`, returns the conn to the pool.
3. ← `next.run(request)` (rls_context.rs:245) — the downstream handler acquires an independent pooled conn (`PgPool::acquire`) with no GUCs set.
4. Origin: the module was introduced as a naive port of the request-context idea before the `RlsConnection` extractor existed; the header was updated to warn but the export was never revoked. Traceable to the earliest revision of `middleware/rls_context.rs` in `git log --follow`.

## Test plan
- [ ] `cargo test -p api-core --test rls_context_deprecation` — new test asserts the correct extractor path (`RlsConnection`) is documented and that the deprecated middleware, if invoked, is flagged by the compiler `deprecated` lint.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` remains green.
- [ ] `grep -rn "require_rls_context\|rls_context_middleware" backend/servers/` returns 0 hits (regression guard against future adoption).

## Out of scope
- Rewriting or renaming `RlsConnection` — it is the target, not the object under change.
- Auditing whether any existing router accidentally relies on the pool-set GUC pattern outside this middleware — separate audit if needed.

## After-merge
- Move this file to `plans/_archive/code-review-api-core-rls-context-pool-not-conn.md`
- Mark the matching `backlog.json` row as `status: "done"`
