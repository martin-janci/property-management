# pm-tech-lead — 2026-10-21

_Rotating role run (Phase 1.6, buffer-low: claimable=12/72). Static analysis only._

## Summary

The merge-gate landscape rotated this cycle: the twin cargo-deny advisory block (RUSTSEC-2026-0367/0368/0369 zip, -0373 ammonia, -0382 rustls-webpki) is cleared across PRs #3125/#3130/#3135, and the migration-upload silent-drop defect was fixed in #3127 with a regression test. But a worse structural blocker has emerged: issue #3131 reports the dispatcher merge pipeline is deadlocked because the dispatcher-bot token acts as `martin-janci` and so can only COMMENT, never APPROVE — reviewDecision never reaches APPROVED, and `ppt-pr-merge` aborts on all 16 approved-by-bot PRs. All five stalled drafts (#3028/#3070/#3072/#3074/#3075) share this single root cause.

## Next actions

| # | Action | Priority | Dependency | Definition of done |
|---|--------|----------|------------|-------------------|
| 1 | Resolve #3131 — provision a distinct dispatcher-bot GitHub identity (separate PAT or GH App) so dispatcher PRs can be APPROVED by a different actor than their author. Alternative: amend ppt-pr-merge to accept bot-approvals-of-self with a documented policy. | high | pm-devops / pm-tech-lead | 16-PR backlog's reviewDecision resolves to APPROVED; `ppt-pr-merge` lands at least one of #3028/3070/3072/3074/3075 as proof. |
| 2 | Address #3132 PRICE_MAP_CACHE DoS follow-up — replace the O(n log n) insert_bounded sort-on-every-insert with LRU or treap; constrain attacker-controlled cache keys (whitelist + length cap). | high | pm-backend | PRICE_MAP_CACHE insert is O(log n) amortized; cache keys bounded; cache-thrash regression test added. |
| 3 | Address #3136 migration-upload storage-error typing + happy-path test — match on typed `StorageError::InvalidContentType` variant rather than `Display` string; add happy-path persistence test. | medium | pm-backend | upload_import_file classifies 400 vs 503 by variant; happy-path test lives in `migration_import_upload_persists_file.rs`. |
| 4 | Close #3006 — collapse @ppt/api-client's triplicated auth-header + MFA-retry logic (gdprClient, paymentMatching, remaining consumers) into one shared `authenticatedFetchJson`; add a Biome rule banning new fetch primitives that bypass it. | medium | pm-tech-lead | One auth-header path; Biome rule in frontend/biome.jsonc; ppt-web + reality-web + admin-web green. | 
| 5 | Replicate the typed-response treatment from #3129 (AML list) onto the remaining untyped `serde_json::Value` list endpoints in `backend/servers/api-server/src/routes/` — identify the next three via `grep` and open one plan. | medium | pm-tech-lead | ≥3 remaining typed-list candidates identified; one promoted to `.research/plans/`. |
| 6 | Define a RUSTSEC exception policy (ADR) — four advisories were cleared in <24h by version bumps this time, but the pipeline lacks a documented ADR for the version-bump-vs-exception tradeoff. Capture in `docs/adr/` or `.research/management/decisions.md`. | low | pm-security | ADR merged; `backend/deny.toml` references it. |

## Risks

| Risk | Prob. | Impact | Mitigation |
|------|-------|--------|------------|
| Merge pipeline stays deadlocked beyond 48h (16 approved PRs blocked). Dispatcher backlog continues to accumulate without land path; buffer-low becomes buffer-starvation. | high | high | Fast-track action #1 (dispatcher identity split); until then, pm-tech-lead reviews + martin-janci human-approves the top-5 blocked PRs by hand so at least #3028/#3072 land. |
| #3132 PRICE_MAP_CACHE DoS is a two-attribute flaw (unbounded sort + attacker-controlled key); fixing one dimension without the other leaves the vector open. | medium | high | Treat #3132 as a single plan; require both the LRU/O(log n) refactor AND the key-whitelist in the same PR. |
| Churn hotspot in `backend/servers/api-server/src/routes/migration.rs` (#3127 landed with a 400/503 Display-string match that #3136 flags as fragile) indicates the migration route is being iterated under time pressure; expect ≥1 more follow-up issue. | medium | medium | Treat migration import routes as a mini-rotation target for the next 2 runs; add typed error classification helper so the next PR cannot regress. |
| Typed-response gap (post-#3129): remaining `serde_json::Value` responses leak schema stability across the generated OpenAPI/TypeSpec. Each one is a client-break waiting to happen. | medium | medium | Enumerate (next-action #5) and plan one per week until exhausted. |
| cargo-deny advisory volume (4 in 36h) suggests the dependency graph is aging faster than our bump cadence; next advisory may land on a crate without a clean upstream. | low | high | ADR (next-action #6) + a monthly dependabot-group review slot. |

## Open questions

1. Is pm-devops already working on a dispatcher-bot GitHub App identity, or is this unassigned?
2. For the ppt-pr-merge policy change in action #1, does the human-reviewer requirement (user story) need updating?
3. Does the PRICE_MAP_CACHE live in `backend/crates/` or in a reality-server route — the #3092 reference is stale and the file path should be reconfirmed before planning.
4. Should the typed-response refactor (action #5) ride on an existing epic (e.g. epic-79 API-client integration) or get a cross-cutting slot?

## Decisions needed

- **Dispatcher-bot identity split (#3131)** — owner: pm-tech-lead (coordinating with pm-devops). Blocks the entire merge pipeline until resolved.
- **ppt-pr-merge same-author-approval policy** — owner: pm-tech-lead. If the identity split is deferred, we need a documented alternative merge policy.
- **RUSTSEC exception ADR** — owner: pm-security. For when a version-bump path is unavailable.
