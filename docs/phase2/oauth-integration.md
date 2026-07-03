# Story 87.1: OAuth Integration Requirements

## Overview

This document outlines the requirements for completing OAuth integration across all platforms. The system uses OAuth 2.0 in two modes:
1. **OAuth Provider** - api-server acts as authorization server for third-party apps
2. **OAuth Consumer** - api-server/reality-server consume external OAuth providers

## Current Implementation Status

### OAuth Provider (Complete)

The OAuth 2.0 Authorization Server is fully implemented in `api-server`:

| Feature | Status | Location |
|---------|--------|----------|
| Authorization Code Flow | Complete | `routes/oauth.rs` |
| PKCE Support (RFC 7636) | Complete | `services/oauth.rs` |
| Client Registration | Complete | Admin endpoints |
| Token Introspection (RFC 7662) | Complete | `/oauth/introspect` |
| Token Revocation (RFC 7009) | Complete | `/oauth/revoke` |
| User Grant Management | Complete | `/oauth/grants` |
| Audit Logging | Complete | All OAuth actions |

**Key Files:**
- `backend/servers/api-server/src/routes/oauth.rs` (993 lines)
- `backend/servers/api-server/src/services/oauth.rs` (795 lines)
- `backend/crates/db/src/repositories/oauth.rs`
- `backend/crates/db/src/models/oauth.rs`

### OAuth Consumer (Partial)

External OAuth integrations are scaffolded but use placeholder tokens:

| Provider | Status | Location | Notes |
|----------|--------|----------|-------|
| Google Calendar | Scaffolded | `integrations/calendar.rs` | Auth URL, token exchange implemented |
| Microsoft Calendar | Scaffolded | `integrations/calendar.rs` | Parallel to Google |
| Airbnb | Scaffolded | `integrations/airbnb.rs` | OAuth config defined |
| Booking.com | Implemented | `integrations/booking/` | Dual-auth: credential-connect + OTA-XML (primary), plus OAuth token-exchange (see below) |
| Google Assistant | Stub | `routes/ai.rs:link_voice_device` | TODO comment for Phase 2 |
| Amazon Alexa | Stub | Voice device model | Not started |

## Phase 2 Requirements

### 1. External Calendar OAuth Flow

**Current State:**
```rust
// backend/crates/integrations/src/calendar.rs
pub async fn exchange_code(&self, code: &str) -> Result<OAuthTokens, CalendarError>
pub async fn refresh_token(&self, refresh_token: &str) -> Result<OAuthTokens, CalendarError>
```

**Required Work:**
1. Store encrypted tokens in database (use `IntegrationCrypto`)
2. Implement automatic token refresh before expiration
3. Handle token revocation by user
4. Add webhook for token invalidation notifications

**Database Changes:**
```sql
-- Already exists: integration_connections table
-- Add: token refresh scheduler job
```

### 2. Airbnb OAuth Integration

**Current State:**
```rust
// backend/crates/integrations/src/airbnb.rs
pub struct AirbnbOAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}
```

**Required Work:**
1. Implement Airbnb OAuth 2.0 flow (they use custom endpoints)
2. Store listing_id mapping after authorization
3. Handle webhook subscription setup post-auth
4. Implement sync scheduler for reservations

**API Endpoints Needed:**
- `GET /api/v1/integrations/airbnb/auth` - Initiate OAuth
- `GET /api/v1/integrations/airbnb/callback` - Handle redirect
- `POST /api/v1/integrations/airbnb/disconnect` - Revoke access
- `GET /api/v1/integrations/airbnb/status` - Check connection

### 2b. Booking.com Channel Integration (Implemented)

Unlike Airbnb, Booking.com is **not** connected through an Airbnb-style OAuth
*connect* handshake. Booking.com's real API model exposes two authentication
surfaces, and both are implemented in `backend/crates/integrations/src/booking/`:

1. **Credential connect + OTA-XML (primary).** The legacy Supply XML /
   Connectivity API authenticates each request with a per-hotel
   `hotel_id + username + password` over HTTP Basic auth — Booking.com issues
   long-lived machine credentials rather than running an authorization-code
   flow. Handled by `BookingClient` / `BookingCredentials`. The api-server
   `POST /booking/connect` route validates the credentials (by calling
   `fetch_property`) and stores them encrypted at rest (`IntegrationCrypto`,
   AES-256-GCM). Reservation pull/push and availability/rate sync ride this path
   as OpenTravel Alliance (OTA) XML messages.
2. **OAuth 2.0 token exchange (newer Connectivity APIs).** Booking.com's newer
   endpoints additionally offer an authorization-code grant, handled by
   `BookingOAuthClient` and the api-server `POST /booking/token/exchange` route
   (mirrors the Airbnb OAuth client; fail-closed 503 when `client_id` is unset).

**Existing endpoints:**
- `POST /api/v1/integrations/organizations/{org_id}/booking/connect` — credential connect
- `GET  /api/v1/integrations/organizations/{org_id}/booking/status` — connection status
- `POST /api/v1/integrations/organizations/{org_id}/booking/sync` — pull + persist reservations
- `POST /api/v1/integrations/organizations/{org_id}/booking/push-availability` / `.../push-rates`
- `POST /api/v1/integrations/organizations/{org_id}/booking/listing-push` — OTA availability+rate push
- `GET  /api/v1/integrations/organizations/{org_id}/booking/conflicts` — cross-platform overlap detection
- `POST /api/v1/integrations/organizations/{org_id}/booking/token/exchange` — OAuth code exchange
- `DELETE /api/v1/integrations/organizations/{org_id}/booking` — disconnect

The absence of an OAuth *connect* handshake on the primary path is therefore
intentional and correct — the credential-connect flow matches Booking.com's
Supply XML contract (Story 83.2 / coverage gap 83-2).

### 3. Voice Assistant OAuth (Google Assistant/Alexa)

**Current State:**
```rust
// backend/servers/api-server/src/routes/ai.rs:1654
// TODO: Phase 2 - Implement OAuth token exchange using auth_code from request.
// Current implementation (Phase 1) stores device linkage but doesn't handle OAuth tokens.
```

**Required Work:**
1. Implement Account Linking flow for Google Assistant
2. Implement Account Linking flow for Amazon Alexa
3. Store device tokens securely
4. Handle token refresh for long-lived devices
5. Implement skill/action manifest endpoints

**Security Considerations:**
- Voice devices need long-lived refresh tokens
- Device unlinking must revoke all tokens
- Rate limiting on voice commands

### 4. SSO Between Servers

**Current State:**
- api-server and reality-server share same database
- reality-server has SSO routes scaffolded
- Users can have accounts on both systems

**Required Work:**
1. Implement SSO token exchange between servers
2. Allow PM users to access Reality Portal with same credentials
3. Implement session synchronization
4. Handle role mapping between systems

**Flow:**
```
User (PM App) → api-server → SSO Token → reality-server → Reality Portal Session
```

## Security Requirements

### Token Storage
- All tokens encrypted at rest using `IntegrationCrypto`
- Environment variable: `INTEGRATION_ENCRYPTION_KEY`
- AES-256-GCM encryption

### Token Lifecycle
- Access tokens: 15 minutes (configurable)
- Refresh tokens: 7 days (configurable)
- Device tokens: 1 year (with refresh)

### Audit Logging
All OAuth actions must be logged:
- `OAuthAuthorize` - User grants access
- `OAuthRevoke` - User revokes access
- `OAuthTokenRefresh` - Token refreshed
- `OAuthClientCreate/Revoke` - Admin actions

## Environment Variables

| Variable | Description | Required |
|----------|-------------|----------|
| `GOOGLE_CLIENT_ID` | Google OAuth client ID | For calendar |
| `GOOGLE_CLIENT_SECRET` | Google OAuth client secret | For calendar |
| `MICROSOFT_CLIENT_ID` | Microsoft OAuth client ID | For calendar |
| `MICROSOFT_CLIENT_SECRET` | Microsoft OAuth client secret | For calendar |
| `AIRBNB_CLIENT_ID` | Airbnb API client ID | For rentals |
| `AIRBNB_CLIENT_SECRET` | Airbnb API client secret | For rentals |
| `INTEGRATION_ENCRYPTION_KEY` | Token encryption key | All integrations |

## Testing Requirements

1. **Unit Tests:** Token generation, PKCE verification, encryption
2. **Integration Tests:** Full OAuth flows with mock servers
3. **E2E Tests:** Calendar sync, Airbnb reservation sync

## References

- [RFC 6749 - OAuth 2.0](https://tools.ietf.org/html/rfc6749)
- [RFC 7636 - PKCE](https://tools.ietf.org/html/rfc7636)
- [RFC 7009 - Token Revocation](https://tools.ietf.org/html/rfc7009)
- [RFC 7662 - Token Introspection](https://tools.ietf.org/html/rfc7662)
- [Google Account Linking](https://developers.google.com/assistant/identity/google-sign-in-oauth)
- [Alexa Account Linking](https://developer.amazon.com/docs/account-linking/understand-account-linking.html)
