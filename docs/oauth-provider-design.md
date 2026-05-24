# OAuth Provider (10A) — Token, Storage & Rotation Design

**Status:** Canonical reference (implementation complete in Epic 10A)
**Owner:** pm-tech-lead
**Last updated:** 2026-05-24
**Source files:**
- `backend/crates/db/migrations/00028_create_oauth_provider.sql`
- `backend/crates/db/migrations/00147_oauth_principal_kind.sql`
- `backend/crates/db/migrations/00150_audit_action_oauth_token_denied.sql`
- `backend/crates/db/src/models/oauth.rs`
- `backend/crates/db/src/repositories/oauth.rs`
- `backend/servers/api-server/src/services/oauth.rs`
- `backend/servers/api-server/src/routes/oauth.rs`

---

## Purpose

This document records the authoritative design decisions for the PPT OAuth 2.0
Authorization Server (Epic 10A). Future implementers working on adjacent
features (third-party integrations, voice assistant OAuth, SSO, or external
consumer OAuth flows) must read this document before touching any of the OAuth
tables or the `OAuthService` to avoid conflicting assumptions.

---

## 1. Scope

The `api-server` acts as an **OAuth 2.0 Authorization Server** (RFC 6749).
Third-party applications and internal consumers (mobile apps, voice assistants,
external portals) authenticate against it to obtain scoped access on behalf of
PPT users.

This document does **not** cover the OAuth Consumer flows (Calendar, Airbnb,
voice-device token exchange) — those are documented in
`docs/phase2/oauth-integration.md`.

---

## 2. Grant Flows Supported

| Grant Type | Status | Notes |
|---|---|---|
| Authorization Code + PKCE | Supported | Required for all clients |
| Refresh Token | Supported | Confidential clients only |
| Client Credentials | Not supported | No machine-to-machine grant |
| Implicit | Not supported | Deprecated per OAuth 2.1 |
| Password Grant | Not supported | Deprecated per OAuth 2.1 |

**PKCE requirement:** S256 is the only accepted `code_challenge_method`. The
`plain` method is intentionally rejected — accepting `plain` defeats PKCE's
replay-attack protection and is banned by OAuth 2.1 draft.

---

## 3. Token Design

### 3.1 Token Format

All tokens (authorization codes, access tokens, refresh tokens) are **opaque
random byte strings**, not JWTs. This is a deliberate choice:

- Opaque tokens can be revoked instantly by deleting the row; JWT revocation
  requires a blocklist.
- The internal API auth tokens (user sessions) are JWTs. OAuth provider tokens
  are a separate surface with different revocation needs.
- Token introspection (RFC 7662) covers the "what does this token grant?"
  query for resource servers that need structured claims.

### 3.2 Entropy & Encoding

| Token type | Raw entropy | Encoded length | Encoding |
|---|---|---|---|
| Authorization code | 32 bytes | 43 chars | base64url (no padding) |
| Access token | 32 bytes | 43 chars | base64url (no padding) |
| Refresh token | 32 bytes | 43 chars | base64url (no padding) |
| Client ID | 16 bytes | 22 chars | base64url (no padding) |
| Client secret | 32 bytes | 43 chars | base64url (no padding) |

**CSPRNG:** `rand::rngs::SysRng` (OS-backed) is used for all secret generation,
not `thread_rng`. This avoids any dependency on the ChaCha20 thread-local
CSPRNG being seeded first.

### 3.3 Token Lifetimes

| Token type | Default TTL | Configurable? |
|---|---|---|
| Authorization code | 10 minutes (600 s) | Via `OAuthConfig` |
| Access token | 15 minutes (900 s) | Via `OAuthConfig` |
| Refresh token | 7 days (604 800 s) | Via `OAuthConfig` |

`OAuthConfig` defaults are set in `OAuthService::default()`. They can be
overridden at construction time via `OAuthService::with_config()` if
environment-variable-based configuration is added later.

### 3.4 Token Storage (Hashing)

Tokens are **never stored in plaintext**. The only value persisted in the
database is a SHA-256 hex digest of the raw token:

```
stored = hex(sha256(raw_token_bytes))
```

SHA-256 is used instead of Argon2id for tokens because:
- Tokens have 256 bits of entropy from a CSPRNG — they are not human-chosen
  passwords, so slow hashing is not needed to resist dictionary attacks.
- Argon2id is intentionally expensive (to slow brute-force of low-entropy
  secrets). Applying it to 32-byte CSPRNG tokens wastes CPU on every request
  for no security gain.
- Client secrets (which could theoretically be shorter and admin-chosen) **do**
  use Argon2id via `AuthService::hash_password`.

| Secret type | Storage hash |
|---|---|
| Authorization codes | SHA-256 hex |
| Access tokens | SHA-256 hex |
| Refresh tokens | SHA-256 hex |
| Client secrets | Argon2id (via AuthService) |

---

## 4. Token Rotation Design

### 4.1 Refresh Token Rotation

Refresh token rotation is **configurable per client** via the
`rotate_refresh_tokens` boolean on `oauth_clients`.

**Default:** `true` (rotation enabled). New clients should enable rotation
unless there is a specific technical constraint preventing it (e.g. offline
device sync that cannot reliably receive a new token before the window closes).

**Rotation flow:**
1. Client presents refresh token RT₁ to `/oauth/token`.
2. Service looks up RT₁ by hash, verifies it is valid and not revoked.
3. Service revokes RT₁ (`revoked_at = NOW()`).
4. Service issues new access token AT₂ and new refresh token RT₂ in the same
   `family_id` as RT₁.
5. Client receives AT₂ + RT₂ and discards RT₁.

### 4.2 Token Family & Reuse Detection

Every refresh token row carries a `family_id` UUID (shared across all rotated
descendants of an original refresh grant). Rotation preserves `family_id`:

```
Initial grant:   family_id = X, refresh_token = RT₁
After rotation:  family_id = X, refresh_token = RT₂
After rotation:  family_id = X, refresh_token = RT₃
```

**Reuse detection:** If a client presents a refresh token that has already been
revoked (i.e. it has a `revoked_at` timestamp), the service treats this as a
potential token theft:

1. The entire token family (all rows with the same `family_id`) is revoked
   immediately via `revoke_token_family(family_id)`.
2. The request returns `OAuthServiceError::TokenReuseDetected` → HTTP 400
   `invalid_grant`.
3. The legitimate client holding the current (non-revoked) token will receive
   errors on its next request, forcing re-authorization — the expected behavior
   after a detected breach.

This pattern is based on the [OAuth Security BCP § 4.14](https://datatracker.ietf.org/doc/html/draft-ietf-oauth-security-topics#section-4.14).

### 4.3 Public vs Confidential Clients

| Client type | `is_confidential` | Gets refresh token? | Secret required for token endpoint? |
|---|---|---|---|
| Confidential | `true` (default) | Yes | Yes |
| Public | `false` | No | No (PKCE only) |

Public clients (e.g. SPAs without a backend) never receive refresh tokens.
The `issue_tokens` helper takes a `with_refresh: bool` parameter — for public
clients this is `false`, so no refresh row is written to the database at all
(avoiding orphan refresh tokens that could be replayed from a DB leak).

---

## 5. Database Schema

### 5.1 Tables

| Table | Purpose |
|---|---|
| `oauth_clients` | Registered OAuth applications |
| `oauth_authorization_codes` | Short-lived codes for the authorization flow |
| `oauth_access_tokens` | Issued access tokens (opaque, SHA-256 hashed) |
| `oauth_refresh_tokens` | Issued refresh tokens with family tracking |
| `user_oauth_grants` | Per-user record of which clients they have authorized |

### 5.2 Key Columns

**`oauth_clients`**
- `client_id` — base64url(16 random bytes), unique
- `client_secret_hash` — Argon2id hash of the client secret
- `redirect_uris` — JSONB array; validated on every authorization request
- `scopes` — JSONB array of allowed scopes
- `is_confidential` — whether the client can keep a secret
- `rotate_refresh_tokens` — enable refresh token rotation
- `allowed_principal_kinds` — TEXT[] restricting which user kinds may auth
  via this client (`public`, `staff`, `platform`); empty = all allowed

**`oauth_refresh_tokens`**
- `family_id` — UUID shared by all rotated descendants; used for reuse
  detection and full-family revocation

**`user_oauth_grants`**
- Unique on `(user_id, client_id)` — one active grant record per user/client pair
- `revoked_at` — set when user removes an authorized app
- Revoking a grant cascades to revoke all access and refresh tokens for that pair

### 5.3 Indexes

Lookup indexes exist on all hash columns (`code_hash`, `token_hash`) and
on `family_id` for efficient family revocation. Partial indexes filter by
`revoked_at IS NULL` / `used_at IS NULL` so cleanup queries do not scan
the full history.

### 5.4 Cleanup

`cleanup_expired_oauth_data()` is a PL/pgSQL function (migration 00028) that
hard-deletes:
- Authorization codes that expired OR were used more than 1 hour ago
- Access/refresh tokens expired OR revoked more than 7 days ago

It is called by the scheduler service hourly.

---

## 6. Scopes

| Scope | Description |
|---|---|
| `profile` | User's name and avatar |
| `email` | User's email address |
| `org:read` | Read-only access to organization data |
| `full` | Full access to account and actions |

Adding a new scope requires changes in four places:
1. `OAuthScope` enum in `backend/crates/db/src/models/oauth.rs`
2. Scope description used on the consent page (`OAuthScope::description()`)
3. Scope validation in `OAuthService::register_client()` and `update_client()`
4. This table

---

## 7. Principal Kind Filtering

**Migration 00147** added `allowed_principal_kinds TEXT[]` to `oauth_clients`.
This allows a client to be restricted to a subset of user kinds:

- `public` — portal users (Reality Portal accounts)
- `staff` — property management staff users
- `platform` — super-admin / platform principals

The check happens in two places:
1. `exchange_code_for_tokens()` — initial token issuance
2. `refresh_tokens()` — at every refresh, so a config change takes effect
   immediately without waiting for existing refresh tokens to expire

A rejection is audit-logged as `oauth_token_denied_principal_kind`
(migration 00150 extends the `audit_action` enum).

---

## 8. Audit Logging

All OAuth operations are audit-logged using the standard `audit_log` table:

| Event | `audit_action` value |
|---|---|
| User authorizes a client | `OAuthAuthorize` |
| User revokes a client | `OAuthRevoke` |
| Token refreshed | `OAuthTokenRefresh` |
| Client registered | `OAuthClientCreate` |
| Client revoked | `OAuthClientRevoke` |
| Secret regenerated | `OAuthSecretRegenerate` |
| Token denied (principal kind mismatch) | `oauth_token_denied_principal_kind` |

Tokens and secrets are **never** included in audit log payloads — only
`client_id`, `user_id`, and scope lists.

---

## 9. API Endpoints

### Public OAuth Endpoints (`/api/v1/oauth/`)

| Method | Path | Purpose |
|---|---|---|
| GET | `/oauth/authorize` | Show consent page (returns `ConsentPageData`) |
| POST | `/oauth/authorize` | User approves/denies; returns redirect with code |
| POST | `/oauth/token` | Exchange code or refresh token for tokens |
| POST | `/oauth/revoke` | Revoke an access or refresh token (RFC 7009) |
| POST | `/oauth/introspect` | Inspect token metadata (RFC 7662) |
| GET | `/oauth/grants` | List user's authorized apps |
| DELETE | `/oauth/grants/{client_id}` | Revoke user's authorization for a client |

### Admin Endpoints (`/api/v1/admin/oauth/`)

All admin endpoints require `Capability::OauthClientWrite` (platform admin).

| Method | Path | Purpose |
|---|---|---|
| POST | `/admin/oauth/clients` | Register new OAuth client |
| GET | `/admin/oauth/clients` | List all clients |
| GET | `/admin/oauth/clients/{id}` | Get client details |
| PATCH | `/admin/oauth/clients/{id}` | Update client configuration |
| DELETE | `/admin/oauth/clients/{id}` | Revoke client + all tokens |
| POST | `/admin/oauth/clients/{id}/regenerate-secret` | Rotate client secret |

---

## 10. Security Invariants

The following rules must be preserved by any future change:

1. **No plaintext secrets in DB.** Tokens → SHA-256. Client secrets → Argon2id.
2. **Authorization codes are single-use.** `used_at` is set atomically on
   consumption; a second attempt returns `invalid_grant`.
3. **PKCE S256 only.** The `plain` method must never be accepted.
4. **Public clients never receive refresh tokens.** No `with_refresh=true` for
   `is_confidential=false` clients.
5. **Token reuse triggers full-family revocation.** A replayed revoked refresh
   token invalidates all siblings.
6. **Principal-kind check on every token issuance including refresh.** A
   permission change on the client config takes effect at the next refresh.
7. **No secrets in logs or audit records.** Only IDs, scopes, and timestamps
   are logged.
8. **Admin endpoints gated by `Capability::OauthClientWrite`.** No role-string
   checks — the capability layer re-derives the principal from the DB.

---

## 11. Future Work / Known Gaps

| Gap | Severity | Notes |
|---|---|---|
| OAuth metadata endpoint (`/.well-known/oauth-authorization-server`) | Low | Useful for auto-discovery by third-party clients |
| Dynamic Client Registration (RFC 7591) | Low | Currently admin-only; self-service registration would enable partner integrations |
| PKCE `state` parameter CSRF validation in frontend | Medium | The consent UI must round-trip `state` back to the client; not enforced server-side beyond returning it |
| Refresh token absolute expiry vs sliding | Low | Current TTL is absolute from issuance; a sliding window could extend long-lived integrations |
| Voice assistant device token (Epic 93) | Medium | Needs refresh TTL of ~1 year + device-specific revocation; must hook into this token infrastructure rather than create a parallel store |

---

## 12. Relation to Internal JWT Auth

The **internal session JWT** used by `api-server` routes (15-minute access +
7-day refresh, stored in HttpOnly cookies) is a **separate surface** from the
OAuth tokens documented here. They share the same user table and the same
token lifetime numbers but are issued by different code paths:

- Internal JWT: `AuthService::generate_jwt()` / stored in Redis session
- OAuth access token: `OAuthService::issue_tokens()` / stored in `oauth_access_tokens`

Do not conflate the two. A client that has an OAuth access token cannot use it
as a session cookie, and vice versa.
