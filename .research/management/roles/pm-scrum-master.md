# pm-scrum-master — 2026-11-03

## Executive summary

Sprint "Epic 6, 7A, 8A & 10A" remains `done` on paper (4/4 epics, 49/49 coverage stories). The run surfaced five new routine signals, two of which are score-2 api-handlers error-swallowing bugs. **The sustained delivery drag is the merge-pipeline deadlock on issue #3215 — 8 approved assignments still blocked on merge because the dispatcher bot cannot grant GitHub `APPROVED` reviews to its own PRs.** The dispatcher is still producing (2 draft PRs opened today: #3221, #3222), but nothing has merged since 2026-11-02T12:36Z.

## Sprint progress

| Epic | Sprint status | Coverage status | Shipped since last run |
|---|---|---|---|
| 6 — Announcements & Communication | done (6/6) | 6/6 done | — |
| 7A — Basic Document Management | done (5/5) | 5/5 done | — |
| 8A — Basic Notification Preferences | done (3/3) | 3/3 done | — |
| 10A — OAuth Provider Foundation | done (3/3) | 3/3 done | — |
| 10B / 80 / 79 / 81 / 82-85 (extended) | done | all done | #3220 (community i18n), #3217 (edit/_number refactor), #3219 (price_map LIKE escape) |

Epic cursor advanced to idx 1 (epic-10a last_checked bumped to 2026-11-03 for all 3 stories; no status flips).

## Shipped since last run (5 PRs)

- **#3216** screen-map-drift reconcile — docs/screens/reality/account-listing-edit.md back-filled after #3212 drift.
- **#3217** repeated-churn edit/page.tsx — _number helper + tests extracted; followup to the 2026-11-02 repeated-churn signal.
- **#3218** code-review EvidenceUploader i18n (sk/cs/de/en/hu/pl) — validate() errors + drop-zone translated.
- **#3219** churn-hotspot price_map LIKE metacharacter escape — small SQL-injection-adjacent fix (user-controlled `city` was interpolated into LIKE pattern).
- **#3220** code-review community-feature i18n backfill — 17 files touched (16 .tsx + 6 locale JSONs + 1 test). This PR dominates this run's churn hotspot picture; the hotspots are planned, not instability.

## Next actions (top 5)

1. **Unblock the merge pipeline on #3215** — pm-tech-lead decision; blocks 8 approved assignments. priority: high.
2. **Backfill cross-tenant regression tests for the 3 new api-handlers findings** (suspend_cascade / templates list / listing photos). owner: pm-qa. priority: medium.
3. **#2652 KMP cloud build egress** — still unresolved; realtor Create Listing (score 5 ready plan) remains unlandable. owner: pm-devops. priority: high.
4. **Lint sweep for `unwrap_or_default()` on `Result<_, E>` in routes/** — clippy deny gate. owner: pm-backend. priority: medium.
5. **Stalled PR fortnightly walk** — 10 PRs >7d idle (2 accounting UC-ACC pair at 96d, 6 dependabot, 2 docs/i18n at ~15-25d). owner: pm-scrum-master. priority: low.

## Blockers

- **#3215 — merge pipeline wedge (approved PRs cannot be merged)**: 8 assignments in `review` state, all with `verdict=approve`, waiting on a GitHub `APPROVED` review the bot cannot grant to itself.
- **#2652 — KMP cloud build egress** (AGP download blocked): structurally blocks every mobile-native-kmp task from landing in the cloud routine.
- **Buffer at 11/13 claimable** per dispatcher (dep-blocked=2); still above the 1/3 half-empty threshold but trending down because nothing is draining via merges.

## Decisions needed

- Resolution path for #3215 (bot-identity split vs. human-review sweep vs. GitHub ruleset tweak). Owner: pm-tech-lead.
- Whether a dispatcher `verdict=approve` + CI green should auto-merge after a cooldown window (and if so, how long). Owner: pm-tech-lead + pm-security.

## Role focus today

pm-qa (rotation idx 3 → 4). Last pm-qa run was 2026-08-25 (70 days ago). Full pm-qa output in `roles/pm-qa.md`.
