# Role: pm-scrum-master — 2026-07-31

> Delivery lead / coordinator. Always runs. Static read-only.

## Summary

Strong 1-day cadence — 21 PRs merged since the 2026-07-30 run (mostly auto-review followup + code-review autofixes), and **both same-window regressions from the previous run are already closed**: #2597 guards DELETE-by-file-key (closes #2573 data-loss), #2593 wires Android SSO mint() (closes #2574 CSRF blocker). The auto-fix loop is now completing its own regression cycles inside 24-48h. Only one new follow-up remains open (#2575 dispute-KPI window validation), the accounting MVP-loop trio (#2555/#2558/#2559) is still starving on reviewer capacity (3 days now, dependabot noise not converting to human review). Phase 1.5 code review surfaced 3 high-severity voice_webhooks security holes that are architectural — unit tests (PR #2604) do not fix them.

## Sprint progress
- Sprint: **Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth**
- epics_done: **3 / 5** (unchanged); coverage remains 47/49 done, 2 partial (84-1, 84-2) — but both partial stories are now unblocked and the 84-1 blocker chain (#2573) is closed.

## Shipped since last run (21 PRs)
- **#2609** code-review-api-core-resolved-rs-leaks-db-error: stop leaking raw sqlx/serde error text on public GET /layout/resolved
- **#2608** code-review-api-core-scheduler-rs-silent-target-err: surface DB errors on scheduler notification-target lookups
- **#2607** chore(api-validation): add reality-api-client drift gate (closes #2556)
- **#2606** code-review-api-core-admin-rs-swallowed-serialize: return 500 on failed layout serialize instead of null body
- **#2605** dx-stale-todo-security-comments-faults-critical-notifications: remove stale TODO(security) gate comments
- **#2604** test-gap-voice-webhooks-zero-coverage: unit tests for voice_webhooks.rs (does NOT fix the 3 security findings)
- **#2603** code-review-reality-web-viewsource-untrusted-cast: validate untrusted ?source= against ViewSource set
- **#2602** gh-issue-2598: fix pre-existing frontend test failures blocking the verify gate (admin-web / reality-web)
- **#2601** dx-admin-web-platform-settings-mobile-config-permanent-noop: fix no-op Save paths
- **#2600** code-review-reality-web-inline-tenant-json-xss: escape inlined tenant-config JSON
- **#2599** refactor-churn-hotspot-api-server-reports-2026-07-27: extract schedule cadence/cron helpers from reports.rs (new 889-line module)
- **#2597** gh-issue-2573: guard DELETE /documents/by-file-key against reaping a still-referenced object (closes #2573 — data-loss blocker)
- **#2596** code-review-ppt-web-aml-review-decision-untrusted-cast: fix(ppt-web) validate AML review decision before submit
- **#2595** code-review-mobile-native-kmp-portfolio-analytics-drops-view-zero-days: keep days with inquiries but zero views
- **#2594** bug-hotfix-no-test-pr-2547: test(api-server) DB-backed regression for support_tooling_events retention prune
- **#2593** gh-issue-2574: fix(mobile-native) wire Android SSO initiation leg (mint CSRF nonce) — closes #2574 SSO blocker
- **#2592** code-review-ppt-web-core-ws-ungated-console: gate WebSocket console diagnostics behind import.meta.env.DEV
- **#2580** refactor-churn-hotspot-platform-admin-authz-batch2-2026-07-23: de-duplicate platform_admin authz batch2 tests
- **#2579** refactor-churn-hotspot-org-property-authz-backfill-2026-07-23: de-duplicate org-property authz backfill tests
- **#2578** code-review-ppt-web-core-ws-token-rotation-stale: re-auth WebSocket on token rotation
- **#2577** screen-map-drift-pr-2497-reality: reconcile listing-detail screen-map with layout-revalidate hardening

## Next actions
1. **[high]** Land 84-1 direct-to-S3 wiring in ppt-web (POST /documents/upload-url consumer) — blocker #2573 now cleared by #2597 — owner: **pm-frontend** — DoD: 84-1 flips partial→done, regression test covers register→PUT→confirm + orphan expiry.
2. **[high]** Fix the 3 voice_webhooks security holes (Phase 1.5 code review): cross-tenant auth bypass in `authenticate_voice_user`, HMAC default-secret fallback + non-constant-time compare, Alexa signature never verified. PR #2604 added tests but the endpoints remain effectively unauthenticated in production — owner: **pm-security / pm-backend** — DoD: 3 findings closed with regression tests; secret required (no default), Alexa signature actually verified, per-caller tenant scoping enforced.
3. **[high]** Address #2575 — `/disputes/kpis` has no window-ordering validation, its only test is quarantined (from PR #2572). Only unclosed same-window follow-up — owner: **pm-backend** — DoD: reject window_end < window_start with 400; un-quarantine the KPIs test; add reporting-consumer contract test.
4. **[medium]** Break the accounting MVP-loop trio reviewer starvation (#2555/#2558/#2559 — sitting 3 days now). Options: (a) explicit reviewer slot; (b) split into smaller PRs; (c) re-scope. — owner: **pm-tech-lead** — DoD: at least one of the three either merged or explicitly re-scoped.
5. **[high]** Build 84-2 signer-facing document-sign page in ppt-web — retry-3/2 pool now exhausted; needs specialist attention — owner: **pm-frontend** — DoD: page shipped, signature-request email E2E verified, screen-map ppt/document-sign `buildStatus: planned → shipped`.
6. **[medium]** Backfill screen-map `epics:` frontmatter across `docs/screens/**` — coverage-scan orphan detector is manufacturing false `screen_refs: []` on 5 epic-7a shipped stories. One-PR fix with high leverage — owner: **pm-frontend** — DoD: 5 epic-7a stories gain `screen_refs`; coverage `screen_gaps` shrinks.

## Risks
- **[high prob / high impact]** Voice-webhook security cluster (Alexa signature never verified, HMAC default-secret with non-constant-time compare, cross-tenant auth bypass) — 3 P0 findings from Phase 1.5 static review. PR #2604 (unit tests) does NOT fix them — the tests exercise the current broken authz. Any voice-provider signature can currently reach production endpoints unauthenticated. Mitigation: land a proper security-focused hardening PR (owner pm-security); do not surface any voice feature in UI until closed.
- **[med prob / med impact]** Accounting MVP-loop trio (#2555/#2558/#2559) has been sitting 3 days with zero reviewer engagement — dispatcher-stack starving on reviewer capacity. Trigger `buffer-low: claimable=6/72` in this run's payload is a symptom of this. Mitigation: reviewer-slot rotation policy for large-scope feature PRs; consider splitting invoice-lifecycle PR (largest of the trio).
- **[med prob / med impact]** 84-2 signer-page retry_3/2 pool is now exhausted — a third failed no-PR would strand this story. Mitigation: tighter scope prompt, or split shell+flow.
- **[low prob / med impact]** New `schedule_cadence.rs` extraction (PR #2599, 889 lines) is fresh and untested-in-context — a subtle behavior drift in `reports.rs` cron scheduling could regress silently. Mitigation: add a targeted regression test on `reports.rs` → `schedule_cadence.rs` round-trip during next backend rotation slot.
- **[med prob / low impact]** Buffer refill mechanism is currently the constraint on dispatcher throughput (6/72 claimable), not implementer capacity. Mitigation: this Phase 1.6 run refills the buffer via role next-actions and the ranker; Phase 2 backlog refill supplements.

## Open questions
- Should the accounting MVP-loop epic be folded into `coverage.json` (currently outside the 13-epic set), given it now has 3 open PRs blocking dispatcher progress?
- Should the freshly-extracted `schedule_cadence.rs` (PR #2599) become its own coverage story so future churn scans anchor to a shipped-and-tested target rather than a bare file?
- With #2573 and #2574 both closed inside 24h of surfacing, is the auto-review→auto-fix cycle now formally acceptable as the delivery model, or does it need explicit gate criteria (e.g., no more than N same-window regressions)?

## Decisions needed
- Reviewer-slot policy for large-scope feature PRs (accounting trio blocking 3 days) — owner: **pm-tech-lead**.
- Voice-webhook security triage: hotfix branch vs feature branch — the 3 findings are architectural but PR #2604 has already merged tests around the broken code; a fix must un-quarantine or re-write those tests — owner: **pm-security / pm-tech-lead**.

## Blockers
- **Voice-webhook security cluster (Phase 1.5 findings)** — 3 P0 findings, no PR opened yet; endpoints are effectively unauthenticated in production. Owner: **pm-security / pm-backend**.
- **Accounting MVP-loop trio (#2555 / #2558 / #2559)** — 3-day reviewer starvation; blocks the accounting story pipeline entirely. Owner: **pm-tech-lead**.
- **84-2 signer-page retry_3/2 pool exhausted** — needs specialist re-scope before another dispatcher attempt. Owner: **pm-frontend / pm-tech-lead**.
