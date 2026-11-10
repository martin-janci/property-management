# PPT Project State

_Generated: 2026-11-09 — routine Phase 1.6 incremental upkeep. Role focus: **pm-qa** (76-day stale rotation slot; pm-cursor idx 3 → 4). pm-scrum-master always-on. Coverage `scan_kind=upkeep`; coverage_cursor idx 0 → 1 (epic-10a re-checked, still 3/3 done; no PRs touched OAuth surfaces this window)._

## Delta since last routine run (2026-11-06)

- **7 PRs merged** in ~64h (see Shipped below) — all code-review relands + a session-revocation fix; every PR shipped with a new regression test file.
- **8 gh-issue-\* follow-ups closed** (3283, 3304, 3305, 3306, 3307, 3308, 3309, 3310 — issue state=closed as of 2026-11-09). These were tracked as `in-progress` in action-list; all 8 moved to `done` and archived this run.
- **4 action-list code-review items resolved** by this run's merged PRs (see Shipped mapping).
- **1 new issue: #3357** — `dispatcher: pipeline fully stalled — 3 approved PRs blocked on non-mechanical conflicts + Tier-1d signals are a dead-end` (filed 2026-11-08 by the dispatcher itself; labels: `follow-up`, `blocked`). Operator action required — the dispatcher cannot self-heal.
- **1 blocker clearing**: issue #3187 (cross-tenant assign-404 test) is now CLOSED; #2652 (mobile-native KMP cloud builds) is also CLOSED (but the structural starvation gap remains — see carried risks).
- **1 quarantined assignment carried**: `code-review-ppt-web-core-refresh-token-localstorage` PR #3294 — the only non-mobile-native in-flight item.

## Executive summary

- **Delivery: 49/49 stories done across 13 epics in coverage.** No status flips this run; `last_checked` refreshed on epic-10a (idx 0, OAuth Provider Foundation — stories 10a-1/2/3 still done, nothing in the window touched OAuth).
- **7 PRs merged, all with regression tests.** Test-shadow debt did NOT grow this window — pm-qa's standing concern on dispatcher-spawned fixes shipping without failing-on-main tests remains a watch, not an active regression.
- **Dispatcher pipeline health is the real blocker now.** 10+ runs at 0-claimed/0-merged until this burst; planner buffer at 6/72 (claimable). The 7-PR burst is explained by pre-existing relands finally merging, NOT by fresh claim throughput.
- **pm-qa rotation came up 76 days stale** — last run 2026-08-25. No fresh QA findings beyond the standing lint-adoption + dispatcher-self-test asks; test discipline on shipped PRs is actually improving.

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 4/4** at sprint level · 49/49 stories done in coverage.

| Epic | Sprint status | Coverage status | Shipped PRs this window |
|---|---|---|---|
| 6 — Announcements & Communication | done (6/6) | 6/6 done | — |
| 7A — Basic Document Management | done (5/5) | 5/5 done | — |
| 8A — Basic Notification Preferences | done (3/3) | 3/3 done | — |
| 10A — OAuth Provider Foundation | done (3/3) | 3/3 done · **last_checked 2026-11-09 (cursor re-check)** | — |
| 10B — Platform Administration | done (7/7) | 7/7 done | — |
| 80 — Dispute Resolution | done (3/3) | 3/3 done | — |

## Shipped since last run (7 PRs merged)

- **#3351** — code-review-ppt-web-ui-registry-no-i18n: wire registry pet/vehicle forms + lists through react-i18next (merged 2026-11-07). Closes action-list `code-review-ppt-web-ui-registry-no-i18n`.
- **#3350** — code-review-reality-web-share-cmp-unchecked-json: validate shared-comparison listing bodies at the API boundary (merged 2026-11-07). Closes action-list `code-review-reality-web-share-cmp-unchecked-json`.
- **#3353** — gh-issue-3352: revoke other sessions on authenticated password change (merged 2026-11-07). Closes issue #3352.
- **#3354** — code-review-ppt-web-ui-registry-page-no-auth: send auth context on registry requests (merged 2026-11-07).
- **#3355** — code-review-ppt-web-ui-registry-page-i18n-gap: translate RegistryPage chrome (merged 2026-11-07). Closes action-list `code-review-ppt-web-ui-registry-page-i18n-gap`.
- **#3359** — code-review-ppt-web-ui-registry-mutation-reland: surface registry destructive-action outcomes via toast (merged 2026-11-08). Closes action-list `code-review-ppt-web-ui-registry-mutation-silent-fail`.
- **#3358** — screen-map-drift-pr-3293-reality: reflect realtor-detail i18n in screen-map (re-land on current dev) (merged 2026-11-08).

## What's next (top 5 from the ranked plan)

1. **[high] pm-scrum-master** — Promote a non-mobile-native code-review vector from the next Phase 1.5 segment (ppt-web-core / reality-server oldest = 2026-11-04) to refill the planner buffer. Why: planner buffer is 6/72; 8 of 9 open backlog items are mobile-native-kmp (structurally unclaimable in cloud) — refill is the single highest-leverage action this run.
2. **[high] pm-scrum-master** — Triage #3357: rebase the 3 blocked PRs OR adjust Tier-1d signal sourcing to stop emitting dead-end vectors. Why: the dispatcher cannot self-heal; this is the direct cause of the ~10-run 0-merged streak.
3. **[high] pm-devops** — Unblock mobile-native/KMP builds in the cloud runner (issue #2652 CLOSED but structural gap unchanged; #2966 still open). Why: structural starvation — 5+ backlog items permanently unclaimable in cloud.
4. **[medium] pm-scrum-master** — Open the diff for code-review-ppt-web-core-refresh-token-localstorage and upgrade signal confidence from medium → high (or drop). Why: it is the ONLY non-mobile-native open row today; stays blocked by Phase 3 gates at score=2+confidence=medium.
5. **[medium] pm-qa** — Add a dispatcher-self-test.sh test case covering the quarantined-row recovery path so #3357-class stalls fail CI instead of silently queueing.

## Blockers

- **#3357 dispatcher stalled pipeline** (filed 2026-11-08, labels: `follow-up`, `blocked`). Operator action required; dispatcher cannot self-heal. Owner: pm-devops / operator.
- **mobile-native-kmp cloud-landability starvation** (carries indefinitely until pm-devops unblocks local-toolchain-in-cloud; issue #2652 CLOSED but gap persists).

## Decisions needed

- **Promotion policy for score=2 + confidence=medium security vectors** — current fast-track requires `confidence=high`, so `code-review-ppt-web-core-refresh-token-localstorage` stays in `open` unless the confidence is upgraded. Owner: pm-security.
- **Buffer-low trigger text promotion as a hard-stop** — should the routine's buffer-low fire text (received as `buffer-low: claimable=6/72`) force Phase 1.5 to pick the oldest non-mobile-native segment regardless of churn-alignment? Owner: pm-scrum-master.

## Role summaries (this run)

- **pm-scrum-master** (always-on): 7 PRs merged, 12 action-list items done this run, planner buffer collapsed to 6/72 — refill is the priority.
- **pm-qa** (rotation; last run 2026-08-25 → 76 days stale): test discipline on shipped PRs is improving (7/7 shipped with regression tests this window); carried decisions on mobile-rn lint adoption + dispatcher-self-test remain the open QA asks.
