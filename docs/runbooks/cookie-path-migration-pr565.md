# Cookie Path Migration — PR #565 (P0-12 Session-Cookie Scope Hardening)

**Issue:** #617  
**PR:** #565 (gap-security-435-cookie-scope)  
**Applies to:** api-server (`refresh_token` cookie) and reality-server (`portal_session` cookie)  
**Date:** 2026-05-28

---

## Summary of the change

PR #565 hardened session-cookie attributes on both servers as part of P0-12:

| Cookie | Server | Old Path | New Path | Old SameSite | New SameSite |
|--------|--------|----------|----------|--------------|--------------|
| `refresh_token` | api-server | `/` | `/api/v1/auth` | `Lax` | `Strict` (default; `Lax` override via `PPT_AUTH_COOKIE_SAMESITE`) |
| `portal_session` | reality-server | `/` | `/api/v1/sso` | `Lax` | `Strict` |

Both cookies have always been `HttpOnly` and `Secure`. The scope narrowing
(`/` → specific API prefix) is the primary change flagged in issue #617.

---

## Does the new Path silently log existing sessions out?

### Short answer: yes, for the first logout attempt after upgrade only.

**Why:** A browser that has an existing `refresh_token` (or `portal_session`)
cookie from before PR #565 holds it with `Path=/`. The new logout handler emits
a clear-cookie with `Path=/api/v1/auth` (or `/api/v1/sso`). Because the two
`Path` attributes differ, the browser sees them as two distinct cookies. The new
clear-cookie creates/expires a _new_ `/api/v1/auth`-scoped slot, but the old
`Path=/` cookie remains alive in the browser's cookie store.

**Effect in practice:**

- The user calls `/api/v1/auth/logout`. The server revokes the token in the DB
  (the token is still sent by the browser on that request because `Path=/` cookies
  match any path). The session is invalidated server-side.
- The old `Path=/` cookie is NOT expired by the new clear-cookie — it stays in
  the browser jar. However, on the very next `/api/v1/auth/refresh` call, the
  browser sends the old cookie, the server validates it, finds it revoked, and
  returns `401 TOKEN_REVOKED`. The client then redirects to login.
- Result: **one extra 401 on the next refresh attempt** — the user is forced to
  log in again. This is _not_ a security regression; the token is already revoked
  server-side. It is a UX hiccup.

**Mitigation:** No code change needed. The server-side revocation is authoritative.
The stale `Path=/` cookie becomes permanently inert after its first 401 — the
client clears its stored token and the browser eventually evicts the cookie when
its `Max-Age` (7 days) expires. Operators may also set `PPT_AUTH_COOKIE_DOMAIN`
to a fully-qualified domain so the upgrade can set an explicit `Max-Age=0 Path=/`
clear on the first login after upgrade (not implemented here — out of scope for P0-12).

### api-server `refresh_token` — detailed path analysis

The cookie `Path=/api/v1/auth` is sent by the browser on every request to:

- `POST /api/v1/auth/login` ✓ (login sets it)
- `POST /api/v1/auth/refresh` ✓ (refresh reads and rotates it)
- `POST /api/v1/auth/logout` ✓ (logout reads and clears it)
- `GET  /api/v1/auth/sessions` — reads it via `parse_refresh_cookie` (informational)
- All other `/api/v1/auth/*` paths ✓

The cookie is NOT sent on:

- `GET /api/v1/users/**` — correct; refresh tokens should not travel on non-auth paths.
- `GET /api/v1/properties/**`, `/api/v1/faults/**`, etc. — correct.
- `GET /api/v1/auth/me` — note: this endpoint is auth'd via `Authorization: Bearer`
  (access token), not the refresh cookie. No regression.

### reality-server `portal_session` — detailed path analysis

The cookie `Path=/api/v1/sso` is sent by the browser on:

- `GET /api/v1/sso/callback` — this endpoint **sets** the cookie; it does not read it.
  The browser doesn't send a `portal_session` on this request because it hasn't been
  set yet. This is correct behavior (see SSO callback section below).
- `POST /api/v1/sso/logout` ✓ reads it via `extract_session_cookie`.
- `GET  /api/v1/sso/session` ✓ reads it via `extract_session_token` (cookie fallback).
- `POST /api/v1/sso/refresh` ✓ reads it via `extract_session_token` (cookie fallback).
- `POST /api/v1/sso/exchange`, `POST /api/v1/sso/sync` — these use a JSON body token,
  not the session cookie. Not affected.

The cookie is NOT sent on:

- `GET /api/v1/listings/**` — correct; session cookies should not go on public listing requests.
- `GET /api/v1/realtors/**`, `/api/v1/reports/**`, etc. — correct.

---

## Does the new Path break the SSO `/auth/callback` cookie read?

**Short answer: No. The SSO callback endpoint SETS the cookie; it never reads one.**

The question may stem from confusion between:

1. **`GET /api/v1/sso/callback`** (reality-server backend) — the OAuth redirect
   landing point. It exchanges the authorization code for tokens, creates a portal
   session, and emits `Set-Cookie: portal_session=...; Path=/api/v1/sso`. The
   browser doesn't need to send a cookie here; it is receiving one.

2. **`/auth/callback`** (ppt-web SPA frontend) — the `AuthCallbackPage` route.
   This is a React client-side route, not a backend endpoint. It processes the
   OAuth authorization code redirect for the _Property Management_ (api-server)
   OAuth flow, not the Reality Portal SSO flow. It reads a `code` query parameter
   from the URL and exchanges it via the JavaScript client. Cookies are not
   involved here.

### SameSite=Strict and the OAuth redirect

The PR comment in `sso_callback` notes that `SameSite=Strict` is safe on the
callback because the cookie is being SET (not read) during the final redirect.
This is correct:

- The browser performs a `GET /api/v1/sso/callback?code=...&state=...` triggered
  by a redirect from the PM OAuth provider (a cross-site navigation).
- `SameSite=Strict` governs whether the browser **sends** cookies on cross-site
  requests. It has no effect on the **Set-Cookie** response header — the browser
  always stores a new cookie regardless of `SameSite`.
- After the callback response, when the SPA makes same-site `fetch()` calls to
  `/api/v1/sso/session` or `/api/v1/sso/logout`, those are same-site requests
  and the `SameSite=Strict` cookie is included normally.

**Conclusion:** The SSO callback flow is unaffected by the SameSite=Strict change.

---

## Operator upgrade checklist

For deployments upgrading from a pre-#565 release to a post-#565 release:

- [ ] **No server-side action required.** All existing sessions remain valid in
  the database. Users will experience at most one forced re-login if they attempt
  to use a stale `Path=/` cookie after the upgrade.
- [ ] **Monitor 401 `TOKEN_REVOKED` spike at upgrade time.** A brief spike of
  `401 TOKEN_REVOKED` responses on `/api/v1/auth/refresh` is expected immediately
  after upgrade as browsers re-use old `Path=/` refresh tokens that have not yet
  been rotated. This is normal and self-resolving.
- [ ] **`PPT_AUTH_COOKIE_SAMESITE` env var.** If your deployment uses the Reality
  Portal SSO flow where the same origin serves both the OAuth provider (api-server)
  and the consumer (reality-web), you may need `PPT_AUTH_COOKIE_SAMESITE=Lax` to
  allow the cross-site redirect to carry the refresh cookie. The `portal_session`
  cookie on reality-server does not have an env override — it is always `Strict`.
- [ ] **`PPT_AUTH_COOKIE_DOMAIN` env var (optional).** Setting this forces the
  browser to scope the new cookie to the specified domain, enabling the old `Path=/`
  cookie to eventually be overwritten on the next login. Only needed for zero-hiccup
  upgrades; omitting it is safe.

---

## Verification (PR #565 + #617 reconciliation)

The following tests were added in this reconciliation PR to lock in the analysis:

### api-server (`backend/servers/api-server/src/routes/auth.rs`)

- `refresh_cookie_path_has_no_trailing_slash` — Path attribute is exactly
  `/api/v1/auth` (no trailing slash that would break browser path-matching).
- `set_and_clear_cookie_use_identical_path` — set-cookie and clear-cookie carry
  the same Path, ensuring logout clears the same slot the login set.
- `parse_refresh_cookie_returns_none_when_cookie_header_absent` — body-fallback
  path still works for pre-migration clients (localStorage flow).
- `parse_refresh_cookie_requires_exact_name_match` — no false-positive matches
  on similarly-named cookies.

### reality-server (`backend/servers/reality-server/src/routes/sso.rs`)

- `portal_session_cookie_path_has_no_trailing_slash` — Path is exactly `/api/v1/sso`.
- `portal_session_set_and_clear_cookie_use_identical_path` — logout clear-cookie
  matches the callback set-cookie path.
- `extract_session_cookie_round_trips_with_build` — the write-side
  (`build_portal_session_cookie`) and read-side (`extract_session_cookie`) agree
  on cookie name `portal_session`.
- `extract_session_cookie_absent_returns_none_for_callback_path` — SSO callback
  request (no pre-existing cookie) returns None, confirming the callback does not
  read a cookie it has not yet set.
