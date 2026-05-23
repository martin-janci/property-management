# PPT Project State

_Generated: 2026-05-23 — produced by `/ppt-project-management scan` (deep coverage scan, role rotation skipped per scan-mode spec)._

## Executive summary

- **49 stories** scanned across 13 epics. **16 done (33%) · 32 partial (65%) · 1 not-started (2%)**.
- The codebase is further along than `sprint-status.yaml` suggests — multiple Epic 7A / Epic 80 / Epic 81 stories have shipped code that isn't reflected in the sprint tracker, but most are `partial` because the apiStatus stub flag in `docs/screens/` means web UIs render without invoking the real backend endpoints.
- Three structural gaps drive most of the partial bucket: **(1)** apiStatus-stub frontend wiring across Epics 6 + 7A; **(2)** missing WebSocket realtime infra blocking 8a-3 + 6-5 closure; **(3)** Epic 10B backend route stubs (10b-4/5/6/7) plus missing 81-x backend pause/resume/history endpoints the frontend already calls.

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

| Epic | Tracked status | Real status (from scan) |
|---|---|---|
| 6 — Announcements & Communication | in-progress (1/6 done) | 6 partial, 0 done — backend done, frontend apiStatus stub |
| 7A — Basic Document Management | in-progress (0/5 done) | 5 partial — backend merged in #6, mobile + API integration missing |
| 8A — Basic Notification Preferences | review (0/3 done) | 2 done, 1 partial (8a-3 awaits WebSocket) |
| 10A — OAuth Provider Foundation | in-progress (0/3 done) | 3 done — backend complete, tests + admin UI noted as gaps |
| 10B — Platform Administration | in-progress (3/7 done) | 3 done, 4 partial (handler stubs) |

## What's next (top 5)

1. **[high · pm-security]** Wire `TwoFactorAuthPage` to `/api/v1/auth/mfa/*` — add `useMfa` hooks to `@ppt/api-client` and ship 2FA end-to-end. _Why:_ security feature silently disabled (UI scaffold only).
2. **[high · pm-backend]** Build WebSocket realtime sync infra for notification preferences (8a-3). _Why:_ blocks Epic 8A closure and unblocks direct-messaging realtime (6-5). Dependency: Epic 2B.
3. **[high · pm-backend]** Complete frontend API integration for documents permission-based access (7a-3). _Why:_ apiStatus stub means RLS enforcement isn't user-visible.
4. **[high · pm-frontend]** Wire login form/logout/session cleanup to `AuthContext` (79-2). _Why:_ auth plumbing missing despite context fully built.
5. **[high · pm-frontend]** Implement mobile (RN) document upload UI with metadata (7a-1). _Why:_ mobile slice missing on a shipped backend; representative of broader mobile lag across Epic 7A.

See `roadmap.md` for the full ranked plan (17 MVP items + Phase 2/4 follow-ups), and `action-list.json`/`action-list.md` for the top-10 in tracker form.

## Blockers

- **Epic 2B WebSocket infrastructure** — gating story 8a-3 (preference sync), 6-5 (DM realtime), and broader notification dispatch flows. Not started per gap-scan evidence.
- **Sprint-status drift** — multiple stories show `ready-for-dev` despite merged PRs (7a-*, 81-*, 80-2/3). Tracker is stale; update once apiStatus is verified.

## Role focus today

_Scan mode — no `pm-*` role rotation executed (per skill spec, scan-mode runs only the deep coverage scan + ranking)._
