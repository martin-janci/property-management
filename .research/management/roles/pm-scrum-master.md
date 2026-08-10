# Role: pm-scrum-master — 2026-08-10

> Delivery lead / coordinator. Always runs. Static read-only.

## Summary
Huge 3-day window: **22 PRs merged, 3 tracked issues CLOSED** (#2703 SSRF DNS-rebinding TOCTOU via #2710, #2704 unbounded-response DoS via #2707, #2612 notification durability watermarks via #2714) and the dispatcher's auto-fix + post-merge-review loops are visibly retiring the security backlog (reality-server auth batch — #2725 password-reset transport, #2726 SSO session invalidate swallowed, #2727 agency-members unauth IDOR — all cleared this window). Coverage still at **47/49 done, 2 partial** (the 84-1 direct-to-S3 wiring and 84-2 signer page). Dispatcher buffer is dry (1/72 open on the dispatcher side, 2/36 open locally) — the top priority this run is refilling the action-list backlog.

## Sprint progress
- Sprint: **Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth**
- epics_done: **3 / 5** (extended coverage across 13 epics: 47/49 stories done, 2 partial)

## Shipped since last run (22 PRs > #2702)
- **#2727** code-review-reality-server-agency-members-unauth-idor-retry1
- **#2726** code-review-reality-server-sso-session-invalidate-swallowed
- **#2725** code-review-reality-server-password-reset-no-transport
- **#2724** code-review-reality-server-db-error-leak (util::errors::db_error routing)
- **#2723** data-announcement-fanout-metric-2026-07-23-retry2 (announcement fan-out metrics — closes pm-data KPI item)
- **#2722** code-review-api-handlers-community-unauthenticated-reads-retry2 (community read gate)
- **#2721** refactor-churn-hotspot-api-server-layout-tenant-2026-07-30 (acquire_public_conn extract)
- **#2720** refactor-churn-hotspot-api-server-reports-2026-07-31 (reports helpers.rs)
- **#2719** code-review-reality-server-inquiry-notify-route-wiring
- **#2718** sec-layout-webhook-hmac-verify-2026-07-23-retry2 (HMAC parity test — closes #2485 mitigation)
- **#2717** dx-fixme-admin-web-mobile-config-patch-endpoint-retry1
- **#2716** dx-fixme-admin-web-platform-settings-patch-endpoint-retry1
- **#2715** refactor-churn-hotspot-api-server-auth-2026-07-31 (auth error boilerplate dedupe -83 lines)
- **#2714** gh-issue-2612-retry1 (notification durability, watermarks — Closes #2612)
- **#2713** refactor-churn-hotspot-api-server-layout-admin-2026-07-30 (dedupe -108 lines)
- **#2712** data-audit-add-evidence-idor-fix-2026-07-23-retry2 (dispute audit event — closes #2483 follow-up mitigation)
- **#2711** refactor(api-server) layout tenant-override dedupe
- **#2710** gh-issue-2703 (SSRF DNS-rebinding TOCTOU closed via connect-time resolver — Closes #2703)
- **#2709** code-review-reality-web-listingform-no-i18n (i18n via next-intl)
- **#2708** workflow_executor non-finite compare guard
- **#2707** gh-issue-2704 (api_call 8MiB body cap DoS — Closes #2704)
- **#2706** RAG partial-batch fail-closed

Closed-not-merged: **#2705** (dtolnay/rust-toolchain 1.100.0 nonexistent — invalid dependabot bump; triage as `pm-devops` follow-up).

## Next actions
1. **[high]** Refill dispatcher action-list to 36 open items — buffer is at 1/72 dispatcher-side and 2/36 locally after the huge burn window — owner: **pm-scrum-master** — DoD: ≥ 36 open items in `.research/management/action-list.json` after this run.
2. **[high]** Finish long-standing **84-1** direct-to-S3 wiring in ppt-web (POST /documents/upload-url consumer + UploadDocument integration) — owner: **pm-frontend** — DoD: api-client binding + integration test; ppt-web upload flow no longer proxies through server.
3. **[high]** Finish long-standing **84-2** signer-facing document-sign page (screen-map planned → shipped, API complete) — owner: **pm-frontend** — DoD: page shipped, signature-request email delivery verified end-to-end.
4. **[high]** Un-quarantine `/disputes/kpis` test + add `window_start <= window_end` validation (follow-up #2575, still open after 3-day window) — owner: **pm-backend** — DoD: 400 on inverted window, un-quarantined test asserting reporting-consumer contract.
5. **[medium]** Shepherd the 4 human-authored open PRs — **#2684** workflow-cond-parse-failopen, **#2559/#2558/#2555** accounting invoice/QR trio — draft-ready for 13+ days, zero reviewer engagement — owner: **pm-tech-lead** — DoD: each PR has a named reviewer + first review round within 48h.
6. **[medium]** Land Alexa voice webhook signature verification (`verify_alexa_signature` currently never checks the signature — carried) — owner: **pm-security** — DoD: HMAC verify + integration test forcing a bad signature to be rejected.

## Risks
- **[med / high]** **Dispatcher backlog starvation**: dispatcher-side open buffer is at 1/72 slots — implementer capacity is idling while gap candidates go un-ranked. Cause: post-scan candidates weren't merged into `action-list.json`. Mitigation: this run's refill (36 items surfaced), plus one-time recheck that ranker mapping is wired.
- **[med / medium]** **Reviewer starvation on human PRs**: 4 non-dispatcher PRs (#2684 human workflow fix, #2555/#2558/#2559 accounting trio) have been open ≥ 13 days with zero engagement. The bot loop is eating all review bandwidth. Mitigation: enforce the 2026-07-30 reviewer-slot policy (already decided).
- **[low / high]** **84-1 / 84-2 partial staleness**: both stories are frontend-only against shipped backend APIs; the fact that they've sat as `partial` across 4 runs suggests owner assignment is not sticking. Mitigation: explicit pm-frontend claim + deadline in next dispatcher cycle.
- **[high / medium]** **Churn continues on reality-server/src/state.rs** (1201 lines, top hotspot this run) and reality-server/src/routes/agencies.rs (624 lines) — no split plan yet. Mitigation: pm-tech-lead drafts a split proposal (mirrors the api-server/layout/auth/reports dedupe work that shipped this window).

## Open questions
- Is the pm-data announcement fan-out metric (#2723) considered sufficient to close the `pm-data-announcement-fanout-instrumentation` action, or is there more scope (per-scope delivered/read/ack breakdown)?
- Should the reality-server churn-hotspot dedupe work follow the same pattern used for api-server (routes/layout, routes/reports, auth error boilerplate) that just shipped, or is a different module boundary needed?
- Are the 12 open dependabot PRs blocking anything (do we need a batch merge slot on the routine)?

## Decisions needed
- **NEW (2026-08-10, pm-scrum-master):** Formal call: does the announcement fan-out KPI (#2723) satisfy the pm-data instrumentation goal, or do we keep the item open with expanded scope? Owner: pm-data.
- **NEW (2026-08-10, pm-scrum-master):** Reviewer-slot policy for human-authored PRs — the accounting trio (#2555/#2558/#2559) has now been draft-ready **13 days** and #2684 is at ~2 days — same failure mode as 2026-07-30, still unaddressed. Owner: pm-tech-lead.
