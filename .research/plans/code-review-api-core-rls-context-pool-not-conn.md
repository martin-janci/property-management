# code-review-api-core-rls-context-pool-not-conn

**Vector:** security
**Score:** 3
**Source:** commit d20fd9e (api-core middleware `rls_context.rs`) — tier1d review 2026-08-09
**Confidence:** medium

## Hypothesis
`api_core::middleware::rls_context::require_rls_context` still ships as callable API-server middleware and its own docstring claims it is "a stricter version … Use this for routes that MUST have tenant isolation". It doesn't. It calls `db::tenant_context::set_request_context(pool.as_ref(), …)`, which issues its `SET`/`set_config` against a transiently-checked-out pool connection that is returned immediately; the handler then acquires a *different* pooled connection with no RLS GUCs set. The module header already diagnoses this ("Setting RLS on the pool doesn't affect individual connections"), but there is no `#[deprecated]` attribute, and the whole module is glob re-exported via `pub use rls_context::*;` in `middleware/mod.rs:16`, so a future route author is one plausible-looking guard away from a silent cross-tenant data leak. Remove the trap: delete the module (correct path is the `RlsConnection` extractor) or, if we want a soft ramp, mark every item `#[deprecated]` and drop the glob re-export so the names cannot be used without acknowledging the deprecation.

## Evidence
- `backend/crates/api-core/src/middleware/rls_context.rs:1-37` — module header explicitly marks the file DEPRECATED and lists the exact defect: "Setting RLS on the pool doesn't affect individual connections. PostgreSQL session settings are connection-scoped, not pool-scoped."
- `backend/crates/api-core/src/middleware/rls_context.rs:180-245` — `require_rls_context` docstring says "Use this for routes that MUST have tenant isolation" and body calls `db::tenant_context::set_request_context(pool.as_ref(), Some(tenant_id), Some(user_id), is_super_admin)` at line 232, then `next.run(request)` at line 245 with no per-connection scoping.
- `backend/crates/db/src/tenant_context.rs:18` — `set_request_context<E: Executor>` sends the `SET`/`set_config` against whatever executor it is given; with `&DbPool` this is a transient pool acquisition.
- `backend/crates/api-core/src/middleware/mod.rs:16` — `pub use rls_context::*;` glob-re-exports every deprecated helper unfiltered, so callers can bind the names via `api_core::middleware::require_rls_context` with no compile-time warning.
- `backend/crates/api-core/src/extractors/rls_connection.rs` — the correct path: `RlsConnection` acquires a dedicated conn from the pool and sets the RLS GUCs on THAT conn before the handler runs.
- Grep `require_rls_context\|rls_context_middleware\|set_rls_context` across `backend/servers/` returns zero call sites today, so severity is a latent trap, not a live bypass — a future author following the module docstring is the failure mode.

## Files
- `backend/crates/api-core/src/middleware/rls_context.rs`
- `backend/crates/api-core/src/middleware/mod.rs:16`
- `backend/crates/api-core/src/extractors/rls_connection.rs`
- `backend/crates/db/src/tenant_context.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (security regression, understand who else touches these names)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Mode:** cloud-ok (`SKIP_NETWORK=1 ./.claude/skills/verify-all.sh --quick` in an ephemeral cloud workspace)
Mode: cloud-ok

## Repro steps
1. `grep -rn "require_rls_context\|rls_context_middleware\|set_rls_context" backend/servers/ backend/crates/api-core/src/` — confirm zero live call sites today (latent trap, not a live regression), so the "test" is a deprecation guard rather than a behavioural test.
2. Write a `#[cfg(test)] compile_fail` doctest (or `trybuild` case) that mounts `require_rls_context` on a route and asserts a deprecation warning is emitted. Confirm it currently passes silently (no warning) — that is the failure the fix should flip.
3. Expected after fix: the same import produces `warning: use of deprecated function 'require_rls_context'` (or a hard `compile_error!` if we choose deletion), and the middleware is no longer glob-reachable via `api_core::middleware::require_rls_context`.

## Suggested approach
1. Prefer **delete** over deprecate: remove `backend/crates/api-core/src/middleware/rls_context.rs` in its entirety, and drop the `pub mod rls_context;` / `pub use rls_context::*;` lines in `backend/crates/api-core/src/middleware/mod.rs`. Zero live call sites (verified in step 1 of Repro) means no consumer churn.
2. If a policy demands a soft ramp instead of deletion, mark every free function *and* the module itself `#[deprecated(since = "…", note = "Use api_core::extractors::RlsConnection — RLS GUCs must be set on the same connection that runs the query, which this middleware cannot guarantee.")]`, and switch `middleware/mod.rs:16` from `pub use rls_context::*;` to an empty line (no re-export) so callers must import the deprecated names via the fully-qualified path and see the warning at every use.
3. Add a `deny(deprecated)` (crate-local) block for `api_core::middleware` that fires on any use of the deprecated names, so a follow-up author cannot silence the warning without noticing.
4. Update the module tests: if we deleted, delete the `#[cfg(test)] mod tests` block; if we soft-deprecated, gate the tests with `#[allow(deprecated)]` and add one new test asserting the deprecation warning fires (via `trybuild`).
5. Land the deletion (or deprecation) with a docstring pointer at the top of `middleware/mod.rs` naming `RlsConnection` as the correct path, so a future grep from a curious author lands on the working API.
6. Verify: `cargo clippy -p api-core --all-targets -- -D warnings` and `cargo test -p api-core` — both must stay green.
7. After merge: sweep for any dispatcher / research plan referencing `require_rls_context` as a name and update the guidance.

## Alternatives considered
- **Leave in place, add a runtime `panic!` if `require_rls_context` is ever called** — rejected because the trap is *compile-time-shaped* (a docstring lies about isolation); waiting for a production panic to teach the lesson is exactly the outcome we're trying to avoid.
- **Rename to `require_rls_context_UNSAFE_DEPRECATED_DO_NOT_USE`** — rejected because renaming without a `#[deprecated]` attribute or removal still lets a future author use the new name (or restore the old one) and the API surface still exists; the mechanism, not the label, is what leaks tenants.

## Root-cause trace
1. Symptom: `require_rls_context` docstring claims it enforces tenant isolation; it does not — its `SET` runs on a transient pool connection that is returned before the handler acquires its own connection.
2. ← Immediate cause at `backend/crates/api-core/src/middleware/rls_context.rs:232` — `set_request_context(pool.as_ref(), …)` uses the pool as executor, which cannot pin the session state to the handler's connection.
3. ← Upstream cause at `backend/crates/db/src/tenant_context.rs:18` — `set_request_context<E: Executor>` accepts any executor including `&DbPool`, which will silently do the wrong thing when the caller passes a pool.
4. Origin: the RLS-context middleware pattern predates the `RlsConnection` extractor introduced in `backend/crates/api-core/src/extractors/rls_connection.rs`; when the extractor landed, the middleware module was marked DEPRECATED in prose (rls_context.rs:1-37) but the API was never removed and never received `#[deprecated]`.

## Test plan
- [ ] Add a `trybuild` (or plain `compile_fail`) case under `backend/crates/api-core/tests/` that references `api_core::middleware::require_rls_context` and asserts either a deprecation warning (soft-ramp variant) or a compile error (deletion variant).
- [ ] Existing suite `cargo test -p api-core` stays green.
- [ ] Command: `cd backend && cargo clippy -p api-core --all-targets -- -D warnings && cargo test -p api-core`

## Out of scope
- Wider audit of every current middleware for pool-vs-conn RLS bugs — this plan targets the specific documented-but-broken helper.
- Migrating any handler to `RlsConnection` — no live call sites exist; migration guidance lives in the extractor's own docs.
- Renaming `db::tenant_context::set_request_context` to reject `&DbPool` executors — worth a follow-up but out of this diff.

## After-merge
- Move this file to `plans/_archive/code-review-api-core-rls-context-pool-not-conn.md`
- Mark the matching `backlog.json` row as `status: "done"`
- Cross-link the sibling backlog item `code-review-api-core-rls-context-deprecated-exported` — this plan supersedes it; drop that row with a "resolved: superseded by rls-context-pool-not-conn" evidence line.
