# Role: pm-data — 2026-10-09

> Data/analytics lens. Rotating role this run (pm_cursor rotation[6]). Static read-only. Previous run: 2026-07-23 (78 days ago).

**Summary:** Coverage still reads 49/49 stories done across 13 epics but the analytics debt has not moved since the 2026-07-23 pm-data run — all six KPI/retention action-list items I filed then remain unqueued because the dispatcher has been consumed by cloud-build unblocks (#2966, #2652) and reviewer-starved accounting PRs. The 2026-10-08 bugfix wave (#3043-3048 — reality-web /profile i18n, /sell 405, /report sitemap link, SyncSchedule banner, community group join/leave + not-found) touched surfaces that still ship with zero analytics instrumentation: we silently fixed user-visible funnel defects (publish-listing 405, group-join silent error) that we cannot measure the user-level impact of.

## Cross-reference to the 2026-10-08 bugfix wave (6 PRs, 5 closed issues)

- **Publish-listing funnel is unmeasurable.** PR #3044 fixed /sell wizard POST→405 (createListing hit a GET-only route). This is a user-visible funnel defect that silently blocked every listing-create for an unknown duration. There is no `listing_publish_attempt` / `listing_publish_fail` / `listing_publish_success` event anywhere in reality-web — we cannot answer "how many publishes failed between the regression landing and PR #3044 merging?". Classic case for a `web_vital_funnel_step` event schema.
- **/report submit success invisible.** PR #3046 wired the report submission to show the report id on success and registered the endpoint in `@ppt/sitemap`. No `report_submit_success` / `report_submit_failure` KPI anywhere — the content-moderation downstream dashboards are blind on inbound volume.
- **/profile locale drift was shipped undetected.** PR #3047 routed `/profile` copy through next-intl — a hardcoded-Slovak slip in a 4-locale app. The silent-locale defect is the same class as the ones the 2026-08-25 pm-qa run flagged for mobile-rn (i18next no-literal-string). We have no cross-platform metric for "hardcoded-string drift density" that would surface it before the post-merge reviewer catches it.
- **community group join/leave silently failed (PR #3048).** The community-groups surface has shipped buildStatus (ppt-web routes/groups/community.tsx is a top churn hotspot now 3rd window) but has zero event instrumentation on membership transitions — we don't know how many users were silently rejected during the window the mutation was swallowing errors.
- **SyncSchedule save-error banner reset (PR #3043).** Mirrors a UX funnel defect on the realtor-import flow — again shipped surface, zero funnel instrumentation.
- **Screen-map reconciliation drift (5 new drift signals).** screen-map-drift-pr-3033/3034/3036/3019/3022 are all `low` priority but structurally interesting for data-quality: our screen-maps describe `apiStatus`/`buildStatus` but have NO `instrumentationStatus` field. Adding one would let the ranker surface shipped-but-unmeasured as its own gap class alongside shipped-without-screen-map.

## 2026-07-23 action-list items: 0 of 6 landed

| Action | 2026-07-23 state | 2026-10-09 state |
|---|---|---|
| Backfill dispute add_evidence audit event (gated by #2483 / PR #2490) | pending #2490 | #2490 still open; action still open |
| Define layout publish/webhook analytics events | queued | not promoted |
| Define dispute-lifecycle KPI set | queued | not promoted |
| Instrument announcement fan-out (delivered/read/ack per scope) | queued | not promoted |
| Publish data-retention policy for support-data / audit trail | queued (DEC-005 carried) | not promoted |
| Formalize support-staff read audit event schema | queued (DEC-005 carried) | not promoted |

Root cause: dispatcher capacity is being spent on infra unblocks (#2966) and reviewer slots for the accounting trio; pm-data items are structurally always `medium` and get crowded out by `high` items. Recommendation (below) is to collapse the six into a single **analytics-schema-foundations** meta-task so it can be scored above `medium` once as a batch rather than six medium items that never win a slot.

## Next actions

| Action | Priority | Dependency | Definition of done |
|---|---|---|---|
| Define a cross-platform `funnel_step` + `funnel_fail` event schema (publish-listing / submit-report / group-join / sync-schedule) — one shared taxonomy covering reality-web + ppt-web + mobile; emit the four events above in follow-up PRs | high | pm-tech-lead (analytics platform decision still open) | schema + emission in /sell, /report, groups/community, SyncSchedule; one dashboard panel per funnel; documented in docs/data/ |
| Collapse the six 2026-07-23 pm-data actions into one `analytics-schema-foundations-2026-10-09` meta-task so it ranks as `high` (phase_weight 4 + 2 risk + 1 dep) and can win a dispatcher slot | high | none | single action-list row replacing the six; prior items marked `done:collapsed-into:` with the new id |
| Add `instrumentationStatus` to the screen-map frontmatter schema (values: none/partial/complete) and backfill 10 highest-trafficked screens | medium | pm-tech-lead (screen-map schema owner) | screen-map schema docs updated; 10 screens backfilled; `/screens validate` passes |
| Decide analytics platform (bespoke pg aggregates vs Amplitude/PostHog/Segment) — carried from 2026-07-23 open decision; blocks every emission wiring | high | pm-tech-lead | DEC entry logged in decisions.md with chosen platform + migration/pilot plan |
| Publish retention policy for support_tooling_events + append-only audit tables (carried from 2026-05-28, 2026-07-23) — now blocks dispute/OAuth/layout audit-event expansion | medium | pm-security (GDPR classification) | policy doc merged; PII-carrying tables get lifecycle SQL jobs |
| Audit mobile-native + mobile-rn event-tracking parity with web — baseline measurement before new instrumentation lands so we know where the gap closes | medium | pm-frontend | audit report under docs/data/mobile-event-parity-2026-10.md; delta list captured in action-list |

## Risks

| Risk | Probability | Impact | Mitigation |
|---|---|---|---|
| Analytics-event emission schema shipped per-feature without a shared taxonomy — every new event becomes incompatible with the next dashboard; retro-fitting costs 10x | high | medium | Land `funnel_step` + `funnel_fail` schema first (above); require schema reference in every emission PR |
| Analytics platform decision stays unresolved — bespoke pg aggregates will accumulate until migration cost becomes prohibitive | high | medium | Force the DEC entry this cycle; even "bespoke pg for 6 months then re-evaluate" is better than open-ended |
| Shipped bugfix wave 10-08 proves our bug-detection is reviewer-driven not metric-driven — the silent-publish-405 defect would have been caught in minutes by a publish-success KPI | high | medium | Instrument publish/submit funnels as the first slice; measure MTTR delta on next bugfix wave |
| Retention policy still unpublished — append-only tables will cross GDPR-retention threshold without triggering any alarm | medium | high | Attach retention publication to the next support-data expansion PR as a blocking checklist item |
| Screen-map has no `instrumentationStatus` field — the "which shipped screens have no analytics?" query cannot be answered mechanically; relies on human recall | medium | medium | Schema extension (above); auto-populate from code scan where feasible |

## Open questions

- Does the pm-security retention/GDPR position on `support_tooling_events` (2026-05-28 → 2026-07-23 open decision) have an owner-of-record yet? If not, pm-security + pm-data joint ownership is the fallback.
- Should the `funnel_step` schema include a `tenant_id` dimension, or is tenant-level dashboarding deferred to a later slice (privacy/complexity tradeoff)?
- For the collapsed `analytics-schema-foundations` meta-task — is pm-frontend or pm-backend the right owner_role on the dispatcher's claim side? (Backend emits; frontend emits web vitals.)
- Do we have any customer contract or regulatory obligation that depends on dispute-lifecycle KPIs, or is this still purely product-internal (carried from 2026-07-23)?
- Is there any existing dashboard tool configured against the staging/prod Postgres that could be the pilot target before an analytics-platform decision lands?

## Decisions needed

- Analytics platform choice (bespoke pg vs Amplitude/PostHog/Segment) — owner: pm-tech-lead + pm-data (carried from 2026-07-23; now blocking)
- GDPR / retention policy for `support_tooling_events` (TTL vs indefinite) — owner: pm-security + pm-data (carried from 2026-05-28)
- Minimum-analytics DoD for new stories (blocking gate or advisory?) — owner: pm-scrum-master + pm-data (carried from 2026-07-23)
- FaultStatusCount canonical definition — owner: pm-data (carried from 2026-05-28; now 136 days open)
- Does the dispatcher ranker get an explicit "collapse low-ranked pm-data items into a single meta-task" rule, or do we keep the manual pattern? — owner: pm-tech-lead
