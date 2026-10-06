# Role: pm-data — 2026-10-06

> Data/analytics lens. Rotating role this run (pm_cursor rotation[6]). Static read-only. Previous run: 2026-07-23 (75 days ago).

**Summary:** Half of the 6 PRs merged this 72h window are the same data-pipeline hygiene bug: **the UI collected input, but the input never reached the network (or a failure never reached the user).** All three were spotted by churn-hotspot review, not by any test. The frontend has no primitive that compares "what fields the form intends to submit" against "what the request body actually carries." This class will recur on every new wizard until a data-contract gate exists. Meanwhile all pm-data risks carried from 2026-07-23 are unchanged: Epic 6/10A/10B/80/84 lack KPI instrumentation, no analytics event emission assertion helper exists, FaultStatusCount definition drift still open, support_tooling_events still has no TTL/retention policy, mobile event parity still unaudited.

## This run's data signal

### Silent-dropped-input / silent-failure cluster (3 of 6 merged PRs)

| PR | Shape | Latent since | Fix |
|---|---|---|---|
| #3019 (Closes #3017) | /report attachment control had no payload field | unknown — feature shipped weeks ago | Remove the dead control |
| #3021 | PriceAlerts mark-read swallowed backend errors | unknown — feature shipped at #2287 | Surface error toast |
| #3022 (Closes #3016) | /sell wizard collected seller-contact fields the API no longer accepts | API changed, UI did not | Drop the dead fields |

**Analytics gap.** We cannot answer "how many /report submissions silently dropped an attachment?" or "how many PriceAlerts mark-read requests actually failed?" from telemetry — there is no event stream carrying the submitted payload shape or the response status for user-facing fetches. This is the same pattern pm-data called out on 2026-07-23 for layout publish / dispute lifecycle.

### Reviewer-feedback corpus (dark data)

`assignments.json` currently holds 12 open items with structured reviewer notes. Three recurring patterns show up:

| Pattern | Count | Examples | Signal |
|---|---|---|---|
| `scope-drift` | 2 | #2902, #3024 | PRs touched `.research/**` or docs beyond stated scope |
| `self-approve blocked bot==author` | 4 | #2997, #3025, #3026, #3028 | Approved via COMMENT, waiting on human merge |
| `screen-map stale` | 1 | #3024 | Note content outdated vs shipped surface |

Multiply by the 100+ archived entries in `assignments-archive.json` and this is a usable longitudinal corpus. **Nothing counts it today.**

## Cross-reference to shipped test-backfill wave (carried from 2026-07-23)

Still true: the 2026-07-22 test wave (BIT-268/BIT-557/BIT-559 in PRs #2447/#2453/#2465) verified code paths execute but did NOT add analytics events, and this week's three silent-dropped-input bugs show the hygiene gap is still latent on new verticals.

- **Missing metrics for disputes epic 80:** no filed/mediation/resolved funnel, no TTR percentiles, no evidence-per-dispute counter.
- **Missing metrics for layout epic:** no `published_by`, `layout_version`, `target_tenant_count` events on the publish path.
- **Missing data-contract tests on reality-web wizards:** demonstrated by #3017 + #3016 + the #3021 class.

## Next actions

| Action | Priority | Dependency | Definition of done |
|---|---|---|---|
| Author a vitest data-contract-fuzz helper under `frontend/packages/testing-helpers`; mount form, submit against msw, diff-assert form-schema vs captured request body; pilot on reality-web /sell + /report | medium | pm-qa | Helper merged; both pilot pages covered; CI gate blocks new reality-web wizard pages without a contract test |
| Build a daily jq-based bucket counter over `assignments.json` + `assignments-archive.json`, emitting a top-N reviewer-note classifier (scope-drift / i18n / screen-map-stale / self-approve-blocked / cloud-build-blocked / rebase-needed); 1-line summary into project-state.md | low | none | Script + daily cron entry; sample output included in next pm-data rotation |
| Draft a minimum-analytics DoD policy proposal for pm-scrum-master: new stories merging a user-visible surface emit >=1 named analytics event + schema doc; advisory for 1 sprint, then blocking | medium | pm-scrum-master | 1-page policy merged; story template updated; retro checkpoint at next sprint |
| Backfill dispute add_evidence access-audit event once #2483/PR #2490 lands (parity with support-data audit_read) | medium | pm-security | audit_write event emitted on evidence upload; visible in SupportDataPage |
| Define layout publish/webhook analytics events (published_by, layout_version, target_tenant_count) — layout epic shipped end-to-end with zero KPI hooks | medium | none | event schema + emission wired in publish_layout handler |
| Define dispute-lifecycle KPI set (filed->mediation->resolved funnel, TTR percentiles, evidence-per-dispute) — Epic 80 all-done, no dashboard | medium | pm-scrum-master | metric definitions + counters + p50/p95 TTR emitted |
| Publish data-retention policy for support-data / analytics events / audit trail — append-only `support_tooling_events` has no TTL | medium | pm-security (GDPR classification) | policy doc merged; PII-carrying tables get lifecycle SQL jobs |
| Formalize support-staff read audit event schema — carried from 2026-05-28 | medium | pm-security | event schema + emit at all support-data read/revoke sites |

## Risks

| Risk | Probability | Impact | Mitigation |
|---|---|---|---|
| reality-web ships wizards that silently drop user input — same shape as #3017/#3016/#3021 — no end-to-end data-contract assertion | high | medium | Land the vitest data-contract helper + CI gate before next reality-web wizard merges |
| Reviewer-feedback corpus is growing (dispatcher writes it, nothing reads it) — trend data stays dark | medium | low | Bucket counter script |
| Shipped MVP features (Epic 6/10A/10B/80/84) lack KPI instrumentation — product decisions blind on exactly the features that just went live | high | medium | Sequence pm-data KPI tasks; minimum-analytics DoD |
| FaultStatusCount vs owner/portfolio fault KPIs diverge (carried from 2026-05-28) — dashboards disagree | high | medium | Single-source-of-truth definitions; deprecate duplicates |
| `support_tooling_events` + audit trail have no TTL — long-term storage + GDPR risk | medium | medium | Retention policy; lifecycle jobs if PII-carrying |
| Test-backfill wave proves code paths execute but does NOT verify analytics events fire — silent metric drift possible | medium | low | Analytics-event assertion helper |
| Mobile (RN + KMP) event tracking parity with web unknown — funnels blind on ~50% of traffic | medium | medium | Audit + backfill (action carried from 2026-07-23) |

## Open questions

- What KPI dashboard tool does PPT use for internal metrics — Grafana over Postgres, a third-party (Amplitude/PostHog/Segment), or bespoke platform-admin pages?
- Are dispute lifecycle KPIs required by any customer contract / regulatory obligation, or purely product-internal?
- Is there a Data Protection Impact Assessment on `support_tooling_events` (support staff reading tenant data)?
- For webhook events (booking / airbnb / esignature / layout), do we emit analytics on delivery + retry + failure, or only log?
- Are the seed-data recipes stable enough to reason about analytics test fidelity (pm-data DoD depends on repeatable seeds)?

## Decisions needed

- Analytics platform choice (bespoke vs Amplitude/PostHog/Segment) — owner: pm-tech-lead + pm-data
- GDPR / retention policy for `support_tooling_events` (TTL vs indefinite) — owner: pm-security + pm-data
- Minimum-analytics DoD for new stories (blocking gate or advisory?) — owner: pm-scrum-master + pm-data
- FaultStatusCount canonical definition (support-data vs owner/portfolio KPI) — owner: pm-data (carried from 2026-05-28)
- Data-contract test gate for new reality-web wizards — advisory or CI-blocking? — owner: pm-qa + pm-data
