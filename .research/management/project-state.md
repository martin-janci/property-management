# PPT Project State

_Generated: 2026-06-17 — daily PM rotation (Scrum Master + pm-security; routine refresh). Coverage `scan_kind=upkeep`; pm_cursor idx 5 → 6 (pm-data next), coverage_cursor idx 12 → 0 (epic-9 → epic-10a)._

## Executive summary

- **Backend stabilization sweep landed: 47 merged PRs since cursor (#1218 – #1499)** — heavy enum-decode (PR #1499 unblocks dev test job), RLS hardening (PR #1495 forms), Booking.com OTA OAuth at-rest encryption (PR #1472), document download org-scope gate (PR #1463), IDOR fixes (PR #1457 appeals org-scope, PR #1438 work-orders RlsConnection), dispatcher self-healing (T21 fix #1444, depends_on propagation #1448), and cron validator drift guard (#1443).
- **`dev` red is RESOLVED.** PR #1495 + #1499 closed the form-RLS-grant / enum-decode dev-test-job redness; PR #1452 added the `cargo check --workspace --tests` dev-push smoke gate (today's `pm-devops-dev-push-compile-gate` action — DONE); PR #1455 mirrored fmt+clippy as CI status check. Issue #1332 / #1437 unblockers shipped.
- **Booking.com OAuth security cluster:** PR #1472 (at-rest credential encryption) merged closing #1362, but PR #1473 (cross-tenant token-binding + manager-role gate) **still OPEN** — flagged as today's top pm-security risk. Until #1473 lands, the connect-flow can bind credentials to the wrong tenant.
- **Phase 1.5 finding emitted:** `code-review-ppt-web-core-mfa-verify-hardcoded-url` — ppt-web App.tsx:69 hardcoded `/api/v1/auth/mfa/verify` ignores VITE_API_URL → cross-origin deploys 404 on MFA verify → silent "wrong code" UX. Added as P-high risk.
- **Open follow-up issues (6):** #1300 (PAP-142 doc download IDOR handler test), #1305/#1306/#1307 (RLS-routing FORCE-RLS test coverage trio), #1332 (CI test-job redness — addressed but not closed), #1238 (dispatcher T21/finish-first gate). #1471 also still open from the form-RLS cluster.
- **Stale drafts still need a call:** #1316, #1197, #988 (epic-scale — Playwright E2E framework epic landed today as `4bf7430` / PR #988!). Confirm #988 closure.

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · epics_done=1/5.

| Epic | Tracked status | Real status (from coverage + this-run merges) |
|---|---|---|
| 6 — Announcements & Communication | in-progress | 1/6 done; pinned-first ordering shipped (PR #1446 UC-06) — partial slices advancing |
| 7A — Basic Document Management | in-progress | doc download/preview org-scope (PR #1463) + folder MoveDialog work; #1300 handler-level IDOR test still open |
| 8A — Basic Notification Preferences | **near-done** | publish-leg replay (#1458) shipped; only mobile-push FCM/APNs leg remains |
| 10A — OAuth Provider Foundation | in-progress | 3/3 backend done; admin/user UI done; integration test suite still owed (#1197 stale draft) |
| 10B — Platform Administration | in-progress | 5/7 stories complete (unchanged this run) |
| 9 — MFA (rotating recheck) | done | All 1/1 stories done; Phase 1.5 finding is deploy-config bug, not a story regression |
| 80 — Disputes | in-progress | dispute watch + flush-on-unmount + i18n (#1468) closes #1364/#1404 |
| 82 — Mobile (Reality KMP) | in-progress | iOS stale-response + staging deep-link + xcconfig drift (e3723bb); debounced search evidenced (#1392) |

## Shipped since last run (47 PRs, range #1218 – #1499) — top 5 themes

- **Backend RLS/IDOR hardening:** #1495 (form RLS-role grant) · #1499 (enum decode) · #1457 (appeals org-scope) · #1463 (doc download building/unit gate) · #1438 (work-orders RlsConnection) · #1429 (reserve balance FOR UPDATE) · #1442 (canonical seed_membership helper)
- **Booking.com OTA OAuth:** #1472 (encrypted at rest, closes #1362) · #1458 (publish-leg replay test, BIT-77) · `4eb5068` (OTA push retry timeout + retry-after) · #1473 cross-tenant token-binding **still open**
- **CI discipline:** #1452 (dev-push cargo check workspace smoke) · #1455 (fmt+clippy CI lint gate) · #1441 (pre-push hook) · #1443 (cron validator drift guard, GH #1368) · #1490 (RLS baseline strict) · 4 dispatcher Tier-2 + depends_on fixes (#1444/#1448/#1483)
- **Dispute & UX:** #1468 (disputes watch + flush + i18n) · #1446 (pinned-first announcement order, UC-06) · #1477 (mobile auth-gate wiring test, GH #1411)
- **Mobile:** #1392 debounced search evidence · e3723bb iOS stale-response guard + staging deep-link host + xcconfig drift fix · #1428 de-tautologize AuthGuard test

## What's next (top 5 actions)

1. **[high · pm-security] Land PR #1473 — Booking.com OAuth cross-tenant token-binding + manager-role gate.** Block further Booking.com OTA work until merged. (today's rotation finding)
2. **[high · pm-frontend] Fix ppt-web App.tsx:69 hardcoded MFA verify URL** (Phase 1.5 finding) — route via @ppt/api-client; vitest cross-origin assertion.
3. **[high · pm-qa] Add handler-level cross-org IDOR test for document download/preview (#1300)** — gate further documents.rs edits on the test file existing.
4. **[high · pm-qa] FORCE-RLS test trio (#1305/#1306/#1307)** — three test files covering ai/sessions+calendar, webhook RLS-routed repos, ESG cross-tenant.
5. **[high · pm-scrum-master] Triage remaining 6 open follow-up issues** (#1300/#1305/#1306/#1307/#1332/#1238) + #1471 — assign owner or close.

## Blockers

- **PR #1473 OPEN** — Booking.com OAuth cross-tenant token-binding hazard. Until merged, the credential-connect flow can bind to the wrong tenant.
- **#1300 IDOR handler-level test gap** for document download — silent regression risk on a heavy-churn surface.
- **`security-test-gate.yml` enforcement** still unconfirmed (carry-over from pm-devops 2026-06-16) — risk that future security PRs ship test-less again.
- **Stale draft PR #1197** (OAuth integration tests, ~8d) — pm-scrum-master decision needed.

## Role focus today: **pm-security** (+ pm-scrum-master always-on)

- **pm-security** (rotation idx 5, last 2026-05-27, 21d stale): 5 new risks appended to `risks.json` (P-high: PR #1473 token-binding, MFA hardcoded URL; P-medium-high: doc download IDOR test, RLS-routing test trio; P-medium: #1471 triage). 5 new actions appended to `action-list.json`. Headline: Booking OAuth #1473 must land before any further OTA work; MFA verify URL is a silent cross-origin deploy bug at the auth trust boundary.
- **pm-scrum-master** (always-on): produced delivery synthesis above. Backend stabilization sweep cleared dev-red; the security-adjacent test debt (IDOR handler tests, FORCE-RLS assertions) is now the dominant residual risk class.

## Coverage upkeep

- **epic-9 (rotation idx 12) refreshed** in `coverage.json`:
  - `9-1-totp-2fa-setup`: `last_checked` 2026-05-27 → 2026-06-17. No MFA/TOTP code churn in the 47-PR window. Phase 1.5 MFA URL finding tracked as risk pm-security-mfa-verify-hardcoded-url, NOT a 9-1 regression. Status stays `done`.
- Next epic to refresh: **epic-10a** (coverage_cursor idx 0).
