# code-review-api-core-rls-context-pool-not-conn

**Vector:** bug
**Score:** 3
**Source:** tier1d review 2026-08-09 (api-core), signal `code-review-api-core-rls-context-pool-not-conn`
**Confidence:** medium

## Hypothesis
`api-core::middleware::rls_context::require_rls_context` is documented as "a stricter version … Use this for routes that MUST have tenant isolation," but it enforces nothing at runtime: `set_request_context` runs on a connection sqlx transiently checks out of the pool and immediately returns; the handler later acquires a different pooled connection where the RLS GUCs are unset. The trap is compounded by `pub use rls_context::*;` in `middleware/mod.rs`, which re-exports the broken guard to consumers. The smallest safe change is to stop re-exporting the deprecated module and add `#[deprecated(note = "…")]` to `require_rls_context` / `rls_context_middleware` so any future adoption becomes a hard compile-time warning; the real path is the `RlsConnection` extractor which sets RLS on the same dedicated connection it hands the handler.

## Evidence
- `backend/crates/api-core/src/middleware/rls_context.rs:1-37` — module header already diagnoses the defect ("Setting RLS on the pool doesn't affect individual connections") and marks the module `# DEPRECATED`, yet no `#[deprecated]` attribute is emitted.
- `backend/crates/api-core/src/middleware/rls_context.rs:185-251` — `require_rls_context` calls `db::tenant_context::set_request_context(pool.as_ref(), Some(tenant_id), Some(user_id), is_super_admin)` then `next.run(request).await`; RLS is not applied to the handler's queries.
- `backend/crates/db/src/tenant_context.rs:18` — `set_request_context<E: Executor>` issues its `set_config`/SET on a connection sqlx transiently checks out of the pool and returns it; per-connection scope of PG session settings makes pool-scope routing incorrect by construction.
- `backend/crates/api-core/src/middleware/mod.rs:16` — `pub use rls_context::*;` re-exports the broken guards through the crate's public surface, undermining the DEPRECATED intent.
- `grep -r require_rls_context backend/servers` returns zero call sites — LATENT bypass rather than a live one, hence confidence medium (matches the source signal).

## Files
- `backend/crates/api-core/src/middleware/rls_context.rs`
- `backend/crates/api-core/src/middleware/mod.rs`
- `backend/crates/api-core/src/extractors/rls_connection.rs`
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

**Execution mode (auto-derived from the ticks):**

Mode: cloud-ok

## Repro steps
1. `grep -n "require_rls_context\|rls_context_middleware" backend/crates/api-core/src/middleware/mod.rs backend/crates/api-core/src/middleware/rls_context.rs` — confirm both symbols are re-exported without any `#[deprecated]` attribute on the definitions.
2. Read `backend/crates/api-core/src/middleware/rls_context.rs:185-251` and `backend/crates/db/src/tenant_context.rs:18-40` back to back — observe that the middleware issues its RLS `set_config` on a pool-checked-out connection that is dropped before the handler acquires its own. Expected: RLS applied for the handler's queries; actual: RLS applied then discarded, then the handler runs on a different pooled connection with no RLS GUC set.

## Suggested approach
1. Add `#[deprecated(since = "…", note = "Use the RlsConnection extractor in api-core::extractors::rls_connection — the pool-level set_request_context is a no-op for handler queries")]` to both `require_rls_context` (rls_context.rs:185) and `rls_context_middleware` (same file).
2. Replace `pub use rls_context::*;` in `backend/crates/api-core/src/middleware/mod.rs:16` with a narrow re-export of only the type aliases the deprecated module still carries — do NOT re-export the two `#[deprecated]` functions through the crate's public surface. If nothing non-deprecated remains, remove the `pub use` line entirely and leave the module private.
3. Extend the file-header doc block on `rls_context.rs` to point at `extractors::rls_connection::RlsConnection` as the canonical path, with a one-line snippet showing `mut rls: RlsConnection` in a handler signature.
4. Run `cargo clippy -p api-core -- -D warnings -D deprecated` to prove no in-tree caller trips the new deprecation (grep already confirms zero call sites, but the compiler is the source of truth).
5. Add a `#[deny(deprecated)]` compile-fail test under `backend/crates/api-core/tests/` that attempts to call `require_rls_context` and asserts it fails to build — the failing-on-`main` test the implementer needs for IG3.
6. Do NOT delete the module in this plan — deletion is a follow-up once the deprecation has landed and any downstream branches have rebased.

## Alternatives considered
- **Delete `rls_context.rs` entirely in this PR** — rejected because there is no visible caller today, but keeping the file with a `#[deprecated]` attribute + narrowed re-export gives one release cycle of a compile-time warning for any branch that adopted the guard between routine runs, and the extra churn (removing tests, retiring the file) belongs in a separate follow-up plan.
- **Fix `set_request_context` to bind the connection through the middleware into a request extension** — rejected because that is what the `RlsConnection` extractor already does correctly; duplicating the wiring in the middleware layer would create two parallel RLS mechanisms and re-introduce the fragility (the middleware would still have to hand its exact connection to every downstream handler by convention, not by type). The extractor path is the invariant that survives.

## Root-cause trace
1. Symptom: A future route author follows the `require_rls_context` docstring, gets a compiling, plausible-looking guard, and ships a cross-tenant data-exposure hole.
2. ← `require_rls_context` at `backend/crates/api-core/src/middleware/rls_context.rs:185-251` invokes `set_request_context(pool.as_ref(), …)` then `next.run(request).await` — the RLS `set_config` runs on a transiently-checked-out pooled connection that is returned to the pool before the handler acquires its own connection.
3. ← `set_request_context<E: Executor>` at `backend/crates/db/src/tenant_context.rs:18` accepts any executor including `&PgPool`; PostgreSQL session settings (`SET LOCAL`/`set_config`) are per-connection, so setting them via a pool-checked-out connection has no effect on subsequent per-request checkouts.
4. ← `pub use rls_context::*;` at `backend/crates/api-core/src/middleware/mod.rs:16` re-exports the broken guards to consumers even though the module header marks the file DEPRECATED — the contradiction is the trap.
5. Origin: introduced together with the RLS middleware exploration before `RlsConnection` extractor became the canonical path (the module header's DEPRECATED marker was added post-hoc when the pool-scope defect was recognised).

## Test plan
- [ ] New compile-fail / deny(deprecated) test under `backend/crates/api-core/tests/rls_context_is_deprecated.rs` that references `api_core::middleware::require_rls_context` under `#![deny(deprecated)]` and expects a build failure — the failing-on-`main` regression test (IG3).
- [ ] `cargo clippy -p api-core -- -D warnings -D deprecated` — passes on the deprecating branch; would fail if any in-tree caller trips the attribute.
- [ ] `cargo test -p api-core` — the existing `api-core` unit suite must still pass.
- [ ] `grep -Rn "require_rls_context\|rls_context_middleware" backend/servers backend/crates` — after the change, the only hits must be inside `backend/crates/api-core/src/middleware/rls_context.rs` itself (definitions) plus the new compile-fail test's expected-error snapshot.

## Out of scope
- Deleting `rls_context.rs` — deferred until at least one release cycle after this deprecation lands.
- Any change to the `RlsConnection` extractor (`backend/crates/api-core/src/extractors/rls_connection.rs`) or how servers mount it — the extractor is the correct path today and stays untouched.
- Auditing every server route for RLS coverage — orthogonal work owned by the `security-rls-migration-residual` line of items.

## After-merge
- Move this file to `plans/_archive/code-review-api-core-rls-context-pool-not-conn.md`
- Mark the matching `backlog.json` row as `status: "done"`
