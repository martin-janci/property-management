# Role: pm-scrum-master — 2026-10-09

> Delivery synthesis. Always-on role (every Phase 1.6 run). Previous run: 2026-10-01 (8 days ago). Rotating role today: pm-data (idx 6 → 7).

**Summary:** Six PRs merged in the 2026-10-08 bugfix dispatcher window (#3043-3048) — all small bug/UX follow-ups closing 4 freshly-opened `gh-issue-30**` tickets plus two code-review items on community-groups + /profile mock-data. Zero coverage status flips. Sprint posture unchanged: coverage still reads 49/49 done across 13 epics; sprint-status.yaml drift against Epic 6/7A/10B/80 persists (safe — matter for a deep scan, not for this upkeep). The three carried infra blockers (#2966 swagger-ui egress, #2652 KMP cloud build, #2951 jest-expo rot) are still open and still gate the three cross-tenant IDOR retries and the KMP code-review backlog.

## Shipped since last run (6 PRs, 2026-10-08)

- **#3048** — `code-review-ppt-web-core-group-join-leave-silent`: surface community group join/leave mutation errors (ppt-web routes/groups/community.tsx)
- **#3047** — `gh-issue-3041`: route reality-web /profile copy through next-intl (fixes hardcoded Slovak in a 4-locale app)
- **#3046** — `gh-issue-3042`: register `reports_submit` in @ppt/sitemap + surface report id on /report success
- **#3045** — `gh-issue-3040`: reuse errors.groupNotFound/common.back in group-detail not-found (i18n reuse)
- **#3044** — `gh-issue-3039`: fix /sell Publish 405 — createListing() POSTs to GET-only route
- **#3043** — `gh-issue-3029`: reset SyncSchedule save-error banner on re-edit (realtor-import flow UX)

## Sprint posture

- Sprint name unchanged: "Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth".
- epics_done at sprint-status level: **3/5** (Epic 8A + 10A + 10B done; Epic 6/7A/80 flagged `in-progress`/`partial` in sprint-status.yaml but coverage-truth has them all `done`). This is sprint-status drift against the deeper coverage.json; no fresh regression surfaced this window.
- epics_done at coverage level: **13/13** (49/49 stories).
- No coverage status flips this run.
- No new blockers; the three carried ones (#2966, #2652, #2951) still gate the dispatcher's claimable pool.

## Buffer-low posture

- `action-list.json` has **36 open items** (= buffer target). No underflow; no refill needed this run beyond the pm-data new actions (+2 net after collapse).
- `claimable` from the dispatcher side is still the pain point (last snapshot showed `9/72` under the #2966 block); this routine does not re-sample claimable but no new "buffer-low" telegram signal came in.

## Noteworthy observations

- The 2026-10-08 window is a textbook post-merge-review follow-up cluster: all six PRs are small, mechanical fixes that closed issues opened in the past 7 days. The dispatcher's bug-triage loop is working; the risk is that none of these funnel defects (publish-405, /report success, group join-leave) are surfaced by metrics — only by reviewers. See the pm-data run for the analytics implications.
- No deep-scan needed; coverage upkeep is sufficient.
- The pm-security 2026-10-01 risk entries carry forward unchanged — all three cross-tenant IDOR retries still gated by #2966.

## Next actions

| Action | Priority | Dependency | Definition of done |
|---|---|---|---|
| Land `analytics-schema-foundations-2026-10-09` meta-task (collapses the six 2026-07-23 pm-data items) — ranked as high so the dispatcher claims it | high | pm-tech-lead (analytics platform decision) | one action-list row replacing six; schema + 4 funnel events emitted in next frontend sprint |
| Keep the top infra lever: unblock #2966 (utoipa-swagger-ui egress) — unchanged from 2026-10-01; still the single highest-leverage move for the backlog | high | pm-devops | api-server builds in cloud runner; the three IDOR retries claim and merge |
| Reviewer slot for the accounting trio **#2559 + #2555** (72d idle now) — carried; still no reviewer engagement | medium | pm-tech-lead | PRs reviewed and merged or explicitly deferred with a dated plan |
| Add `instrumentationStatus` to the screen-map frontmatter schema — low-cost structural change that unlocks mechanical "shipped-but-unmeasured" queries | medium | pm-tech-lead (screen-map schema owner) | schema update + validate script + 10 screens backfilled |

## Risks

| Risk | Probability | Impact | Mitigation |
|---|---|---|---|
| Analytics debt is now the delivery-shape risk — the publish-405 was a user-visible funnel defect we could not measure the impact of; same shape as announcement fan-out, dispute lifecycle, OAuth usage | high | medium | Land the pm-data meta-task; set a minimum-analytics DoD for new stories |
| Infra blockers (#2966 + #2652 + #2951) now 15+ days unresolved — the dispatcher is structurally bottlenecked and we have no SLA on infra fixes | high | high | Explicit named owner (pm-devops) with a dated milestone; escalate if not resolved by 2026-10-16 |
| 2026-07-23 pm-data items have been in the queue 78 days with zero promotion — reveals a ranker fairness issue where `medium` items from always-structural roles never win | medium | medium | Collapse-into-meta pattern (this run); or add a staleness boost to the ranker |

## Open questions

- Should we escalate #2966 to a named-owner-with-dated-SLA stance (e.g. "resolved or formal defer by 2026-10-16") or keep the current "pm-devops owns, no deadline" shape?
- Is the dispatcher's ranker ready to accept a staleness boost for items > N days open with no retry, or do we stick with the manual collapse-into-meta pattern?

## Decisions needed

- Analytics platform choice (bespoke pg vs Amplitude/PostHog/Segment) — owner: pm-tech-lead + pm-data (now 78 days open; blocks pm-data meta-task)
- Collapse-into-meta pattern as formal ranker rule, or keep manual? — owner: pm-tech-lead

## Scrum-Master-only extended keys

- **shipped_since_last_run:** #3043, #3044, #3045, #3046, #3047, #3048
- **sprint_progress:** sprint="Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"; epics_done=13; epics_total=13 (coverage view)
- **blockers (carried):**
  - #2966 utoipa-swagger-ui egress — owner: pm-devops
  - #2652 mobile-native/KMP cloud build — owner: pm-devops
  - #2951 mobile RN jest-expo version rot — owner: pm-devops
