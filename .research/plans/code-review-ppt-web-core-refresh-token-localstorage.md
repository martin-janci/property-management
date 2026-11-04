# code-review-ppt-web-core-refresh-token-localstorage

**Vector:** security
**Score:** 2
**Source:** rotating-expert-review 2026-11-03 ppt-web-core (dispatcher Tier-1d); upgraded to confidence=high in routine run 2026-11-04 (buffer-low refill) after re-reading AuthContext.tsx
**Confidence:** high

## Hypothesis
ppt-web persists the long-lived rotating refresh token in `localStorage` under `ppt_refresh_token` (AuthContext.tsx:100, 147), then reads it back into the request body on `/auth/refresh` (around 138-191, 406-416). Any single XSS on the SPA origin reads it and renews sessions indefinitely, surviving logout. The api-server already ships `build_refresh_cookie` (`backend/servers/api-server/src/routes/auth/mod.rs:641-739`) emitting an `HttpOnly; Secure; SameSite=Strict` cookie scoped to `/api/v1/auth`, and the refresh handler already prefers the cookie over the JSON-body token (same file, around 843). The fix is SPA-only: stop writing the refresh token to `localStorage`, send `/auth/refresh` with `credentials: 'include'`, and drop `refreshToken` from the request body. Blast radius is the refresh flow; access token can stay in memory/localStorage (short-lived).

## Evidence
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:100` — `const REFRESH_TOKEN_KEY = 'ppt_refresh_token';`
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:145-151` — `setRefreshToken` writes to `localStorage`.
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:137-143` — `getRefreshToken` reads it back.
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:116` — comment acknowledges "localStorage for MVP, httpOnly cookies later" — the migration is now unblocked.
- `backend/servers/api-server/src/routes/auth/mod.rs:641-739` — `build_refresh_cookie` is the server-side mechanism the SPA is bypassing.

## Files
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [x] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [x] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Log in to ppt-web, DevTools → Application → Local Storage → the origin: key `ppt_refresh_token` holds a long-lived JWT.
2. Expected: no refresh token in localStorage; only an `HttpOnly refresh_token` cookie on `/api/v1/auth` is visible via `document.cookie` returning empty for it.
3. Actual: `localStorage.getItem('ppt_refresh_token')` returns a readable JWT that survives logout-then-close.

## Suggested approach
1. In `AuthContext.tsx`, delete `getRefreshToken` / `setRefreshToken` and the `REFRESH_TOKEN_KEY` constant; remove the `localStorage.removeItem(REFRESH_TOKEN_KEY)` line from `tokenStorage.clear()`.
2. Replace every `refreshTokenInternal` body send of `{ refresh_token }` with `fetch(..., { credentials: 'include' })` (or axios `withCredentials: true`) and no body token. The api-server `/api/v1/auth/refresh` already prefers the `HttpOnly` cookie set by `build_refresh_cookie`.
3. On login + SSO-callback success, stop calling `tokenStorage.setRefreshToken(...)`. The server already sets the cookie on every successful auth.
4. On logout, call the existing `POST /auth/logout` with `credentials: 'include'` so the server clears the cookie; no client-side action on the refresh token is needed.
5. Keep the access token in memory + `localStorage` (short-lived — unchanged).
6. Add a regression test in `frontend/apps/ppt-web/src/contexts/AuthContext.test.tsx` that asserts `localStorage.getItem('ppt_refresh_token')` is `null` after a successful login response.
7. Remove the stale MVP comment at AuthContext.tsx:116 noting the migration happened.

## Alternatives considered
- **Keep both** (`localStorage` AND cookie) — rejected because the point of `HttpOnly` is to make the token JS-unreachable; keeping the localStorage copy preserves the XSS exfiltration path.
- **Encrypt the localStorage value with a server-issued key** — rejected because the decryption key would itself be JS-reachable, so XSS still exfiltrates both; cosmetic defense that adds complexity without raising the bar.

## Root-cause trace
1. Symptom: refresh token readable by any script on `ppt-web` origin.
2. ← `AuthContext.tsx:147` `localStorage.setItem(REFRESH_TOKEN_KEY, token)` — the SPA writes to a JS-readable store.
3. ← MVP shortcut at `AuthContext.tsx:116` — comment acknowledges the intended target was `HttpOnly` cookies.
4. Origin: the original pre-cookie commit that introduced `REFRESH_TOKEN_KEY`. The backend cookie path (`build_refresh_cookie`) was added later (gap-security-435-cookie-scope), but the SPA never migrated.

## Test plan
- [ ] Unit test in `frontend/apps/ppt-web/src/contexts/AuthContext.test.tsx`: after `login()` resolves, `localStorage.getItem('ppt_refresh_token')` returns `null`.
- [ ] Unit test: calling `logout()` clears `ppt_access_token` but does not touch `ppt_refresh_token` (which must never exist).
- [ ] Manual: log in, confirm `Set-Cookie: refresh_token=...; HttpOnly; Secure; SameSite=Strict; Path=/api/v1/auth` on the login response; `document.cookie` does not reveal it.
- [ ] Run: `pnpm -F @ppt/web test -- AuthContext`.

## Out of scope
- Access-token storage (short-lived; unchanged).
- Mobile app refresh storage (React Native has its own secure-storage; separate plan if needed).
- Backend changes — `build_refresh_cookie` already exists and is wired up for refresh/login/logout.

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-core-refresh-token-localstorage.md`
- Mark the matching `backlog.json` row as `status: "done"`
