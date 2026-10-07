# code-review-ppt-web-core-mfa-verify-hardcoded-origin

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review ppt-web-core 2026-10-07
**Confidence:** high

## Hypothesis
`MfaWrapper.verify` in `frontend/apps/ppt-web/src/App.tsx:67-81` sends the MFA challenge via a raw `fetch('/api/v1/auth/mfa/verify', ...)` with a hardcoded same-origin path. Every other api-server call in ppt-web routes through the generated `@ppt/api-client`, whose base URL is set to `VITE_API_URL` in `main.tsx:18-20`. In any deployment where the SPA origin differs from the api-server origin (the normal split-origin topology that `VITE_API_URL` exists for), this request lands on the SPA's own origin, 404s against the static server, and `response.ok` is false forever. Every `mfa_required` manager/admin mutation is permanently blocked. Replace the raw fetch with `getApiClient().post('/auth/mfa/verify', { code })` so the configured base URL, bearer header, and 401 refresh all flow through the shared client.

## Evidence
- `frontend/apps/ppt-web/src/App.tsx:67-81` — `MfaWrapper.verify` hard-codes `fetch('/api/v1/auth/mfa/verify', { method: 'POST', headers, body: JSON.stringify({ code }) })` with the bearer header manually injected from `getAccessToken()`.
- `frontend/apps/ppt-web/src/main.tsx:18-20` — `client.setConfig({ baseUrl: import.meta.env.VITE_API_URL || '' })` is the single configuration point the raw fetch bypasses.
- `grep -rn "fetch('/api/v1" frontend/apps/ppt-web/src/` returns only this one call-site (every other hook routes through the generated client).
- Same bug class was previously fixed in news/person-months paths (see `news-jwt-auth.test.tsx:3-7` regression coverage) — this is the last residual raw-fetch.

## Files
- `frontend/apps/ppt-web/src/App.tsx`

## Dependencies
<none>

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Deploy or run ppt-web with `VITE_API_URL=https://api.example.test` (any origin different from the SPA's own).
2. Sign in as a user whose next mutation triggers `mfa_required` (admin/manager action).
3. Trigger the mutation; the MFA challenge modal opens; enter the 6-digit code.
4. Expected: the mutation proceeds; actual: the modal reports `admin.mfa.verificationFailed` (translates to "Verification failed") indefinitely because the `fetch` posts to the SPA origin, which returns 404 or HTML, so `response.ok === false`.

## Suggested approach
1. Open `frontend/apps/ppt-web/src/App.tsx` at lines 67-81.
2. Replace the raw `fetch` block with a call through the shared client. Minimal shape:
   ```ts
   verify={async (code: string) => {
     try {
       const res = await client.post({ url: '/api/v1/auth/mfa/verify', body: { code } });
       return res.response.ok;
     } catch {
       return false;
     }
   }}
   ```
   Import `client` from `@ppt/api-client` the same way other call-sites do; drop the manual `getAccessToken()` / `Authorization` header plumbing — the client's `registerAuthInterceptors` already injects it (see `main.tsx` registration).
3. Verify the URL path matches the generated client's prefix (`/api/v1/...` or `/auth/mfa/verify` depending on the typegen's base path) — adjust to match what other authed POSTs in the file use.
4. Add a regression unit test under `frontend/apps/ppt-web/src/App.test.tsx` (or a new focused test) that renders `MfaWrapper`, calls the `verify` callback with a mocked client, and asserts the request goes to `${VITE_API_URL}/api/v1/auth/mfa/verify` rather than a bare `/api/v1/...`. Mirror `news-jwt-auth.test.tsx`.
5. Run `pnpm -F @ppt/web typecheck && pnpm -F @ppt/web test` and confirm Biome stays clean (`pnpm check`).

## Alternatives considered
- **Prepend `import.meta.env.VITE_API_URL` manually to the raw fetch** — rejected because it duplicates the base-URL plumbing the shared client already owns and bypasses the 401-refresh interceptor. The bug would recur the moment anyone changes auth flow.
- **Add a one-off helper `fetchWithAuth(path)` that wraps fetch** — rejected because it recreates a third parallel path to the api-server; the whole #1522 cleanup was to centralize on the generated client. Introducing another wrapper would re-fragment auth.

## Root-cause trace
1. Symptom: in cross-origin deployments the MFA modal shows `verificationFailed` indefinitely; every `mfa_required` mutation is blocked.
2. ← `App.tsx:72` `fetch('/api/v1/auth/mfa/verify', ...)` resolves against `window.location.origin` (the SPA), not `VITE_API_URL`.
3. ← The author of `MfaWrapper` did not route this call through the shared client; the surrounding file was authored before #1522 centralized auth on the generated client.
4. Origin: initial MFA wrapper introduction (predates the client-centralization refactor that migrated news/person-months callers).

## Test plan
- [ ] Unit test: `frontend/apps/ppt-web/src/App.test.tsx` (new) renders `MfaWrapper`, invokes the `verify` callback with a stubbed `fetch`/`client`, and asserts the request URL is prefixed with `VITE_API_URL`.
- [ ] Regression: run with a nontrivial `VITE_API_URL` in `.env.test` and confirm the request lands on that origin via a `nock`-style request matcher.
- [ ] Command: `pnpm -F @ppt/web test -- App` and `pnpm -F @ppt/web typecheck`.

## Out of scope
- Reworking any other MFA UX path (modal copy, retry behavior, lockout logic).
- Backend `/auth/mfa/verify` handler changes.
- Migrating any other raw fetch call-sites — this plan only touches the one in `App.tsx`.

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-core-mfa-verify-hardcoded-origin.md`
- Mark the matching `backlog.json` row as `status: "done"`
