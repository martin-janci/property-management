# PPT Project State

_Generated: 2026-10-24 — routine Phase 1.6 upkeep (pm-backend rotation slot; pm-cursor idx 1 → 2) + pm-scrum-master always-on. Coverage `scan_kind=upkeep`; coverage_cursor idx 11 → 12 (epic-8a Basic Notification Preferences re-checked, all 3 stories still done, no material change). ~29h since 2026-10-23 run — on schedule._

## Delta since last routine run (2026-10-23)

- **7 PRs merged** — all dispatcher-authored auto-fix flow, none sprint stories:
  - **#3151** reality-web agency-href-scheme-xss (http(s) allowlist on agency-authored hrefs) — resolves backlog `code-review-reality-web-agency-href-scheme-xss` (`status: done`).
  - **#3145** api-core push-fanout client-build-expect (surface HTTP client build errors instead of panic) — resolves `code-review-api-core-push-fanout-client-build-expect` (`status: done`); action-list row flipped `open → done` this run.
  - **#3144** reality-web agency gradient-color sanitisation — already resolved yesterday.
  - **#3147** gh-issue-3136: migration import upload storage-error handling + happy-path persistence test.
  - **#3146** gh-issue-3132: bound PRICE_MAP_CACHE eviction (insertion-order index).
  - **#3148** gh-issue-3143: cargo-deny ammonia RUSTSEC-2026-0373 patched-floor guard — resolves `gh-issue-3143` (`status: done`); action-list row flipped `open → done` this run.
  - **#3150** gh-issue-3149: route AML/DSA client calls to /api/v1/aml-dsa (Closes #3149).
- **No new sprint story movement.** Coverage stays 49/49 done; all 11 open assignments are already `review` with `verdict=approve` and self-approval-blocked (`needs-human-review` label), awaiting human merge.
- **Dispatcher buffer-low fire (this run's trigger payload):** `claimable=12/72 — coverage exhausted (49/49 done), backlog+retry avail=0; refill planner please`. Interpretation below.

## Executive summary

- **Delivery: 49/49 stories done across 13 epics in coverage.** No status flips this run; `last_checked` refreshed on all 3 epic-8a stories (cursor re-check — negative, all still done).
- **Dispatcher starvation diagnosis.** Buffer-low is NOT a coverage regression — it is a *supply* problem on cloud-claimable plans:
  1. Coverage is 49/49 done → nothing new from epic-catalog work for the planner to seed from.
  2. The 2 `ready` backlog plans (`code-review-mobile-native-kmp-create-listing-not-wired`, `code-review-mobile-native-kmp-inquiries-response-contract`) are **`Mode: local-only`** — cloud dispatcher cannot claim them (gated by #2652 KMP cloud-build egress).
  3. The 23 remaining open action-list rows that are NOT yet assigned have **no plan files yet**; the planner has to produce the plans before the dispatcher can claim them. That is the actual bottleneck this run.
- **Fix this run:** 3 new pm-level `next_actions` added to action-list.json — the lead one commissions a cloud-ok plan for the api-core workflow batch-abort bug (score 2, confidence medium) so there is at least one claimable backlog unit while #2652 keeps the KMP plans parked. The pm-scrum-master row surfaces the planner-starvation diagnosis itself so the next run picks it up.
- **Not fixed this run:** #2652 (KMP cloud builds) stays open; it is the structural blocker and needs the on-demand local `/ppt-project-management scan` to refresh coverage against the full epic catalog once mobile-native work can land again.

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 3/5** at sprint level (sprint-status.yaml); coverage-level all extended epics are complete.

| Epic | Sprint status | Coverage status | Shipped PRs this window |
|---|---|---|---|
| 6 — Announcements & Communication | in-progress (3/6)† | 6/6 done | — |
| 7a — Basic Document Management | in-progress (2/5)† | 5/5 done | — |
| 8a — Basic Notification Preferences | done (3/3) | 3/3 done (re-checked today) | — |
| 10a — OAuth Provider Foundation | done (3/3) | 3/3 done | — |
| 10b — Platform Administration | in-progress (7/7)† | 7/7 done | — |

† The sprint-status.yaml "stories_completed" field for 6/7a/10b is a stale manual count; the per-story `development_status` block shows all 43 stories at `done`. Coverage.json (the authoritative delivery map) agrees: 49/49 done.

## Shipped since last run (not sprint work — dispatcher auto-fix flow)

- #3151 — reality-web agency href scheme XSS allowlist (resolves `code-review-reality-web-agency-href-scheme-xss`)
- #3148 — cargo-deny ammonia RUSTSEC-2026-0373 patched-floor guard (resolves `gh-issue-3143`)
- #3147 — migration import upload storage-error + persistence test (resolves `gh-issue-3136`)
- #3146 — PRICE_MAP_CACHE insertion-order eviction cap (resolves `gh-issue-3132`)
- #3145 — api-core push-fanout surface client-build errors (resolves `code-review-api-core-push-fanout-client-build-expect`)
- #3150 — route AML/DSA client to /api/v1/aml-dsa (resolves `gh-issue-3149`)
- #3144 — reality-web agency gradient color sanitisation (dispatcher post-merge-reviewed)

## What's next (top 5 — refreshed from action-list.json)

1. **high** `pm-backend-plan-api-core-workflow-condition-err-aborts-batch` — pm-backend to author a cloud-ok plan for the workflow_executor process_event batch-abort bug (adds a claimable backlog unit for the cloud dispatcher while local-only KMP plans stay parked).
2. **high** `pm-scrum-master-buffer-low-planner-starved-coverage-exhausted` — diagnosis of the buffer-low fire; recommends commissioning plans for the 7 open api-core / reality-server / ppt-web-core un-planned items OR running local `/ppt-project-management scan` to refresh coverage.
3. **high** `gh-issue-3124` — cargo-deny advisories on dev (RUSTSEC-2026-0368/0369 for `zip 8.6.0` via `rust_xlsxwriter`) — still blocks every backend PR merge on top of #2966 and #2652 (**Note:** #3148 only closed the *ammonia* side via #3143; the zip side is still open).
4. **high** `pm-devops-unblock-mobile-native-cloud-builds` — unblock mobile-native/KMP cloud builds (issue #2652); 5+ open backlog items + both `ready` plans remain structurally unclaimable in cloud until this lands.
5. **medium** `code-review-api-handlers-violations-missing-role-authz` — SECURITY: violations router gates every privileged mutation on tenant membership only; PR #3072 already in review with verdict=approve, awaiting human merge (self-approval blocked).

## Blockers

- **#2652** — mobile-native/KMP cloud builds blocked on egress to `dl.google.com` (Android SDK). 5+ open backlog items + both `ready` plans unclaimable in cloud until unblocked. _Owner: pm-devops._
- **#3124** (zip-side still open) + **#2966** (swagger-ui egress) — compound backend-merge gating. _Owner: pm-security + pm-devops._
- **Planner-side supply** — only 2 cloud-ok backlog items at score ≥3 across the whole backlog, and both already have PRs in review; new cloud-ok plans must be commissioned by pm-backend in next 1-2 runs or the dispatcher stays dark. _Owner: pm-backend / pm-scrum-master._

## Role focus today

- `pm-scrum-master` (always-on): planner-starvation diagnosis, action-list refill, cursor advance.
- `pm-backend` (rotation): identified `code-review-api-core-workflow-condition-err-aborts-batch` as the next cloud-ok plan candidate; flagged 6 backlog rows aged ≥14d for decay-review.

<sub>Prior upkeep note (archived): 2026-10-19 — pm-integration rotation; coverage_cursor 10 → 11 (epic-7a re-checked). Earlier: 2026-10-16 — pm-data rotation; coverage_cursor 8 → 9 (epic-83 re-checked)._
