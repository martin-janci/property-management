# code-review-api-core-rls-context-pool-leak

**Vector:** refactor
**Score:** 3
**Source:** code-review-finding (Tier-1d api-core 2026-10-14)
**Confidence:** medium

## Hypothesis
`api-core` still exports a deprecated, pool-scoped RLS middleware (`middleware::rls_context::{rls_context_middleware, require_rls_context}`) via a `pub use rls_context::*;` re-export. The file self-documents as "fundamentally flawed": `set_request_context` is called on the shared pool rather than a dedicated connection, so RLS session settings can leak across pooled connections (cross-tenant data exposure). A grep shows the middleware is wired into no router today — but keeping it on the public API of `api-core` is a foot-gun: a future handler author can `use api_core::middleware::rls_context_middleware;` and silently re-adopt the flawed path. Fix: delete the module (or make it crate-private and remove the broad re-export) so the only RLS path exposed is the correct `RlsConnection` extractor.

## Evidence
- `backend/crates/api-core/src/middleware/rls_context.rs:3` — module header self-documents as DEPRECATED and "fundamentally flawed"
- `backend/crates/api-core/src/middleware/rls_context.rs:110` — `set_request_context(pool.as_ref(), …)` sets tenant context on the shared pool
- `backend/crates/api-core/src/middleware/mod.rs:16` — `pub use rls_context::*;` re-exports the flawed middleware on the public api-core surface
- Grep confirms no router under `backend/servers/{api-server,reality-server}/src/` references `rls_context_middleware` or `require_rls_context` — dead code
- Correct replacement is already in use elsewhere: `api_core::db::rls::RlsConnection` extractor pins the context to one acquired connection

## Files
- `backend/crates/api-core/src/middleware/rls_context.rs`
- `backend/crates/api-core/src/middleware/mod.rs:16`

## Required capabilities
- [ ] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. `rg -n '\\brls_context_middleware\\b|\\brequire_rls_context\\b' backend/ frontend/` returns only self-references inside `rls_context.rs` and the `pub use` in `middleware/mod.rs` — zero call sites.
2. Author a one-line test crate (or just `cargo expand -p api-core`) importing `api_core::middleware::rls_context_middleware`; it compiles successfully today, demonstrating the symbol remains publicly reachable from any downstream handler.
3. Expected: `api-core` exposes only the `RlsConnection`-based RLS path. Actual: the deprecated pool-scoped middleware is still re-exported.

## Suggested approach
1. Confirm no out-of-tree consumer imports `rls_context_middleware` / `require_rls_context` by running `rg -n '\\brls_context_middleware\\b|\\brequire_rls_context\\b' backend/` and `rg -n 'use api_core::middleware::rls_context' backend/`.
2. Delete `backend/crates/api-core/src/middleware/rls_context.rs` outright. If any helper in it is still useful (unlikely — the file is explicitly deprecated), move the single useful item into `middleware/mod.rs` as a `pub(crate)` symbol.
3. In `backend/crates/api-core/src/middleware/mod.rs`, remove line 16 (`pub use rls_context::*;`) and any `mod rls_context;` declaration pointing at the deleted file.
4. Run `cargo check -p api-core` to confirm nothing else in the crate referenced the deleted module. Run `cargo check --workspace --all-targets` to confirm no server/handler used the removed public symbols.
5. Run `cargo clippy -p api-core -- -D warnings` and `cargo test -p api-core` to catch any leftover referential breakage.
6. Add a regression test in `backend/crates/api-core/tests/public_api_rls.rs` that fails to compile if `rls_context_middleware` or `require_rls_context` is re-added to `api_core::middleware::*` (use `trybuild` or a compile-fail doc-test).

## Alternatives considered
- **Keep the file but downgrade the re-export to `pub(crate)`** — rejected because the module is already documented as fundamentally flawed and no in-crate caller uses it either; keeping dead code invites accidental resurrection and churns reviewers.
- **Add a `#[deprecated]` attribute and leave the code in place** — rejected because `#[deprecated]` only produces a warning, not a compile error; a security-sensitive foot-gun warrants deletion, not a lint.

## Root-cause trace
1. Symptom: `api-core` still ships a publicly-exported RLS middleware whose own doc comment says it leaks tenant context across pooled connections.
2. ← `middleware/mod.rs:16` broad `pub use rls_context::*;` re-export keeps every item in the deprecated module reachable.
3. ← `middleware/rls_context.rs:110` sets tenant context on the shared pool rather than a pinned connection — the original design flaw.
4. Origin: the module predates the `RlsConnection` extractor; when the correct path replaced it, the deprecated file was annotated rather than deleted and the re-export was never pruned.

## Test plan
- [ ] `backend/crates/api-core/tests/public_api_rls.rs` — compile-fail (trybuild) test asserting `api_core::middleware::rls_context_middleware` and `api_core::middleware::require_rls_context` are not reachable.
- [ ] Regression: `cargo check --workspace --all-targets` is green after deletion (no transitive dependency on the removed module).
- [ ] Local run: `cd backend && cargo check -p api-core && cargo clippy -p api-core -- -D warnings && cargo test -p api-core`

## Out of scope
- Auditing every other `api-core` middleware for similar "deprecated but exported" patterns. Keep this plan tight to the one named module.
- Introducing a new RLS helper. The correct replacement (`RlsConnection`) already exists and is in use.

## After-merge
- Move this file to `plans/_archive/code-review-api-core-rls-context-pool-leak.md`
- Mark the matching `backlog.json` row as `status: "done"`
