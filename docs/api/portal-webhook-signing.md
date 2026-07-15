# Portal webhook signing (inbound)

Partner-facing reference for signing webhooks POSTed to the PPT inbound portal
receivers. Both receiver families verify authenticity **and** freshness before
the payload is parsed or acted on; unverified or replayed deliveries are never
processed.

## Connection-scoped receiver — `POST /api/v1/integrations/webhooks/portal/{connection_id}`

Signing secret: `PORTAL_WEBHOOK_SECRET` (shared, out of band at connection time).

Every delivery MUST include two headers:

| Header | Value |
|--------|-------|
| `X-Webhook-Timestamp` | Unix time in **seconds** at send time. |
| `X-Webhook-Signature` | Hex-encoded HMAC-SHA256, optionally `sha256=`-prefixed. |

The signed payload is `"{X-Webhook-Timestamp}.{raw_body}"` — the timestamp,
a literal `.`, then the exact request body bytes:

```
signed_payload = "{timestamp}.{body}"
signature      = hex( HMAC_SHA256(PORTAL_WEBHOOK_SECRET, signed_payload) )
```

Rejection rules (all return an opaque `401 INVALID_SIGNATURE`):

- Missing/non-numeric `X-Webhook-Timestamp`.
- `|server_now - timestamp| > 300s` (replay / clock-skew window, gap 83-3).
- Missing signature, or a signature that does not match the HMAC over
  `"{timestamp}.{body}"`.

Because the timestamp is folded **into** the signed material, it cannot be
refreshed independently of the signature: a captured delivery cannot be
replayed with a new timestamp to beat the window.

### Example (pseudocode)

```
ts   = now_unix_seconds()
sig  = hex(hmac_sha256(secret, ts + "." + body))
POST /api/v1/integrations/webhooks/portal/{connection_id}
  X-Webhook-Timestamp: {ts}
  X-Webhook-Signature: sha256={sig}
  {body}
```

## Per-portal receivers — `POST /api/v1/webhooks/portals/{portal}/...`

`{portal}` ∈ `reality-portal`, `sreality`, `bezrealitky`, `nehnutelnosti`.
Signing secret: the per-portal `<PORTAL>_WEBHOOK_SECRET` (e.g.
`SREALITY_WEBHOOK_SECRET`). Signature header name varies by portal
(`X-Webhook-Signature`, `X-SReality-Signature`, `X-BR-Signature`,
`X-Nehnutelnosti-Signature`). These signatures are **base64**-encoded
HMAC-SHA256.

Replay protection here is being rolled out in an **accept-both** phase
(issue #2330):

- **Preferred / replay-protected:** include `X-Webhook-Timestamp` and sign
  `"{timestamp}.{body}"` (base64). The ±300s freshness window is enforced.
- **Legacy (deprecated):** no timestamp header, signature over the bare body.
  Still accepted for now (with a server-side deprecation warning), but has **no**
  replay protection. Migrate to the timestamped scheme.

Once all senders are migrated the legacy body-only path will be removed and
`X-Webhook-Timestamp` will become mandatory on these routes too.
