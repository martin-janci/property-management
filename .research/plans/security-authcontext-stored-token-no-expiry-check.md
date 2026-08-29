# security-authcontext-stored-token-no-expiry-check

**Vector:** security
**Score:** 2
**Source:** dispatcher Tier-1d ppt-web-core review 2026-08-28 (signal `code-review-ppt-web-core-init-no-exp-check`)
**Confidence:** high

## Hypothesis
`AuthContext.tsx` cold-boot init reads a stored access token from `localStorage` and re-authenticates the user without validating the token's `exp` claim, despite an in-source comment that claims otherwise (`// Validate the token is not expired (basic check)`). The result: any stored access token — even one issued weeks ago and long past its short-lived TTL — is treated as valid on tab reload until the *next* API call happens to return 401, and even then no runtime silent-refresh exists (see sibling plan). The smallest safe change is to decode the JWT `exp` locally, treat an expired token as absent, and drive the existing `refreshTokenValue` cold-boot branch instead of the naive `setUser(storedUser)` branch.

## Evidence
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:412-415` — `if (storedUser && accessToken) { // Validate the token is not expired (basic check) ... setUser(storedUser); }`; grep of the file confirms no `exp` decode, no `jwt-decode`, no expiry check anywhere in the init effect.
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:116` — comment `// Token Storage (localStorage for MVP, httpOnly cookies later)` explicitly documents the MVP contract; the missing expiry check is a residual gap in that contract, not a design goal.
- Sibling finding `code-review-ppt-web-core-no-runtime-token-refresh` — the axios client is wired with `getToken` only, no `onUnauthorized`, so after a stale token is accepted here the first 401 is *not* auto-refreshed either; the two findings compound: stale-accept + no-recover = indefinite silent auth failure.
- Signals file: `.research/signals/2026-08-28-ppt-web-core-tier1d-auth-refresh.json`

## Files
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:405`
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:412`
- `frontend/apps/ppt-web/src/contexts/AuthContext.test.tsx`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug/security vector)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. In `AuthContext.test.tsx`, seed `localStorage` with a JWT whose `exp` claim is in the past (e.g. `exp: Math.floor(Date.now()/1000) - 3600`) plus a matching `ppt_user` and `ppt_refresh_token`.
2. Render `<AuthProvider>` and wait for the init effect.
3. Expected: cold-boot detects the expired access token, treats it as absent, and enters the `else if (refreshTokenValue)` branch — hitting `refreshTokenInternal()` (which can be mocked to reject → clean logout, or resolve → rotated tokens).
4. Actual (today): `setUser(storedUser)` runs unconditionally, the mocked refresh is never called, and the user is authenticated with an expired token.

## Suggested approach
1. Add a tiny `isTokenExpired(token: string): boolean` helper alongside `tokenStorage` (~line 119): base64-decode the JWT payload, parse `exp`, return `true` when `exp * 1000 <= Date.now() - 30_000` (30s clock-skew slack), also `true` on any decode/parse error (fail-safe). No external dep — `atob` + JSON.parse is sufficient for this MVP contract.
2. In the init effect at `AuthContext.tsx:412`, change the branch to `if (storedUser && accessToken && !isTokenExpired(accessToken))`. The existing `else if (refreshTokenValue)` branch already handles the "refresh on cold-boot" path via `refreshTokenInternal` (the correct routine — see the block comment at :417-425).
3. Delete the misleading `// Validate the token is not expired (basic check)` comment (now a lie become truth: replace with a one-liner explaining the helper).
4. Add three unit tests in `AuthContext.test.tsx`: (a) expired access + refresh present → `refreshTokenInternal` called, (b) expired access + no refresh → user cleared and logged out, (c) valid access → fast-path setUser (regression on the happy case).
5. Run `pnpm -F @ppt/web test src/contexts/AuthContext.test.tsx` and `pnpm -F @ppt/web typecheck`.

## Alternatives considered
- **Server-side validation on init** — rejected because it costs a round-trip on every cold boot for a check the client can do locally in µs; the existing runtime path already validates via the actual API call that fails 401.
- **Adopt `jwt-decode` package** — rejected because the payload decode is 5 lines with `atob` and adding a runtime dep for one call site is bloat; if a future story needs richer JWT introspection, revisit then.

## Root-cause trace
1. Symptom: stored, expired access token accepted on cold boot → user authenticated with a token the server will reject on the first request.
2. ← `AuthContext.tsx:412` — init branch trusts `storedUser && accessToken` without any expiry check.
3. ← `AuthContext.tsx:116` — module-level comment documents the token-storage design as an MVP with httpOnly-cookies as the eventual target; the `exp` gate was deferred to "later" alongside the storage move but is trivially addable now.
4. Origin: initial `AuthContext` scaffolding (predates the churn window). Latent since the auth epic landed; surfaced by dispatcher Tier-1d review 2026-08-28.

## Test plan
- [ ] `frontend/apps/ppt-web/src/contexts/AuthContext.test.tsx` — three new cases covering expired-with-refresh, expired-without-refresh, valid-access happy path.
- [ ] Regression: existing SSO callback + login-flow tests continue to pass (they use fresh tokens; the change is a no-op for them).
- [ ] `pnpm -F @ppt/web test -- src/contexts/AuthContext.test.tsx && pnpm -F @ppt/web typecheck`

## Out of scope
- Wiring the axios `onUnauthorized` runtime silent-refresh (that's the sibling `code-review-ppt-web-core-no-runtime-token-refresh` plan — deliberately separated so this PR stays small).
- Migrating tokens off `localStorage` to httpOnly cookies (documented as a future story in the source comment; requires backend cookie-issuance work).
- Introducing a scheduled proactive-refresh timer.

## After-merge
- Move this file to `plans/_archive/security-authcontext-stored-token-no-expiry-check.md`
- Mark the matching `backlog.json` row as `status: "done"`
