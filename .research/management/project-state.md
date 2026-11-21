# Project State — PPT (Property Management)

<sub>Generated: 2026-11-21T03:06:10Z — Scrum Master synthesis + pm-backend (rotation index 1 → 2). Coverage cursor advanced idx 7 → 8 (epic-82 re-checked, next epic-83).</sub>

## Executive summary

A major delivery wave: **32 PRs merged** (#3650–#3686). The three structural refactors pm-tech-lead escalated last run — `push_fanout.rs` / `voice_webhooks.rs` / `oauth_integration_tests.rs` — **all shipped in-window** (#3659 push/, #3685 voice_webhooks/, #3686 oauth_integration_tests/). Accounting MVP trio progressed: #3661 landed invoice sent/cancel lifecycle + PAY-by-square QR, closing #2555 / #2559. Issues #3649 (suggestions seed-city for `de`) and #3666 (HelloSign webhook replay) both closed. #3669 broadened the reality-server surface (portal auth fix + buyer inquiry threads + notification settings + agency dashboard). #3675 shipped feed delete + tenant application/forms 500 fixes + release 248 quarantined tests. Sprint spine (6/6 epics) remains done — delivery continues in continuous-hardening mode. Churn files this run (`voice_webhooks/` children, `push/` children, `oauth_integration_tests/` children) are the new post-split module trees; the real question for the next run is whether behaviour survived the splits.

## Sprint progress

- **Sprint:** Hardening & refactor track (post-sprint — Epic 6, 7A, 8A, 10A, 10B, 9 all done)
- **Epics done:** 6 of 6 (100%)
- **Coverage (upkeep this run):** 49 of 49 stories done; rotating cursor re-checked **epic-82** (iOS SwiftUI Reality Portal — 5/5 stories done; `last_checked` advanced).

## Shipped since last run (2026-11-20T03:06:10Z → 2026-11-21T03:06:10Z)

- #3659 — `push_fanout.rs` → `push/` module split (fcm/apns/adapter/worker/config)
- #3685 — `voice_webhooks.rs` module split (verification + handler + parsers)
- #3686 — `oauth_integration_tests.rs` split by flow (authorize/token/introspect-revoke/grants)
- #3669 — reality-server portal auth fix + buyer inquiry threads + notification settings + agency dashboard
- #3675 — feed delete, tenant application/forms 500 fixes, release 248 quarantined tests
- #3660 — reality-portal client alignment
- #3670 — reality-portal Authorization header fix
- #3661 — accounting-server invoice sent/cancel lifecycle + PAY-by-square QR (closes #2555 / #2559)
- #3666 — HelloSign webhook replay (issue closed)
- #3649 — suggestions seed-city for region `de` (issue closed)
- Many code-review-* fix PRs across api-core, api-handlers, reality-web, mobile-rn

## What's next (top 5)

1. **[high · pm-backend]** Sweep the new `push/` and `voice_webhooks/` child modules for the RLS-before-verify ordering bug that survived the split — webhook signature verification must precede the `RlsConnection` extractor.
2. **[high · pm-scrum-master]** Promote-or-close the 6 open PRs (#3668 dependabot, #3687/#3688 pm-integration drafts, #3676/#3677 mobile-rn i18n drafts, #3682 search-drainer) within 48h.
3. **[high · pm-tech-lead]** Resolve dispatcher stall #3357 — same blockers as last run; the 32-PR merge wave did not drain it.
4. **[high · pm-devops]** Unblock KMP + swagger-ui egress in the cloud runner (#2652, #2966) — claimable still ≈9/72.
5. **[medium · pm-tech-lead]** Reset `sprint-status.yaml` — current sprint goal is fully "done"; next sprint goal should name the hardening/refactor track explicitly.

## Blockers

- **Dispatcher pipeline (#3357)** — 3 approved PRs still blocked on non-mechanical conflicts; the merge wave did not drain it. Owner: `pm-tech-lead`.
- **KMP / mobile-native backlog items** (CreateListingScreen stub, InquiriesResponse contract, Ktor HttpTimeout, SSO error body) — Gradle/KMP builds unavailable in cloud runner (#2652, #2966); claimable ≈9/72. Owner: `pm-devops`.
- **Dependabot cargo-group bump #3668** — open, unmerged; needs rebase against post-split HEAD. Owner: `pm-devops`.
- **6 open PRs** — drafts need promotion-or-close decisions. Owner: `pm-scrum-master`.

## Role focus today

`pm-scrum-master` (always-on), `pm-backend` (rotation index 1 → 2).

### pm-backend — one-line

The three oversized backend files all split this window (#3659/#3685/#3686) + accounting lifecycle (#3661) + HelloSign replay (#3666 closed). The next structural risk is whether the mechanical splits kept the RLS-before-verify ordering correct — that's a sweep, not a refactor.

### pm-scrum-master — one-line

32 PRs merged, 3 structural refactors executed, 2 issues closed, accounting MVP progressed. Pipeline-side the constraint remains #3357 + KMP/swagger egress + a 6-PR open-draft overhang.
