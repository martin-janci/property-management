# PPT Project State

_Generated: 2026-07-02 — daily PM rotation (Scrum Master + pm-security; routine refresh). Coverage `scan_kind=upkeep`; pm_cursor idx 5 → 6 (pm-data next), coverage_cursor idx 12 → 0 (epic-9 → epic-10a). 16-day catch-up window: 380 merged PRs, 165 issues touched, 83 closed-not-merged, 582 commits._

## Executive summary

- **Massive shipping window.** 380 merged PRs since 2026-06-16. Dominant themes: (1) test-hardening flood on the backend (BIT-405/406/407/408/416/420/440/479 waves — dozens of `test(api-server)` PRs re-enabling quarantined suites and backfilling happy-path 2xx coverage for finance, governance, faults, accounting, admin/platform-admin, budget); (2) endpoint-checklist tallies advancing Wave 7 → Wave 8 (1132 → 1287 done, 56.4% → 64.1%, BIT-258); (3) coverage-gap reconcile flurry for epic-6, 8a, 79, 81, 82, 85 stories (all “done” via dev-reconcile against sprint-status).
- **pm-security rotation focus (36d stale — role_last_run was 2026-05-27).** Three new security-relevant findings this window: (1) **#1987 SSO callback CSRF state-bypass** (reality-web) — real-live gap now fixed, but the pattern likely repeats across other OAuth/SSO callback handlers → new risk `risk-security-sso-oauth-callback-csrf-audit`; (2) **#1973 + #1976 manager-gate PII read-path fixes** (rentals + regional-compliance) landed together — indicates a systemic pattern of GET/list handlers dropping tenant/manager gates → new risk `risk-security-manager-gate-pii-read-path-audit`; (3) **#1967 JwtService access-gate pinning** shipped with **no test file in diff** (observation hotfix-no-test signal) → new risk `risk-security-jwt-service-access-gate-no-test`. Three matching pm-security actions added.
- **Dev-red incident from 2026-06-16 resolved.** Issue #1437 (dev backend compile break from #1426) is off the blockers list. Backend CI is green on `dev` (assumed from 380 subsequent merges, including `#1985` "unblocks dev test gate"). The structural gap that permitted #1426 — no push-gate on `dev` — remains, but the immediate fire is out.
- **Test-gate discipline still moving:** #1984 quarantined 9 dev-hostage suites (BIT-440), #1985 removed a duplicate `#[ignore]`, #1991 un-quarantined 5 finance suites (BIT-479). Net direction: coverage restored.
- **Reverted this window:** PR #1713 revert(delegations): removed re-added delegation frontend (#1690, BIT-213). Only one revert in 16 days is a good signal for the merge queue.
- **New coverage upkeep — epic-9 (TOTP 2FA):** re-checked; no new PRs touch story 9-1-totp-2fa-setup; status remains `done` with `last_checked=2026-07-02`.

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"**. Coverage snapshot (49 stories) at 2026-06-23 was **37 done · 12 partial · 0 not-started**; the flurry of "reconcile to done" PRs (#1830-#1918 range) has since closed most of the sprint's partial rows.

| Epic | Tracked status | Real status (from coverage + activity) |
|---|---|---|
| 6 — Announcements & Communication | in-progress | 6-1/6-2/6-3/6-4/6-5 all landed via reconcile PRs (#1832-#1844) |
| 7A — Basic Document Management | in-progress | 7a-5 mobile share sheet UUID-validation shipped (#2003); 7a-1/7a-2 verify tasks still open |
| 8A — Basic Notification Preferences | done | 8a-1 (#1918), 8a-2 (#1913), 8a-3 (#1915) all reconciled; mobile-push (FCM/APNs) leg tracked in action-list |
| 9 — TOTP 2FA | done | 9-1 done since 2026-05-24; no new activity this window |
| 10A — OAuth Provider Foundation | in-progress | Still partial; #1197 draft stale, security contract tests still owed |
| 10B — Platform Administration | in-progress | 10b-1, 10b-2, 10b-7 reconciled; 10b-6 onboarding-tour helper coverage shipped (#1996) |
| 79 — Frontend Foundation | in-progress | 79-1 done, 79-2 auth-flow blocked on pm-security SSO/JWT/cookie sign-off, 79-3 error handling shipped tests (#2002) |
| 80 — Disputes | in-progress | 80-2 5-step wizard + i18n still open; 80-3 mediation submissions dropped from action-list |
| 81 — Reports | in-progress | 81-1 done (#1917), 81-2 verify-to-done shipped (#2004) |
| 82 — Mobile (Reality KMP) | in-progress | 82-3 home/search KMP buildStatus reconciled on Home + Search screen-maps (#2005) |
| 84 — Notifications trigger | partial | 84-5 pgvector RAG done (#1833); 84-4 trigger system dropped |
| 85 — Mobile Build Pipeline | done | 85-1 (#1914), 85-2 (#1916) both reconciled |

## Shipped since last run (16-day catch-up, top slice)

- **#2005** — 82-3-home-search-screens: track KMP build status on Home + Search screen-maps [pm-frontend]
- **#2004** — 81-2-report-execution-history: verify-to-done + pin apiStatus reason [pm-frontend]
- **#2003** — 7a-5-document-sharing: mobile share sheet UUID-validate user-ID (#485) [pm-frontend]
- **#2002** — 79-3-error-handling-toasts: unit coverage for ppt-web error handler [pm-frontend]
- **#1994** — fix(migration): SQL-side template pagination + honest import/export checklist (#1905) [pm-backend]
- **#1993** — fix(messaging): unread soft-delete leak + set-based block check (#1771/#1776/#1789) [pm-backend]
- **#1992** — perf(reality): set-based favorite-alert worker + idempotent mark-read [pm-backend]
- **#1991** — test(api-server): fix + un-quarantine 5 finance happy-path suites (BIT-479) [pm-qa]
- **#1987** — fix(reality-web): close SSO callback CSRF state-bypass [pm-security] ← **new risk basis**
- **#1976** — fix(regional-compliance): live quorum lookup + manager-gate writes + export scoping [pm-security]
- **#1973** — fix(rentals): manager-gate guest/booking PII read paths [pm-security]
- **#1970** — test(iot): sensor WS authz regression tests (non-member 403 before upgrade) [pm-qa]
- **#1967** — fix(api-core): restore token_type/raw_kind logs + pin JwtService access gate [pm-security] ← **hotfix-no-test signal**
- **#1955** — integration: consolidate 22 reviewed/compiling PRs onto one branch (merge-then-fix) [pm-devops]
- Test-hardening waves (BIT-405/406/407/408/416/420/440/479, ~15 PRs) — [pm-qa]
- Endpoint-checklist Wave 7 → Wave 8 tallies (#1959, #1978) — 64.1% done [pm-backend]

## What's next (top 5 actions)

1. **[high] Audit OAuth/SSO callback CSRF state parameter** across Airbnb, Booking.com, admin SSO, any Google/OIDC path — same defect that PR #1987 fixed on reality-web. Owner: pm-security. Action: `pm-security-audit-oauth-sso-callbacks-csrf`.
2. **[medium] Sweep read-path handlers dropping `_principal`** cascading from #1973 + #1976 manager-gate fixes. Owner: pm-security. Action: `pm-security-audit-manager-gate-pii-read-paths`.
3. **[medium] Backfill regression tests for JwtService access gate** pinned in #1967 (no test in diff). Owner: pm-security. Action: `pm-security-jwt-access-gate-regression-test`.
4. **[medium] 79-2 auth-flow sign-off (SSO/JWT/cookie)** still owed — carried over from 2026-06-23 roadmap. Owner: pm-security (with pm-frontend). Elevated priority given #1987 findings.
5. **[medium] Close open follow-ups from prior windows** — 10a-1/10a-3 OAuth blocked by issue #481 (revoked-token bypass), 8a-3 mobile push (FCM/APNs), 80-2 dispute 5-step wizard.

## Blockers

- **OAuth callback CSRF pattern (NEW).** Owner: pm-security. Fixed on reality-web (#1987); other callbacks unaudited.
- **Issue #481 OAuth refresh-token revocation bypass.** Blocks 10a-1/10a-3 promotion. Owner: pm-qa + pm-security.
- **Issue #480 JWT access token in WebSocket query-param access logs.** Blocks 8a-3 final promotion. Owner: pm-qa.
- **Stale drafts still need a call** — #1316, #1197, #988 (unchanged from prior run — no evidence they moved).

## Role focus today: **pm-security** (+ pm-scrum-master always-on)

- **pm-security** (rotation idx 5, last 2026-05-27, 36d stale): 3 new next_actions appended to `action-list.json`; 3 new risks appended to `risks.json`. Headline: real CSRF state-bypass fixed on reality-web SSO (#1987) but pattern likely systemic across OAuth callbacks; two manager-gate PII read-path fixes (#1973/#1976) suggest broader sweep needed; JwtService access-gate hardening (#1967) shipped without tests.
- **pm-scrum-master** (always-on): produced this delivery synthesis; headline = massive test-hardening + reconcile-to-done wave has closed most sprint partials; only structural residual is stale draft PRs (#1316/#1197/#988) and the OAuth/JWT security follow-ups.

## Coverage (upkeep — 2026-07-02, epic-9)

- **coverage_cursor advanced 12 → 0** (rotated through the 13-epic table). Today's slice: epic-9 (TOTP 2FA) — single story `9-1-totp-2fa-setup` remains `done` with high confidence, no PRs touched it this window; `last_checked` bumped to 2026-07-02.
- No structural coverage changes; deep-scan snapshot from 2026-06-23 (37 done · 12 partial · 0 not-started) remains authoritative for the `roadmap.md` ranked plan. The dev-reconcile flood has closed many of the partial rows in practice; a fresh deep scan is due when the coverage_cursor completes its next full rotation.
