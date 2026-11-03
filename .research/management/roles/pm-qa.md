# pm-qa — 2026-11-03

**Last run:** 2026-11-03 (prior: 2026-08-25 — 70 days stale; rotation idx 3 → 4)

## Summary (<=2 sentences)

Sprint-coded surfaces are all `done` on paper, but the active test gap this routine window surfaced three new error-swallowing patterns in merged / audit-visible handlers (`list_templates`, `get_listing`, `suspend_organization`) — all shipping on `main` behind an unwrap_or_default / log-and-skip anti-pattern. Pipeline wedge (#3215) means 8 approved-but-unmerged PRs sit in `review` — QA signal: the correctness of those 8 PRs isn't the risk right now; the risk is that review feedback can't be acted on and merged-code audits run against stale heads.

## Next actions

1. **Add a repo-wide lint for `unwrap_or_default()` on `Result`** (biome rule for TS equivalents; clippy lint `clippy::unwrap_or_default` already covers Rust — raise to deny for `routes/` crates). Priority: medium. DoD: `cargo clippy --workspace --all-targets -- -D clippy::unwrap_or_default` passes on dev; CI gate added. Dependency: none.
2. **Backfill cross-tenant regression tests for the 3 new api-handlers findings** (`suspend_cascade`, `templates list`, `listing photos`). Priority: medium. DoD: three failing-on-main tests under `backend/servers/api-server/tests/suites/` land before the fix PRs do; follows the `_cross_org_idor_tests.rs` naming pattern. Dependency: none — can be driven as a dispatcher implementer task.
3. **Confirm pipeline-wedge workaround for #3215 at QA level**. Priority: high. DoD: either (a) `claude-code-router[bot]` gets a second-reviewer identity that can approve its own PRs, or (b) a human reviewer sweeps the 8 approved-verdict assignments weekly. Dependency: pm-devops / pm-tech-lead decision.
4. **Add Playwright/jest smoke on the new _number helper** (`frontend/apps/reality-web/src/app/[locale]/account/listings/[id]/edit/_number.ts`) at UI level — unit covers the extracted logic; a smoke prevents regressions in the parent route. Priority: low. DoD: 1 Playwright scenario against seeded listing-edit page.
5. **Walk the 10 stalled PRs (>7d idle, non-draft) once per fortnight** and move each to one of: resolved, revived with a decision, or closed. Priority: low. DoD: `gh pr list` 7d-idle count halves within 2 weeks. Dependency: operator time.

## Risks

1. **unwrap_or_default sprawl** — probability high, impact medium. Three fresh instances in merged handlers in one week suggests the pattern isn't isolated. Mitigation: lint + sweep.
2. **Pipeline wedge normalisation** — probability high, impact high. The dispatcher continues to accumulate approved PRs it can't merge; teams start treating merge-latency as background noise and stop treating `review` as actionable. Mitigation: force resolution of #3215 within the next sprint.
3. **Mobile-native cloud build blocker (#2652)** — probability high, impact medium. QA cannot run KMP tests in the cloud routine; cumulative risk on realtor Create Listing + other KMP stories. Mitigation: pm-devops action already open.

## Open questions

- Is the dispatcher's self-review verdict trusted enough that we should relax the human-approval requirement on auto-reviewer-approved PRs with `confidence=high` and `score>=3`?
- Should Tier-1d adopted findings count as "reviewed" for the review_cursor, or do we want the routine to also do a lighter cross-check of the segment?

## Decisions needed

- Pipeline wedge #3215 — decide between bot-identity split vs. human-sweep cadence vs. GitHub ruleset tweak. Owner: pm-tech-lead.
