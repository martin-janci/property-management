# pm-scrum-master

<sub>Last ran: 2026-11-20</sub>

## Summary

Sprint spine is fully done (6/6 epics, coverage.json 49/49 done, so no gap-driven stories remain). Delivery this run was a hardening wave: 16 merged code-review/churn PRs, mostly security. The pipeline is the constraint: dispatcher stalled (#3357), KMP/swagger egress unclaimable in cloud (#2652/#2966), and the sprint-status test_hardening_batch still lists open items.

## Sprint progress

- Sprint: Epic 6, 7A, 8A & 10A - Announcements, Documents, Notifications & OAuth
- Epics done: 6 / 6

## Shipped since last run

- #3647 shared per-hop SSRF redirect revalidation guard (reality-server import fetch)
- #3648 bind HelloSign webhook envelope to verified provider
- #3646 runtime 401 token refresh for generated API client (ppt-web)
- #3645 guard SSO error_description decode against URIError
- #3644 gate raw ErrorBoundary message behind DEV
- #3643 handle useAgencyBranding fetch error (reality-web)
- #3642 regression test for cross-tenant rental connection IDOR
- #3641 remove dead unscoped rental IDOR-footgun repo methods
- #3640 surface swallowed idempotency commit error
- #3639 throttle vote reminders to ~5-min minimum
- #3631 echo clamped limit in compliance audit-log pagination
- #3630 WidgetSettingsScreen copy routed through i18n (mobile RN)
- #3628 derive suggestions seed region from request (follow-up bug #3649: region de resolves to Wien)
- #3632 / #3633 characterization tests for notification role-default JSONB and digest config parsing (churn hotspots)
- #3638 backfill shipped-story KPI definitions

## Blockers

- **Dispatcher pipeline (#3357)** — 3 approved PRs blocked on non-mechanical conflicts; pipeline fully stalled (owner: `pm-tech-lead`)
- **KMP/mobile-native backlog items (inquiries-response-contract, create-listing-not-wired, http-client-no-timeout, sso-error-body-unguarded)** — Gradle/KMP builds unavailable in cloud runner (#2652, #2966); claimable only ~9/72 (owner: `pm-devops`)
- **Dependabot bumps #3634-#3637, #3441, #3635 (gradle wrapper)** — Open and unmerged; gradle/kotlinx bumps cannot be validated in cloud (owner: `pm-devops`)

## Next actions

- **[high]** Resolve dispatcher stall #3357: rebase or manually resolve the 3 approved conflicting PRs, merging security-flavored ones first — dep: pm-tech-lead — DoD: 3 approved PRs merged or re-queued; #3357 closed
- **[high]** Sequence listings.rs PRs: rebase #3617 (record_view existence + per-IP throttle) and merge before #3628 follow-up; fix #3649 so region de does not seed Wien — dep: pm-backend — DoD: #3617 merged; #3649 fixed or explicitly scoped
- **[high]** Unblock KMP and swagger-ui egress in the cloud runner (#2652, #2966) — dep: pm-devops — DoD: KMP backlog items claimable; claimable ratio above buffer threshold
- **[high]** Sweep violations mutation handlers (moderation.rs/aml_dsa.rs) for the 403-vs-404 cross-tenant contract (assign still returns 403 per #3187) — dep: pm-security — DoD: All violation mutation handlers return 404 cross-tenant; regression tests added
- **[medium]** Reconcile sprint-status test_hardening_batch (items #480-#487 still marked open, batch in-progress) against GitHub issue state and close or defer — dep: none — DoD: Batch status done, or each open item has a deferral note
- **[medium]** Triage churn hotspots (oauth_integration_tests.rs, voice_webhooks.rs runs_seen=5, push_fanout.rs runs_seen=4) into refactor or characterization backlog items; carry on #3650 (retry-after draft) and reality-web preview i18n items — dep: pm-tech-lead — DoD: Backlog items created for each hotspot; #3650 out of draft

## Risks

- **high/high** — Dispatcher stall (#3357) leaves approved PRs unmerged and review debt growing (stalled-review #3380 8d)  _mitigation:_ Manual conflict resolution; prioritize security PRs
- **high/medium** — Cloud runner cannot build KMP or fetch swagger-ui, so mobile-native and some backend security work is unclaimable  _mitigation:_ Fix egress (#2652/#2966) or route such items to a local/self-hosted runner
- **medium/medium** — Persistent churn hotspots (voice_webhooks.rs 5 runs, push_fanout.rs 4 runs) signal unstable design and rework  _mitigation:_ Dedicated refactor plus characterization tests before further feature edits
- **medium/high** — Open cross-tenant 403-vs-404 inconsistency and IDOR-class patterns recurring (rental IDOR needed both test and dead-code removal)  _mitigation:_ Pattern audit across repos; add cross-tenant regression tests per route
- **medium/low** — 6 open dependency bumps pile up (npm 25 updates, cargo 5, gradle wrapper, kotlinx-serialization) and may conflict with each other  _mitigation:_ Merge npm/cargo bumps after CI green; batch gradle bumps when KMP CI is available

## Open questions

- Is the sprint goal (Epics 6/7A/8A/10A) still the active sprint, given every epic and coverage story is done? A new sprint definition or a scan for new stories is needed.
- Which 3 approved PRs are blocked in #3357, and who owns the non-mechanical conflicts?
- Are test_hardening_batch issues #480/#481/#482/#487 actually closed? Entries at lines 140/170/181 of sprint-status.yaml still read status: open, while story notes say gates are cleared.
- Should the suggestions seed-city for region de (#3649) map to a German city, or is it a deliberate fallback?
- Is #3616 ListingSummary slug propagated to OpenAPI and the generated reality-api-client and KMP/iOS clients?

## Decisions needed

- Define the next sprint and goal, since coverage is 49/49 done and the current sprint is complete — owner: pm-product
- Fund or approve runner egress changes for KMP and swagger-ui (#2652/#2966) versus deferring the mobile-native backlog — owner: pm-devops
- Order of merge for listings.rs PRs (#3617 before #3628 follow-up) — owner: pm-tech-lead
