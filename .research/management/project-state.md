# PPT Project State

_Generated: 2026-08-10 — routine Phase 1.6 rotating role slot (**pm-qa**). Coverage `scan_kind=upkeep`; pm_cursor idx 3 → 4 (pm-qa → pm-devops next), coverage_cursor idx 5 → 6 (epic-80 re-checked, evidence refreshed via PR #2712; no status flip)._

## Executive summary

- **Delivery still at 47/49 stories done, 2 partial** (the 84-1 direct-to-S3 upload wiring and 84-2 sign page). The 2026-08-07 → 08-10 window shipped **22 PRs** and closed **3 tracked issues** (#2703 SSRF DNS-rebinding TOCTOU via #2710, #2704 unbounded-response DoS via #2707, #2612 fire-once notifications via #2714). The dispatcher's auto-fix + post-merge-review loops are visibly retiring the reality-server auth security backlog.
- **Reality-server security batch cleared this window:** #2724 (db_error leak), #2725 (password-reset transport), #2726 (SSO session-invalidate error swallow), #2727 (agency-members unauth IDOR) all shipped code fixes. QA gap flagged: no evidence in the merge digest of matching failing-on-main negative tests (see pm-qa risks).
- **Churn dedupe pattern working on api-server:** #2721 (layout tenant, `acquire_public_conn` extract), #2720 (reports helpers), #2715 (auth error boilerplate –83 lines), #2713 (layout admin dedupe –108 lines), #2711 (layout tenant-override dedupe). Reality-server not yet touched; state.rs (1201 lines) and routes/agencies.rs (624 lines) are top hotspots this run and queued for a matching split proposal.
- **Two long-standing PR follow-ups closed:** #2483 add_evidence cross-tenant IDOR via PR #2712 (audit event added), and #2485 layout webhook replay via PR #2718 (HMAC parity test extended to layout leg).
- **Buffer starvation was the top-priority signal this run:** dispatcher-side open buffer at 1/72 and local buffer at 2/36 before this run; refilled to **36/36** with 34 new items (mvp finish work, this window's test-coverage debt, carried pm-data KPI wave, churn hotspots).
- **Open PRs (16):** 4 human-authored (accounting trio #2555/#2558/#2559 at 13 days idle, workflow-cond fix #2684 at ~2 days) — reviewer starvation is the sustained delivery blocker; 12 dependabot bumps stacking up.

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 3/5** unchanged this run. Extended-scope coverage across 13 epics: 47/49 stories done.

| Epic | Sprint status | Coverage status |
|---|---|---|
| 6 — Announcements & Communication | in-progress | 6/6 stories done |
| 7A — Basic Document Management | in-progress | 5/5 stories done |
| 8A — Basic Notification Preferences | done | 3/3 stories done |
| 10A — OAuth Provider Foundation | done | 3/3 stories done |
| 10B — Platform Administration | in-progress | 7/7 stories done |
| 80 — Dispute Resolution | partial | 3/3 stories done in coverage; **80-1 evidence refreshed via PR #2712** this run |
| 84 — Documents / e-signature | (extended) | 3/5 done, 2 partial (84-1 direct-S3 wiring, 84-2 sign page) |
| 82 / 83 / 85 / 79 / 81 / 8a / 9 | (extended) | all done in coverage |

## Shipped since last run (22 PRs > #2702)

- **#2727** code-review-reality-server-agency-members-unauth-idor-retry1
- **#2726** code-review-reality-server-sso-session-invalidate-swallowed
- **#2725** code-review-reality-server-password-reset-no-transport
- **#2724** code-review-reality-server-db-error-leak
- **#2723** data-announcement-fanout-metric-2026-07-23-retry2 (retires pm-data announcement-fanout item)
- **#2722** code-review-api-handlers-community-unauthenticated-reads-retry2
- **#2721** refactor-churn-hotspot-api-server-layout-tenant-2026-07-30
- **#2720** refactor-churn-hotspot-api-server-reports-2026-07-31
- **#2719** code-review-reality-server-inquiry-notify-route-wiring
- **#2718** sec-layout-webhook-hmac-verify-2026-07-23-retry2 (closes #2485 mitigation)
- **#2717** dx-fixme-admin-web-mobile-config-patch-endpoint-retry1
- **#2716** dx-fixme-admin-web-platform-settings-patch-endpoint-retry1
- **#2715** refactor-churn-hotspot-api-server-auth-2026-07-31 (–83 lines)
- **#2714** gh-issue-2612-retry1 (notification durability, watermarks — Closes #2612)
- **#2713** refactor-churn-hotspot-api-server-layout-admin-2026-07-30 (–108 lines)
- **#2712** data-audit-add-evidence-idor-fix-2026-07-23-retry2 (closes #2483 mitigation, refreshes 80-1 evidence)
- **#2711** refactor(api-server) layout tenant-override dedupe
- **#2710** gh-issue-2703 (Closes #2703 SSRF TOCTOU)
- **#2709** code-review-reality-web-listingform-no-i18n (next-intl)
- **#2708** workflow_executor non-finite compare guard
- **#2707** gh-issue-2704 (Closes #2704 8MiB body cap DoS)
- **#2706** RAG partial-batch fail-closed

Closed-not-merged: **#2705** dependabot rust-toolchain 1.100.0 nonexistent (triage queued).

## What's next (top 5 actions from ranked backlog)

1. **[high] Un-quarantine /disputes/kpis test + window_start<=window_end validation** (#2575 open 10 days) — **owner: pm-backend**.
2. **[high] Reality-server security batch regression tests** (#2724/#2725/#2726/#2727 shipped fixes without negative tests) — **owner: pm-backend/pm-qa**.
3. **[high] Convert workflow_executor.rs unparseable-condition branch to fail-closed** (separate from #2708 NaN guard) — **owner: pm-backend**.
4. **[high] Finish 84-1** direct-to-S3 wiring in ppt-web (POST /documents/upload-url consumer) — **owner: pm-frontend**. Soft-blocked by #2320 upload hardening.
5. **[high] Finish 84-2** signer-facing document-sign page (screen-map planned → shipped) — **owner: pm-frontend**.

## Blockers

- **Dispatcher buffer starvation** (mitigated this run) — dispatcher side was at 1/72; local queue was 2/36; refilled to 36/36. Cause: post-scan candidates weren't merged into `action-list.json` on prior 3 runs. Guard: pm-scrum-master owns the buffer-health check.
- **Reviewer starvation on human PRs** — #2555/#2558/#2559 accounting trio at 13 days idle; #2684 workflow-cond fix at ~2 days; the bot review loop is consuming all reviewer bandwidth. Same failure mode as 2026-07-30, still not addressed. Owner: pm-tech-lead.

## Role focus today: pm-qa (rotating idx 3), pm-scrum-master (always)

**pm-qa summary:** 22-PR burn window shipped a heavy security wave — every code fix warrants a matching failing-on-main negative test. Reality-server security batch (#2724-#2727) coverage bar needs an explicit QA gate before we call the security backlog cleared. `/disputes/kpis` still quarantined 10 days after the follow-up was filed.

**pm-scrum-master summary:** Huge 3-day window, 22 PRs merged, 3 tracked issues CLOSED; coverage unchanged at 47/49. Top priority this run was the buffer refill (2/36 → 36/36); next priority is unblocking the two long-standing 84-x partials.
