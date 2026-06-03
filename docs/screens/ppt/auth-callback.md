---
id: ppt/auth-callback
name: SSO Callback
product: ppt
sitemapRefs:
  ppt-web: ppt-auth-callback
implementations:
  ppt-web:
    component: AuthCallbackPage
    buildStatus: in-progress
    redesignStatus: not-started
    apiStatus: stub
  mobile:
    buildStatus: n/a
    redesignStatus: n/a
    apiStatus: n/a
endpoints:
  - auth_sso_callback
relatedScreens:
  - id: ppt/login
    rel: parent
  - id: ppt/dashboard
    rel: child
sharedComponents:
  - login-spinner
  - banner
diagrams: []
owner: pm-frontend
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Happy path
- [w] Reads `?code=` and `?state=` query params from SSO provider redirect
- [w] Validates client-side state nonce via `getAndClearSsoState()` before any network call (OIDC §3.1.2.7)
- [w] Guards against React StrictMode double-invoke with `useRef(false)` flag
- [w] Calls `AuthContext.loginWithSsoCode({ code, state, redirectUri })` to exchange code for PPT JWT
- [w] On success: redirects to stored return URL (or `/dashboard`) via `getAndClearReturnUrl()`
- [w] Short-circuits on `isAuthenticated` (e.g. browser back after success) — redirects without re-exchange

### Error handling
- [w] SSO provider error in redirect (`?error=`, `?error_description=`): shows provider error message
- [w] Missing `code` or `state` params: shows "Invalid callback URL" error banner
- [w] State nonce mismatch: shows "state parameter mismatch" security error, blocks network call
- [w] Token exchange failure: shows user-friendly error + "Back to login" button

### Loading UI
- [w] Pending spinner with "Completing sign-in…" while exchange is in flight
- [w] Success spinner with "Redirecting…" while navigation is in flight

## States

- **Pending**: spinner shown while `loginWithSsoCode` is in flight.
- **Success**: brief "Redirecting…" state before `navigate()` fires.
- **Error (provider)**: SSO provider returned `?error=` in the redirect; shows decoded `error_description`.
- **Error (missing params)**: `?code` or `?state` absent — malformed redirect.
- **Error (state mismatch)**: stored nonce absent or doesn't match URL param — CSRF/open-redirect guard.
- **Error (exchange failed)**: backend rejected the code (expired, already used, backend not yet implemented).

## Notes

### Broader context

Intermediate page in the SSO login flow. The user never navigates here directly — they land via an SSO provider redirect from the `/login` page's SSO buttons. The flow is: Login page → SSO provider → `/auth/callback` → dashboard.

### Specific (recent)

- **Backend stub**: `POST /api/v1/auth/sso/callback` is not yet implemented (tracked separately). The exchange step will surface an error until the backend ships.
- **State nonce lifecycle**: `setSsoState(crypto.randomUUID())` must be called by the SSO initiation path (Login page SSO buttons) before the provider redirect. `getAndClearSsoState()` in this page consumes and removes the nonce atomically so it cannot be replayed.
- **`redirectUri` must match exactly** what the backend registered with the SSO provider. Derived from `window.location.origin + '/auth/callback'`.
- **`biome-ignore lint/correctness/useExhaustiveDependencies`** on the mount-only `useEffect` is intentional — `loginWithSsoCode` and `navigate` are stable refs; `searchParams` changes every render but the exchange must run only once.
- **Return-URL is open-redirect hardened** (PR #922, dev-review round 2): the post-exchange redirect via `getAndClearReturnUrl()` now passes through `sanitizeReturnUrl()` in `@ppt/shared` (same-origin rooted paths only; absolute/protocol-relative/scheme/control-char values are dropped to `null` → falls back to `/dashboard`). This complements the state-nonce check as a second open-redirect guard on the SSO return path.

## Agent Log

<!-- newest entries on top -->

- 2026-06-03 — agent: test-gap-screen-map-drift-pr-922-ppt — noted PR #922 (dev-review round 2) return-URL hardening: post-SSO redirect via `getAndClearReturnUrl()` now sanitized by `sanitizeReturnUrl` in `@ppt/shared`, a second open-redirect guard alongside the state nonce. No frontmatter change (still in-progress/stub — backend endpoint pending).
- 2026-05-27 — agent: gap-79-2 review fixes — created screen-map (ppt/auth-callback); buildStatus=in-progress, apiStatus=stub (backend endpoint not yet implemented); added state nonce validation (OIDC §3.1.2.7) and loginWithSsoCode catch block (partial write rollback) per reviewer findings on PR#568
- 2026-05-27 — agent: gap-79-2-login-flow-impl-v2 — new route /auth/callback; AuthCallbackPage wired; loginWithSsoCode added to AuthContext
