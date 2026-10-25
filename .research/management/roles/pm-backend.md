# pm-backend — 2026-10-25

_Phase 1.6 rotation (next_index 1 → 2). Last run 2026-07-30 — ~3 months stale; rotation hits a timely backend-touching window._

## summary

Backend made real progress this window — 3 backend-touching merges landed (**#3145** push-fanout error-surface hotfix, **#3146** PRICE_MAP_CACHE unbounded-growth fix on reality-server, **#3147** migration-import storage error handling), plus **#3148** cleared the ammonia RUSTSEC advisory. But **#3145 shipped WITHOUT a regression test**, repeating the exact pattern pm-backend's 2026-07-30 decision flagged (PR #2547 scheduler, PR #2568 Android SSO CSRF). And six backend PRs are stuck in review as part of the #3131 deadlock.

## next_actions

1. **{high}** Backfill a failing-on-main regression test for PR #3145 push-fanout error surfacing. **Dependency:** none. **DoD:** test exercises both the success path and the Redis-publish-failure path (surfaced via tracing); CI green on `cargo test -p api-server`.
2. **{high}** Review-queue triage for the 6 backend PRs in the review-queue deadlock (#3131). **Dependency:** pm-tech-lead. **DoD:** each backend PR in the stuck set has a named reviewer within 24h.
3. **{medium}** Audit PR #3147 migration-import error-handling for retry/rollback correctness (does a failed S3 PUT mid-import leave partial rows? what's the compensation?). **Dependency:** none. **DoD:** either a retry-safe guarantee documented in `docs/data/` or a follow-up issue filed.
4. **{medium}** Fast-lane PR #3158 (moderation sub-paths follow-up for #3157) — unblocks the CRS adjacency. **Dependency:** none. **DoD:** #3158 merges by 2026-10-27.
5. **{medium}** Audit `reality-web/src/app/[locale]/agency/[slug]/page.tsx` sanitize surface — touched TWICE this window (PR #3144, #3151). Extract a shared sanitize helper to prevent one-input-at-a-time fixing. **Dependency:** pm-tech-lead (shared crate design). **DoD:** `sanitizeAgencyHtml()` lives in a shared TS package; `page.tsx` has one import.
6. **{low}** Confirm the ammonia RUSTSEC advisory cleared by #3148 flips cargo-deny green on dev (`gh-issue-3124` was dropped from action-list.json 2026-10-22 but the exception row may still be present). **Dependency:** pm-devops. **DoD:** `cargo-deny check` passes without the ammonia exception on `dev`.

## risks

- **{high/high}** Hotfix regression gap (#3145 shipped without test). The push-fanout error path is business-critical (silent failure = unseen push drops). **Mitigation:** backfill test within 48h; enforce "hotfix needs follow-up at merge time" via PR template check.
- **{medium/high}** Review-queue deadlock aging backend PRs. Branches diverge from `dev`, sqlx offline-data drift, Cargo.lock conflicts — the longer a backend PR sits in review the worse the rebase cost. **Mitigation:** daily "stale backend reviews" digest line + per-role reviewer rotation.
- **{medium/medium}** Shared sanitize pattern debt (agency/[slug]/page.tsx twice this window). Each XSS/sanitize fix is a reactive single-input patch rather than defense-in-depth. **Mitigation:** extract shared sanitize helpers (as 2026-08-25 pm-qa flagged for CSV sanitizers).
- **{low/medium}** PRICE_MAP_CACHE eviction (#3146) may not catch all unbounded-key classes — need a short review of other anonymous-GET caches for the same pattern (listing search, agency index). **Mitigation:** 1h audit pass on reality-server anonymous caches.

## open_questions

- Which specific 6 PRs of the 15 stuck in review are backend-owned (payload didn't partition)? Candidates by title context: #3024, #3070, #3072, #3107, #3108, #3120 — needs confirmation from `gh pr list`.
- Is #3147 migration-import retry-safe end-to-end, or does a partial failure require operator intervention?
- Should the ammonia RUSTSEC exception row stay in cargo-deny.toml as a safety belt, or be removed now that #3148 pins the guard?

## decisions_needed

- Pre-merge CI gate requiring a modified-or-added test file in the same PR for any `fix(` or `hotfix` commit — owner: pm-tech-lead + pm-qa (ties back to 2026-08-25 pm-qa dispatcher-fix-PR regression-test decision)
