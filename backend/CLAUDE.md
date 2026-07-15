# Backend - CLAUDE.md

> **Parent:** See root `CLAUDE.md` for namespace and architecture.

## Overview

Rust backend with Axum framework. Contains two servers sharing common crates.

## Servers

| Server | Port | Purpose |
|--------|------|---------|
| api-server | 8080 | Property Management API |
| reality-server | 8081 | Reality Portal public API |

## Quick Commands

```bash
# Build all
cargo build

# Build release
cargo build --release

# Run api-server
cargo run -p api-server

# Run reality-server
cargo run -p reality-server

# Run tests
cargo test --workspace

# Format
cargo fmt --all

# Lint
cargo clippy --workspace -- -D warnings

# Check
cargo check --workspace
```

## Workspace Structure

```
backend/
├── Cargo.toml           # Workspace root
├── crates/              # Shared libraries (see crates/CLAUDE.md)
│   ├── common/
│   ├── api-core/
│   ├── db/
│   └── integrations/
└── servers/             # Backend servers (see servers/CLAUDE.md)
    ├── api-server/
    └── reality-server/
```

## Dependencies

Key workspace dependencies:
- `axum` - Web framework
- `tokio` - Async runtime
- `sqlx` - Database
- `utoipa` - OpenAPI generation
- `serde` - Serialization
- `tracing` - Logging

## Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `DATABASE_URL` | Yes | PostgreSQL connection string |
| `JWT_SECRET` | Yes | Secret key for JWT signing (min 32 chars) |
| `RUST_LOG` | No | Log level (default: info) |
| `CORS_ALLOWED_ORIGINS` | No | Comma-separated list of allowed CORS origins |
| `PORTAL_WEBHOOK_SECRET` | If receiving integration-connection portal webhooks | HMAC secret for the inbound portal webhook receiver at `/api/v1/integrations/webhooks/portal/{connection_id}`. Signature (`X-Webhook-Signature`, hex HMAC-SHA256, optionally `sha256=`-prefixed) is computed over `"{X-Webhook-Timestamp}.{body}"` — senders MUST send `X-Webhook-Timestamp` (unix seconds); deliveries outside ±300s are rejected `401 INVALID_SIGNATURE` (replay defense, gap 83-3). Fails closed (500 `CONFIG_ERROR`) when unset. See `docs/api/portal-webhook-signing.md`. |
| `AIRBNB_WEBHOOK_SECRET` | If receiving Airbnb webhooks | HMAC secret for the inbound Airbnb webhook receiver; fails closed (500 `NOT_CONFIGURED`) when unset |
| `STRIPE_WEBHOOK_SECRET` | If receiving Stripe webhooks | Signing secret for the Stripe payment-confirmation webhook receiver; fails closed (503 `NOT_CONFIGURED`) when unset |
| `REALITY_PORTAL_WEBHOOK_SECRET` | If receiving reality-portal webhooks | Per-portal HMAC secret for `/api/v1/webhooks/portals/reality-portal/...`; **distinct from `PORTAL_WEBHOOK_SECRET`** — this family backs the per-portal receiver in `routes/portal_webhooks.rs`, not the integration-connection receiver. Signature is base64 HMAC-SHA256. Replay protection (issue #2330) is **accept-both**: senders that include `X-Webhook-Timestamp` are verified over `"{timestamp}.{body}"` with a ±300s window; legacy body-only signatures are still accepted (with a deprecation warning) until senders migrate. Fails closed (`401`) when unset. |
| `SREALITY_WEBHOOK_SECRET` | If receiving Sreality webhooks | Per-portal HMAC secret for `/api/v1/webhooks/portals/sreality/...`; same family as above |
| `BEZREALITKY_WEBHOOK_SECRET` | If receiving Bezrealitky webhooks | Per-portal HMAC secret for `/api/v1/webhooks/portals/bezrealitky/...`; same family as above |
| `NEHNUTELNOSTI_WEBHOOK_SECRET` | If receiving Nehnutelnosti webhooks | Per-portal HMAC secret for `/api/v1/webhooks/portals/nehnutelnosti/...`; same family as above |

```bash
# Required
DATABASE_URL=postgres://user:pass@localhost:5432/ppt
JWT_SECRET=your-secure-random-secret-key-min-32-chars

# Optional
RUST_LOG=debug
CORS_ALLOWED_ORIGINS=https://example.com,https://api.example.com
```

### CORS Configuration

Both servers support configurable CORS origins via the `CORS_ALLOWED_ORIGINS` environment variable.

**Format:** Comma-separated list of origins (e.g., `https://example.com,https://api.example.com`)

**Default origins (if not set):**

| Server | Default Origins |
|--------|-----------------|
| api-server | localhost:3000, localhost:3001, localhost:8080, localhost:8081, ppt.three-two-bit.com, reality.three-two-bit.com |
| reality-server | localhost:3000, localhost:3001, localhost:8080, localhost:8081, ppt.three-two-bit.com, reality.three-two-bit.com, reality-portal.sk, reality-portal.cz, reality-portal.eu |

> **Security:** `JWT_SECRET` has no fallback. Server will fail to authenticate requests if not set.
