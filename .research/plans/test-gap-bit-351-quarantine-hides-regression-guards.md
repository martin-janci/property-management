# test-gap-bit-351-quarantine-hides-regression-guards

**Vector:** test-gap
**Score:** 3
**Source:** Issue #1998, #1999, #2001 (post-merge follow-ups on PR #1993, #1992, #1983)
**Confidence:** high

## Hypothesis
Three separate post-merge review issues opened on 2026-07-01 all diagnose the same anti-pattern: recent fix PRs add a regression test but mark it `#[ignore = "BIT-351 quarantine ..."]`, so the fix ships with zero executing regression coverage. BIT-351 tracks a broken sqlx-test harness; BIT-352 tracks the repair. Until BIT-352 lands, every new backend regression test lands ignored by default and the shipped fix is unguarded. Rather than wait for BIT-352, we can add three non-DB SQL-string / string-shape assertion tests (one per just-shipped fix) that assert the fix's *query shape* directly on the constant string. These run in CI without a DB, so BIT-351 does not apply, and they catch a future regression that reverts the fix's SQL change. Same file layout as the header note already in `thread_participant_state_tests.rs` ("catalog-metadata guard style").

## Evidence
- Issue #1998 (2026-07-01) — `soft_deleted_thread_excluded_from_unread_count` in `backend/crates/db/tests/thread_participant_state_tests.rs` is `#[ignore]`d; PR #1993 fix (`AND tps.deleted_at IS NULL` on `count_unread_rls`) has no executing guard. Sketch already in the issue: `assert!(sql.contains("tps.deleted_at IS NULL"))`.
- Issue #1999 (2026-07-01) — `mark_favorite_alert_read_is_idempotent` in `backend/servers/reality-server/tests/favorite_alert_worker_tests.rs` is `#[ignore]`d; PR #1992's `FavoriteAlertReadOutcome::Flipped/AlreadyRead/NotFound` transition has no executing guard.
- Issue #2001 (2026-07-01) — `docs/endpoint-checklist/groups/finance.md` flips ~59 endpoint rows from `partial → done` on tests that remain `#[ignore]`d under BIT-417/BIT-440; README claims `finance 100%` but tally is partly illusory.
- `backend/crates/db/tests/thread_participant_state_tests.rs` header — already documents a "catalog-metadata guard style" convention for exactly this bypass (non-DB assertion tests as a fallback while the sqlx harness is broken).
- BIT-351 is unresolved; BIT-352 tracks the repair. No timeline on repair.

## Files
- `backend/crates/db/src/repositories/messaging.rs`
- `backend/crates/db/tests/thread_participant_state_tests.rs`
- `backend/servers/reality-server/src/services/favorite_alerts.rs`
- `backend/servers/reality-server/tests/favorite_alert_worker_tests.rs`
- `docs/endpoint-checklist/README.md`
- `docs/endpoint-checklist/groups/finance.md`

## Dependencies

## Required capabilities
- [ ] C1 — Systematic debugging (not a bug — this is coverage repair)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion

**Execution mode:**

Mode: cloud-ok

## Repro steps
1. Read the three post-merge review issues (#1998, #1999, #2001) and confirm each cited regression test carries the `#[ignore = "BIT-351 ..."]` marker.
2. Run `cargo test -p db --test thread_participant_state_tests` — the file's four `#[sqlx::test]`s are skipped; there is no non-ignored coverage of the `count_unread_rls` join predicate.
3. Search `docs/endpoint-checklist/groups/finance.md` for rows labelled `done` whose "cited test" is a suite that grep `-nE '#\[ignore.*BIT-(417|440)'` still matches.
4. Expected: at least one running guard per shipped fix. Actual: zero running guards, coverage claim overstated in `finance.md`.

## Suggested approach
1. Hoist the `count_unread_rls` SQL from `messaging.rs` into a `pub(crate) const COUNT_UNREAD_SQL: &str = "..."` (or expose a `sql_text()` fn returning it). Then in `thread_participant_state_tests.rs` add `#[test] fn count_unread_query_excludes_soft_deleted()` asserting `COUNT_UNREAD_SQL.contains("tps.deleted_at IS NULL")` and `.contains("LEFT JOIN thread_participant_state")`. No `#[ignore]`, no DB — runs in CI on the standard `cargo test` gate.
2. Same shape for `blocked_among_rls`: hoist the SQL, assert it contains the bidirectional `blocker/blocked` predicate + `= ANY($2)`.
3. Same shape for `FavoriteAlertReadOutcome`: extract the mapping into a pure `fn map_outcome(...)` (no DB) and add unit tests for `Flipped`, `AlreadyRead`, `NotFound` transitions.
4. Edit `docs/endpoint-checklist/groups/finance.md`: for every row whose cited test still greps `#\[ignore.*BIT-(417|440)`, revert `done → partial` (or introduce a `quarantined` status) and drop the fabricated 100% claim in `README.md`. Add a footer paragraph documenting that `#[ignore]`d tests never count as `done`.
5. Add a `#[test]` in `docs/endpoint-checklist/tests` (or a shell test) that greps every row marked `done` and fails if any cited test file contains an `#[ignore]` for that test name — mechanically prevents the honesty drift from returning.
6. Cross-reference the SQL-string guards from a short comment near the fix so a future reader sees why a plain string assertion exists.

## Alternatives considered
- **Wait for BIT-352 to repair the sqlx harness** — rejected because BIT-352 has no timeline and three fixes already shipped unguarded in one 24h window; the anti-pattern is compounding.
- **Un-`#[ignore]` the individual tests and let them fail loudly** — rejected because BIT-351 is a shared harness issue (not per-test), so un-ignoring cascades red across dozens of unrelated tests and blocks the merge gate for everyone.

## Root-cause trace
N/A — test-gap doesn't need backward tracing. The proximate cause is a shared harness (`BIT-351`) that has stayed broken long enough for authors to `#[ignore]` new tests as the local convention. The plan doesn't fix BIT-351; it fills the specific coverage gaps left by three recent merges.

## Test plan
- [ ] `cargo test -p db --test thread_participant_state_tests count_unread_query_excludes_soft_deleted` — new non-DB assertion, must pass on the current fix and fail if the `tps.deleted_at IS NULL` predicate is removed.
- [ ] `cargo test -p db --lib blocked_among_rls_sql_shape` — new non-DB assertion locking the bidirectional block predicate.
- [ ] `cargo test -p reality-server favorite_alert_read_outcome_transitions` — new unit test on the extracted `map_outcome`.
- [ ] Add a `docs/endpoint-checklist` mechanical test / grep-based checker; document run command in `docs/endpoint-checklist/README.md`.
- [ ] Local run: `cargo test -p db && cargo test -p reality-server && cargo test -p api-server`.

## Out of scope
- Repairing BIT-351 sqlx-test harness — that's BIT-352, tracked separately.
- Un-ignoring the four `#[sqlx::test]`s already in `thread_participant_state_tests.rs` — they wait on BIT-352.
- Fixing the actual endpoint 500 bugs behind the BIT-417/BIT-440 quarantines in `market_pricing`/`multi_currency` — separately handled by `bug-multi-currency-enum-rename_all-broken-wire-and-db`.

## After-merge
- Move this file to `plans/_archive/test-gap-bit-351-quarantine-hides-regression-guards.md`
- Mark the matching `backlog.json` row as `status: "done"`
