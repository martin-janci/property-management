# PPT Project State

_Generated: 2026-10-06 — routine Phase 1.6 upkeep (pm-data rotation slot; pm-cursor idx 6 -> 7) + pm-scrum-master always-on. Coverage `scan_kind=upkeep`; coverage_cursor idx 8 -> 9 (epic-83 re-checked, no material change; advances to epic-84). 72h / 3-day lag since last routine run (2026-10-03T03:12:00Z)._

## Executive summary

- **Delivery: 49/49 stories done across 13 epics in coverage.** No status flips this run. Last partial epics (sprint-status.yaml drift on 6, 7A, 10B, 80) still carry forward — no action, next deep scan reconciles.
- **6 PRs merged** in the 72h window since #3015: three reality-web silent-fail / silent-dropped-input fixes (#3019, #3021, #3022), two ppt-web screen-map docs reconciles (#3023, #3027), and one dispatcher doc hardening (#3018). Zero coverage status flips — all surfaces already done.
- **6 draft PRs opened by the dispatcher** this window (#3020, #3024, #3025, #3026, #3028, plus #2902 still stalled). Most carry `verdict=approve note=self-approve blocked bot==author` — they need human merge, not more automation. #3024 landed with `verdict=changes` (dashboard-manager.md note falsely claimed 'no role gate') and has since been superseded by the merged #3027.
- **Dominant delivery signal:** half of this run's merged PRs (3/6: #3019, #3021, #3022) are **silent-failure or silent-dropped-input** fixes on reality-web — a repeating class. No end-to-end data-contract test catches them. New risk + action list entries added below.
- **Standing infra blockers unchanged:** #2966 utoipa-swagger-ui egress 403 still gates all backend PRs (3 IDOR retries parked); #2652 KMP cloud build gate still open. PR #3026 (vendor swagger-ui, approved via self-authored draft) is the proposed fix for #2966 and needs human approve+merge.

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 3 / 5** at sprint level (sprint-status.yaml, no file in repo this run); coverage-level all 13 epics complete.

| Epic | Sprint status | Coverage status | Shipped PRs this window |
|---|---|---|---|
| 6 — Announcements & Communication | in-progress (3/6) sprint-drift | 6/6 done | — |
| 7A — Basic Document Management | in-progress (2/5) sprint-drift | 5/5 done | — |
| 8A — Basic Notification Preferences | done (3/3) | 3/3 done | — |
| 10A — OAuth Provider Foundation | done (3/3) | 3/3 done | — |
| 10B — Platform Administration | in-progress (7/7) sprint-drift | 7/7 done · **last_checked 2026-10-06** | #3023, #3027 (docs/screen-map reconciles for role-gated dashboard surfaces shipped prior) |
| 80 — Dispute Resolution | partial (1/3) sprint-drift | 3/3 done | — |
| 83 — Portal Integrations | — | 3/3 done · **last_checked 2026-10-06 (cursor idx 8 re-check)** | — |
| 84 — Documents & e-signature | — | 5/5 done · **84-3 last_checked 2026-10-06** | #3021 (reality-web PriceAlerts mark-read error surfacing) |
| reality-web correctness (cross-cutting) | — | — | #3019 (/report dead-attachment control removed, Closes #3017), #3022 (/sell dead seller-contact fields removed, Closes #3016) |
| dispatcher hygiene | — | — | #3018 (forbid `git stash` in shared worktrees, Closes #3012) |

## Shipped since last run (6 PRs merged since #3015)

- **#3018** — docs/dispatcher: codify "no `git stash` in shared worktrees" rule (Closes #3012); prevents a class of lost-work incidents when auto-rebaser cross-contaminates worktrees.
- **#3019** — reality-web /report: remove never-transmitted attachment control (Closes #3017). UI collected files but no payload field ever carried them.
- **#3021** — reality-web PriceAlerts mark-read: surface error toast on backend failure instead of swallowing. Patches a silent-failure hole on the alerts feed shipped by #2287.
- **#3022** — reality-web /sell wizard: drop dead seller-contact fields (Closes #3016). Same class as #3017 — form schema > payload schema.
- **#3023** — docs/screens/ppt/home: reconcile with PR #2995 Home CTA role-gate; brings docs in line with shipped RBAC hardening.
- **#3027** — docs/screens/ppt-web/dashboard: reconcile with PR #3002 manager-dashboard role-gate; supersedes the earlier draft #3024 that reviewer flagged `verdict=changes` for falsely claiming "no role gate".

## Open PRs touched this window (dispatcher drafts awaiting human merge)

| PR | Task | Reviewer verdict | Blocker |
|---|---|---|---|
| #2902 | screen-map-drift-pr-2894-reality | `verdict=changes` scope-drift (stale, contaminated branch) | Human reset + re-cut |
| #3020 | code-review-ppt-web-ui-workflow-automation-toasts-hardcoded-english | `verdict=approve` rebased, i18n 28 keys x 6 locales | Human merge |
| #3024 | screen-map-drift-pr-2994-ppt | `verdict=changes` note false | **Superseded by merged #3027 — close draft** |
| #3025 | pm-devops-unblock-mobile-rn-jest-expo-version-rot (Closes #2951) | `verdict=approve` COMMENT (self-approve blocked) | Human approve + merge |
| #3026 | pm-devops-unblock-api-server-utoipa-swagger-egress (Closes #2966) | `verdict=approve` COMMENT (self-approve blocked) | Human approve + merge — **highest-leverage unblock in the backlog** |
| #3028 | code-review-reality-server-add-to-compare-toctou-max-cap | `verdict=approve` TOCTOU + regression test | Human merge |

## Blockers (carried, none new)

- **#2966** — api-server utoipa-swagger-ui egress 403. **Proposed fix in draft PR #3026 (vendored swagger-ui, approved);** needs human merge to unblock all 3 IDOR retries and ~9 claimable items. **Highest-leverage action in the backlog.** Owner: pm-devops.
- **#2951** — mobile RN jest-expo version rot. Fix in draft PR #3025, approved. Owner: pm-devops.
- **#2946 / #2944 / #2945** — portfolio_analytics / violation / portfolio_properties cross-tenant IDOR retries. All parked behind #2966. Owner: pm-security.
- **#2652** — mobile-native/KMP cloud build gate. Owner: pm-devops.

## Buffer-low status

- Dispatcher reports continue to show claimable < floor (details in Phase 1 observation data; exact count not re-measured this run). Carried-forward infra blockers unchanged. The three draft PRs #3025/#3026/#3028 approved-but-not-merged are **the entire backlog of near-term unblocks** — all three are gated on human review-and-merge, not on more implementer work.

## Role analysis — pm-data (2026-10-06, rotation idx 6; previous run 2026-07-23, 75 days ago)

### Signal picked up from the 6 merged PRs this window

Three of the six PRs this run are the same shape: **"UI collects data, data never leaves the browser (or failure never surfaces to user)."** All three were found by churn-hotspot code review, not by any end-to-end test:

| PR | What silently failed | Source of truth that would have caught it |
|---|---|---|
| #3019 | /report attachment control existed in the form, no multipart field in the submit payload | A data-contract test that captures the network request and diff-asserts form-schema vs payload-schema |
| #3021 | PriceAlerts mark-read PATCH returned 4xx/5xx and the UI stayed silent | A test that mocks the PATCH to fail and asserts a visible user-facing error |
| #3022 | /sell wizard collected seller-contact fields that the API no longer accepts | Same data-contract test — the fields had no payload home |

**pm-data interpretation.** This is a data-pipeline hygiene gap, not a product bug. The frontend has no primitive for "what fields does this form intend to transmit?" and no CI gate that compares that intent to what the network actually sees. The failure mode is latent — the three fixes landed this week each describe a feature that has shipped already and been silently losing data for an unknown window. **We cannot answer "how many /sell submissions never carried seller_phone before #3022?"** because there is no analytics event capturing the submitted payload shape.

### Signal picked up from assignments.json (dispatcher ledger)

12 open items currently sit in `status=review` with reviewer notes. Three recurring patterns in `reviewer_summary.verdict`:

- **scope-drift** (2 of 12): #2902, #3024 — PRs that touched `.research/management/**` or screen-map metadata beyond their stated scope.
- **self-approve blocked bot==author** (4 of 12): #2997, #3025, #3026, #3028 — reviewer approved via COMMENT because bot cannot approve its own PR. These are *all* unblocked by human merge, no further work.
- **screen-map stale** (1 of 12): #3024 — note content outdated vs the shipped surface.

**pm-data interpretation.** There is a growing structured corpus of reviewer feedback in `assignments.json` + `assignments-archive.json` that **nobody is counting.** A one-pager that buckets the notes by regex (scope-drift / i18n / screen-map-stale / self-approve-blocked / cloud-build-blocked / rebase-needed) would turn 2 months of ledger entries into a trend. Low-cost, high-ROI pm-data task.

### Standing pm-data risks (carried, no change)

- Shipped MVP features (Epics 6, 10A, 10B, 80, 84) still lack KPI instrumentation; no analytics event emission assertion helper in the workspace.
- FaultStatusCount vs owner/portfolio KPI definition drift (2026-05-28 open decision).
- `support_tooling_events` + audit trail have no TTL / retention policy (GDPR exposure).
- Mobile (RN + KMP) event tracking parity with web unknown.
- Webhook delivery/retry/failure emits only logs, no analytics events.

### Suggested next pm-data actions (3)

1. **Data-contract fuzz test pattern.** Author a vitest helper in `frontend/packages/testing-helpers` that mounts a form, submits against msw, and diff-asserts form-schema vs captured request body. Pilot on `/sell` + `/report` (both just fixed this run) so a regression gets caught at test-time. Owner: pm-qa. Priority: medium. **(Added to action-list.json this run.)**
2. **Reviewer-feedback bucket counter.** Daily jq-based classifier over `.research/management/assignments.json` + archives, emitting a top-N bucket report into project-state.md. Retire if signal is thin after 2 weeks. Owner: pm-data. Priority: low. **(Added to action-list.json this run.)**
3. **Minimum-analytics DoD proposal.** Draft a 1-page policy for pm-scrum-master: new stories merging a user-visible surface must emit at least one named analytics event + document the schema. Not a blocking gate yet — advisory for 1 sprint, then upgrade. Carried from 2026-07-23.

### Open pm-data questions (unchanged)

- Analytics platform choice (bespoke platform-admin vs Amplitude/PostHog/Segment)?
- GDPR / retention policy for `support_tooling_events` (TTL vs indefinite)?
- Dispute lifecycle KPIs — customer-contract requirement or product-internal only?

## Coverage (upkeep this run — 2026-10-06)

- **`coverage.json` refreshed via mechanical upkeep** — `scan_kind=upkeep`, `generated=2026-10-06T06:30:00Z`.
- **Epic re-check: epic-83 (cursor idx 8)** — all 3 stories (83-1 Airbnb, 83-2 Booking, 83-3 portal webhooks) still `done`; no merged PR in the 2026-10-03..10-06 window touched OTA / webhook surfaces. `last_checked = 2026-10-06` stamped on all 3; negative-check evidence line appended.
- **Merged-PR evidence appended** on 84-3-price-tracking (PR #3021) and 10b-1-organization-management-dashboard (PR #3023 + #3027). No status flips.
- **`coverage_cursor` advances 8 -> 9** (epic-83 -> epic-84 next run).
- **`pm_cursor` advances 6 -> 7** (pm-data -> pm-integration next). `role_last_run["pm-data"] = "2026-10-06"`.
- **Composition unchanged: 49 done · 0 partial · 0 not-started** across 13 epics. Missing UC links: 3 (UC-33.1/33.2/33.3, carried).
