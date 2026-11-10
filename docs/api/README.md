# API Documentation

This directory contains the API specifications for the Property Management System.

## Structure

```
api/
├── typespec/           # TypeSpec source files
│   ├── main.tsp       # Entry point
│   ├── domains/       # Domain-specific specs
│   └── shared/        # Shared models
├── generated/         # Generated OpenAPI specs
│   ├── openapi.yaml   # Full consolidated spec
│   └── by-service/    # Per-server specs
└── rules/             # Linting rules
    └── spectral.yaml
```

## Quick Start

### Generate OpenAPI from TypeSpec

```bash
cd docs/api/typespec
npx tsp compile .
```

### Lint OpenAPI Spec

```bash
npx spectral lint docs/api/generated/openapi.yaml
```

### Generate SDKs

```bash
# TypeScript
npx @hey-api/openapi-ts \
  -i docs/api/generated/openapi.yaml \
  -o frontend/packages/api-client/src/generated

# Kotlin (KMP)
openapi-generator generate \
  -i docs/api/generated/openapi.yaml \
  -g kotlin \
  -o mobile-native/shared/src/commonMain/kotlin/api \
  --additional-properties=library=multiplatform
```

## Domain Specifications

| File | Use Cases | Description |
|------|-----------|-------------|
| `auth.tsp` | UC-14 | Authentication, registration, 2FA |
| `organizations.tsp` | UC-27 | Multi-tenancy, organizations |
| `buildings.tsp` | UC-15 | Buildings, floors, common areas |
| `units.tsp` | - | Properties, units, owners |
| `faults.tsp` | UC-03 | Fault reporting and tracking |
| `voting.tsp` | UC-04 | Voting, polls, assemblies |
| `documents.tsp` | UC-08 | Document management |
| `rentals.tsp` | UC-29-30 | Airbnb/Booking integration |
| `listings.tsp` | UC-31-32 | Real estate listings |
| `compliance.tsp` | UC-26 | GDPR, data export/deletion |

## Shared Models

| File | Description |
|------|-------------|
| `models.tsp` | Common data types (Address, Money, etc.) |
| `errors.tsp` | Standard error responses |
| `pagination.tsp` | Pagination patterns |
| `tenant.tsp` | Multi-tenant context |

## API Versioning

- URI-based: `/api/v1/`, `/api/v2/`
- Max 2 active versions
- 12-month deprecation window

## SSO Session: the transient-vs-permanent 401 contract

Reality Portal authenticates through the api-server (Property Management) OAuth
provider and consumes the session on **reality-server** (`:8081`). The
`reality-web` SPA then re-validates the session on every mount. Both sides must
agree on one rule, or a user whose request hit a struggling backend gets
silently logged out. This section pins that rule down against the code that
actually implements it, so integrators (mobile-native, a second SPA, a future
BFF) handle a 401 the same way reality-web does.

### The rule

A **401 (or 403) is a definitive, permanent auth failure**: the presented
credential (bearer token or `portal_session` cookie) is genuinely rejected, so
the consumer MUST clear it and treat the user as logged out.

A **5xx or a transport/network error is transient**: the credential may still
be valid, the backend just could not answer. The consumer MUST keep the
existing (optimistic) session and retry later — it MUST NOT log the user out.

The two sides are symmetric, and the symmetry is load-bearing:

- **Server side**: return `401`/`403` *only* when the credential is actually
  invalid; return `5xx` for any transient/backend failure. Never collapse a
  "my database/upstream is down" condition into a `401`.
- **Client side**: branch on the status. `401`/`403` → discard the credential
  and log out; anything else (including `fetch` rejecting) → keep the session
  and retry on the next mount.

A consumer that treats *any* non-OK response as a logout will drop sessions on
every transient blip; a server that answers a transient failure with `401` will
make a correct consumer log the user out. Both are the same bug seen from
opposite ends, and the fix on the client was PR #3121 (see below).

### What the server actually returns

| Endpoint (reality-server) | Auth | `401`/`403` means… | `5xx` means… |
|---|---|---|---|
| `GET /api/v1/users/me` | bearer (form-login) | `RequestPrincipal` rejected the token — missing/invalid/expired → **permanent** | handler `500` on a DB lookup failure → **transient** (a `404` means the user row is gone) |
| `GET /api/v1/sso/session` | `portal_session` cookie | no/invalid session token → treat as unauthenticated | see caveat below |
| `POST /api/v1/sso/refresh` | `portal_session` cookie | session expired/invalid | see caveat below |
| `POST /api/v1/sso/sync` | PM token in body | `401 pm_session_expired` — PM token genuinely inactive → **permanent** | `500 session_invalidation_failed` — could not tear the portal session down → **transient, caller should retry** (`inactive_pm_token_response`) |
| `POST /api/v1/sso/exchange`, `/sso/mobile/*` | PM token in body | `401 invalid_token` — PM introspection says inactive | a transient PM-introspection outage surfaces here as a `401 invalid_token` carrying the upstream error text — see note |

Server-side transient guards worth knowing about:

- **Token introspection is poison-resistant** (`introspect_pm_token_inner`,
  Epic 104.2): a non-2xx response from the PM introspection endpoint (a PM 5xx
  outage, gateway error, or transport error) returns an error and is **not**
  cached. Only a genuine `{"active": false}` — which RFC 7662 delivers as an
  HTTP 2xx — is cached as a definitive "inactive" verdict. So a transient PM
  outage cannot deny every session presenting a token for the full cache TTL.
  It does, however, surface to the exchange/mobile/sync caller as a
  `401 invalid_token`, so a caller that wants true transient-vs-permanent
  resolution on these endpoints should inspect the error body rather than the
  status alone.
- **`sync_session` never claims a false logout**: if the PM token is inactive
  but the portal session cannot be invalidated, it returns `500`
  (`session_invalidation_failed`), not `401`, so the caller keeps the live
  session and retries.

> **Caveat (honest gap).** `get_session` and `refresh_session`
> (`/sso/session`, `/sso/refresh`) currently map **any** `session_service`
> error to `401` (`.map_err(|_| StatusCode::UNAUTHORIZED)`), so a transient
> session-store failure on the cookie path is reported as a permanent `401`.
> This is masked today because the reality-web bearer path (below) bails on a
> transient failure *before* it reaches the cookie probe, and the cookie probe
> only runs for users with no stored bearer session (whose optimistic user is
> already `null`). A new consumer that relies on the cookie path directly
> should be aware of this and not over-trust a `401` from these two endpoints.

### What the reality-web client actually does

`frontend/apps/reality-web/src/lib/auth-context.tsx` implements the client half.
`AuthProvider` hydrates an optimistic user from `localStorage`, then
`checkSession()` re-validates on mount:

1. **Bearer path** — `GET /api/v1/users/me` with the stored token:
   - `2xx` → adopt the returned user.
   - `401`/`403` → the token is rejected: `clearStoredSession()` and fall
     through to the cookie probe (effectively logs out).
   - **any other status (notably `5xx`)** → keep the optimistic session and
     bail; the next mount retries. Does **not** fall through to the cookie
     probe (which would hit the same struggling backend and clobber the
     session to `null`).
   - **`fetch` rejects (network error)** → same as above: keep the session,
     bail, retry next mount.
2. **Cookie path** — `probeSsoSession()` → `GET /api/v1/sso/session`: a network
   error returns `ok:false` **without caching**, so the next mount retries
   rather than caching a false "logged out".

`refreshSession()` applies the identical branch (`401`/`403` → clear token +
`setUser(null)`; any other status or a network error → keep optimistic state).

This transient-failure cushion was introduced in **PR #3121**
(`code-review-reality-web-auth-ctx-transient-logout`, merged 2026-10-18). Before
it, `checkSession`'s bearer path treated *any* non-OK `/users/me` response as a
token rejection and fell through to the cookie probe, so a transient `5xx` (or a
network blip) wiped a logged-in user's session. Regression coverage lives in
`frontend/apps/reality-web/src/lib/auth-context.test.tsx`.

### Integrator checklist

- Treat `401`/`403` — and only `401`/`403` — as "the credential is dead; log
  out and (if interactive) send the user to login".
- Treat `5xx` and transport failures as retryable; keep the session.
- When you own a server endpoint, do not answer a transient/backend failure
  with `401`; use `5xx` (`500`/`503`) so a correct consumer retries instead of
  logging the user out.
