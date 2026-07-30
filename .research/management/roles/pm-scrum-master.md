# Role: pm-scrum-master — 2026-07-30

> Delivery lead / coordinator. Always runs. Static read-only.

## Summary
Very productive 2-day window: 17 PRs merged (post-merge-review batch of 2026-07-28 is now fully closed except #2528 booking-webhook parity), plus 3 fresh follow-up issues opened on this window's own PRs (#2573 DELETE-by-file-key regression, #2574 Android SSO CSRF half-wired, #2575 dispute KPI window validation). Coverage still at 47/49 done; the two long-standing 84-x frontend partials are unchanged.

## Sprint progress
- Sprint: **Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth**
- epics_done: **3 / 5**

## Shipped since last run (17 PRs)
- PR #2576 gh-issue-2563: schedule layout_change_events retention prune
- PR #2572 gh-issue-2562: wire get_dispute_kpis into a reporting endpoint
- PR #2571 gh-issue-2564: org-scoped DELETE-by-file_key for direct-upload orphan cleanup
- PR #2570 gh-issue-2557: dedupe private seed_org/set_ctx in db test suites
- PR #2569 dx: run SDK drift gate on client + workflow changes
- PR #2568 code-review mobile-native-kmp: Android SSO CSRF state verification
- PR #2567 code-review api-core: clear scheduler global-read RLS GUC before pool return (retry1)
- PR #2566 gh-issue-2561: version-bump rebase+retry to fix GH006 on concurrent dev merges
- PR #2565 gh-issue-2560: reality-web Docker build fix (api-client node_modules in builder stage)
- PR #2554 chore(research): refill starved dispatcher stack (7 new vectors, 14 promoted)
- PR #2553 code-review ppt-web-core: AuthContext cold-boot routes through refreshTokenInternal (stale-role fix)
- PR #2549 gh-issue-2532: layout publish/webhook/revalidate event emission + sink
- PR #2504 fix(api-server): signature-request list/create — mount as document sub-resource (BIT-313)
- PR #2491 chore(deps): npm-minor-patch group (5 updates)
- PR #2482 refactor: reconcile docs/repo-map.md with current tree
- PR #2478 fix(layout): review-hardening sweep (authz, publish TOCTOU, webhook replay, defensive rendering)
- PR #2433 feat(mobile-native): iOS listing detail renders through the shared resolved layout

## Next actions
1. **[high]** Address #2573 — DELETE /documents/by-file-key can delete a still-referenced object within the same org (regression from PR #2571) — owner: pm-backend — DoD: reference-check added before delete; regression test covering shared-file-key same-org case.
2. **[high]** Address #2574 — Android SSO CSRF guard half-wired (SsoStateStore.mint() has no call site so every reality://sso callback is rejected) — owner: pm-mobile / react-native — DoD: mint() wired at deep-link entry; integration test covers the happy path.
3. **[medium]** Address #2575 — /disputes/kpis has no window-ordering validation, only test is quarantined (PR #2572) — owner: pm-backend — DoD: reject window_end < window_start with 400; un-quarantine the KPIs test.
4. **[medium]** Merge or triage the accounting MVP-loop trio (#2555 invoice lifecycle, #2558 invoice PDF, #2559 PAY by square QR) — all draft-ready since 2026-07-28 — owner: pm-backend / pm-tech-lead — DoD: reviewed + green + merged, or explicitly re-scoped.
5. **[high]** Finish long-standing 84-1 direct-to-S3 wiring in ppt-web (POST /documents/upload-url consumer) — owner: pm-frontend — DoD: api-client binding + UploadDocument integration + regression test.
6. **[high]** Finish long-standing 84-2 signer-facing document-sign page (screen-map planned→shipped, API complete) — owner: pm-frontend — DoD: page shipped, signature-request email delivery verified end-to-end.

## Risks
- **[med prob / high impact]** PR #2571 (DELETE-by-file-key) landed with a same-org reference-check gap (#2573) — an active same-org file key can be deleted out from under a live document row — mitigation: land a reference-count guard + integration test before any client wires the endpoint.
- **[high prob / high impact]** PR #2568 CSRF state fix is non-functional (#2574) — mint() has no call site so every SSO deep-link is rejected — mitigation: wire mint() at the SSO deep-link entry point (fresh subagent on the mobile-native slice).
- **[med prob / med impact]** Accounting MVP-loop trio (3 open PRs) has been sitting 2 days with no reviewer engagement — dispatcher stack starving on reviewer capacity, not implementer capacity — mitigation: explicit reviewer slot for the trio next 24h; document reviewer-slot rotation for large-scope PRs.

## Open questions
- Should the accounting MVP-loop epic be added to coverage.json (currently outside the 13-epic set)?
- Should the layout epic (scheduler.rs + tenant.rs + admin.rs are top churn this window) be promoted to its own coverage epic entry?

## Decisions needed
- Reviewer-slot policy for large-scope feature PRs (accounting trio blocking) — owner: pm-tech-lead.

## Blockers
- **#2574 Android SSO CSRF half-wired** — the freshly-merged CSRF fix (#2568) has no call site — every reality://sso callback is now rejected; blocks any Android SSO usage until re-wired — owner: pm-mobile.
- **#2573 DELETE-by-file-key same-org reference gap** — new endpoint can delete a still-referenced S3 object within the same org (regression from PR #2571); blocks safe client wiring for 84-1 direct-to-S3 — owner: pm-backend.
- **Accounting trio (#2555 / #2558 / #2559)** — no reviewer engagement in 2 days; dispatcher can't advance the MVP-loop — owner: pm-tech-lead.
