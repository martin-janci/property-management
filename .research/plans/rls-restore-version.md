# rls-restore-version

**Vector:** security
**Score:** 3
**Source:** PR #421 | Issue #160
**Confidence:** high

## Hypothesis
The `POST /documents/{id}/versions/{version_id}/restore` handler still performs its
database work through the deprecated, non-RLS `document_repo.restore_version`, even
though the handler already holds an `RlsConnection`. Because that repo method runs
its `find_by_id` / `create_version` queries on the shared pool (no `SET LOCAL`
tenant context), row-level security is not enforced for the restore write — the
tenant isolation relies entirely on the handler's manual checks. Migrating the call
to a new `restore_version_rls` composed from the existing `find_by_id_rls` and
`create_version_rls` primitives closes the gap with a small, pattern-matching change
identical to the one PR #421 applied to the pricing/reports handlers.

## Evidence
- `backend/servers/api-server/src/routes/documents.rs:1631` — `// TODO: Migrate restore_version to RLS pattern` directly above `#[allow(deprecated)] state.document_repo.restore_version(id, version_id, user_id)`
- `backend/crates/db/src/repositories/document.rs:1873` — deprecation note literally says `Use restore_version_rls`, but no such method exists (only the deprecated `restore_version` at line 1879)
- `backend/crates/db/src/repositories/document.rs:547` (`find_by_id_rls`) and `:1505` (`create_version_rls`) — the RLS primitives needed to compose the variant already exist
- The handler signature already binds `mut rls: RlsConnection` and calls `rls.release()` in its Ok branch, so the connection is available but unused for the restore write
- PR #421 migrated `market_pricing.rs` / `notification_preferences.rs` / `reports.rs` with the same `repo.method_rls(&mut **rls.conn(), ...)` shape — a direct precedent to copy

## Files
- `backend/servers/api-server/src/routes/documents.rs:1631`
- `backend/crates/db/src/repositories/document.rs:1879`
- `backend/crates/db/tests/rls_penetration_tests.rs`

## Required capabilities
- [ ] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [x] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):** no C4/C5 → cloud-ok. The change is
backend Rust + a `cargo test` against Postgres, runnable via the `ppt-bridge` MCP.

Mode: cloud-ok

## Repro steps
1. Bring up the dev stack with Postgres (RLS policies applied): `ppt_dev_up` (or `stack up pm-local`).
2. Seed two organizations A and B, each with a document that has at least two versions.
3. Author an integration test (mirroring `rls_penetration_tests.rs`) that opens an RLS connection scoped to org A's tenant context and calls the org-B document's restore path through the RLS-aware code.
4. Expected after fix: the restore is blocked by RLS (no row visible → `RowNotFound`). Actual on `main`: the deprecated `restore_version` runs on the pool with no tenant context, so RLS does not gate the underlying reads/writes for that call.

## Suggested approach
1. In `backend/crates/db/src/repositories/document.rs`, add `restore_version_rls<'e, E: Executor<'e, Database = Postgres>>(&self, executor: E, document_id, version_id, restored_by)` directly beside the deprecated `restore_version` (line 1879), composing it from `find_by_id_rls` (line 547) and `create_version_rls` (line 1505) so every query runs on the passed executor.
2. Preserve the existing root-document-chain validation (`root_document_id()` equality check) from the deprecated method verbatim — it is correctness logic, not RLS plumbing.
3. In `backend/servers/api-server/src/routes/documents.rs:1631`, replace the `#[allow(deprecated)] state.document_repo.restore_version(...)` call with `state.document_repo.restore_version_rls(&mut **rls.conn(), id, version_id, user_id)`, and delete the `// TODO: Migrate restore_version to RLS pattern` comment and the `#[allow(deprecated)]` attribute.
4. Keep the existing `rls.release().await` placement (Ok branch) and add a `rls.release().await` before any new early-return error path the migration introduces, matching the pattern PR #421 used in `reports.rs`.
5. Leave the deprecated `restore_version` method in place only if other callers remain; otherwise remove it and its `#[deprecated]`/`#[allow(deprecated)]` attributes once `git grep restore_version` shows the handler was the sole caller.
6. Add the regression test from *Repro steps* to `backend/crates/db/tests/rls_penetration_tests.rs` (or a document-specific sibling) asserting a cross-tenant restore is not visible under RLS.
7. Run `cargo test -p db --test rls_penetration_tests` and `cargo build -p api-server` to confirm green.

## Alternatives considered
- **Leave the manual creator/manager check as the only guard** — rejected because it duplicates the authorization concern in handler code and diverges from the RLS-everywhere invariant the rest of the document routes now follow (PR #421), leaving one inconsistent write path that the next refactor can silently break.
- **Wrap the existing deprecated method in a transaction on the RLS connection without composing the `_rls` primitives** — rejected because the deprecated method captures `&self.pool` internally, so it cannot run on a borrowed executor; threading the executor in is exactly what `restore_version_rls` does, and the primitives already exist.

## Root-cause trace
1. Symptom: a document version restore executes its reads/writes without a `SET LOCAL` tenant context, so RLS policies do not gate the operation.
2. ← immediate cause at `backend/servers/api-server/src/routes/documents.rs:1631` — handler calls `document_repo.restore_version` (deprecated, pool-bound) instead of an RLS variant despite holding `rls: RlsConnection`.
3. ← upstream cause at `backend/crates/db/src/repositories/document.rs:1879` — `restore_version` runs `find_by_id` / `create_version` on `&self.pool`; the promised `restore_version_rls` (named in the line-1873 deprecation note) was never written.
4. Origin: the bulk RLS migration deprecated the pool-bound document methods (since = "0.2.276") but left `restore_version` as an unfinished tail; PR #421 (2026-05-22) continued the migration for other routes without reaching this one.

## Test plan
- [ ] `backend/crates/db/tests/rls_penetration_tests.rs` — new `test_cross_tenant_document_restore_isolation` asserting org A cannot restore org B's document version under an org-A RLS context
- [ ] Regression: restoring a version of one's *own* document still succeeds and returns the new version number (happy path unchanged)
- [ ] `cargo test -p db --test rls_penetration_tests -- --ignored` (the penetration suite is `#[ignore]`d; run serially per Issue #375 guidance: `--test-threads=1`)

## Out of scope
- The `budgets.rs` dashboard/reserve-transaction and `lease_abstraction.rs` migrations (tracked separately as `security-rls-migration-remaining` in `backlog.json`).
- Any change to the deliberately non-RLS public/pre-auth endpoints (document share-by-token, subscription_plans, login 2FA lookups) — those are correctly annotated.
- Making `security-tests` a required CI check (Issue #375 follow-up).

## After-merge
- Move this file to `plans/_archive/rls-restore-version.md`
- Mark the matching `backlog.json` row as `status: "done"`
