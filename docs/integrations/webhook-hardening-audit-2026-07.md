# Webhook Hardening Audit — timestamp / replay / idempotency parity

- **Date:** 2026-07-23
- **Owner role:** pm-integration
- **Trigger:** [#2485] — the outbound *layout* webhook lacked timestamp/replay
  protection; folded the timestamp into the signed payload
  (`"{timestamp}.{body}"`) + shipped `X-Webhook-Timestamp`. This audit treats
  that fix as the occasion for a cross-cutting review of **every** webhook
  handler for the same three properties.
- **Scope:** all inbound webhook *receivers* and outbound *notifiers* in
  `backend/` — booking, airbnb, esignature, layout, plus the portal and Stripe
  receivers they are meant to be at parity with.

This document is the audit deliverable. It does **not** change receiver
behaviour: two of the three gaps require provider-specific signing knowledge
(Booking.com) or gate on a currently-unmounted surface (e-signature), so the
concrete fixes are captured as scoped follow-ups (§4) rather than shipped as
speculative auth code here.

---

## 1. The three properties under review

| Property | What it defends | Canonical implementation |
| --- | --- | --- |
| **Signature** | Authenticity — the payload came from the real sender | HMAC-SHA256 over the raw body, constant-time compare, fail-closed on unset/empty secret |
| **Timestamp / replay freshness** | A *captured* valid delivery cannot be replayed indefinitely | `X-Webhook-Timestamp` folded into the signed payload (`"{timestamp}.{body}"`) + a ±300 s tolerance window so the timestamp cannot be swapped without invalidating the signature |
| **Idempotency / dedup** | An at-least-once *re*-delivery has no duplicate side effect | Persistent dedup ledger keyed on a delivery id, or a terminal-state / already-settled no-op guard |

The reference-quality inbound receiver is the **Stripe** payment webhook
(`handle_payment_webhook`) — it has all three. The **portal** receivers and the
**layout** outbound notifier were brought to parity in prior work
(#2330 / #833 / gap 83-3 / #2485).

---

## 2. Parity matrix

Files: receivers live in
`backend/servers/api-server/src/routes/integrations/webhook.rs` and
`.../routes/portal_webhooks.rs`; the outbound layout notifier is
`.../routes/layout/webhook.rs`; provider primitives are in
`backend/crates/integrations/`.

| Handler | Direction | Signature | Timestamp freshness | Idempotency / dedup | Mounted? | Side effects today |
| --- | --- | :---: | :---: | :---: | :---: | --- |
| **Stripe payments** `handle_payment_webhook` | inbound | yes (Stripe `t=`+`v1=` scheme, ts folded) | **yes** (±`DEFAULT_SIGNATURE_TOLERANCE_SECS`) | **yes** (session `completed` no-op + atomic settle claim) | yes | settles invoices |
| **Portal (connection-scoped)** `handle_portal_webhook` | inbound | yes | **yes** (mandatory, `verify_portal_webhook`) | n/a (parse-only, documented) | yes | none |
| **Portal (per-portal)** `portal_webhooks.rs` | inbound | yes | **yes** (staged accept-both, `verify_timestamped_portal_webhook`) | no — count-dedup (documented gap) | yes | persists views / inquiries |
| **Layout** `layout/webhook.rs` (**#2485**) | outbound | yes (`"{ts}.{body}"`) | **yes** (ships `X-Webhook-Timestamp`; reality-web enforces ±300 s) | n/a (sender) | yes | outbound notify |
| **Airbnb** `handle_airbnb_webhook` | inbound | yes (body-only HMAC) | **NO** freshness window | **yes** (`airbnb_webhook_events` ledger + synthetic key) | yes | enqueues sync jobs; cancels bookings |
| **Booking.com push** `booking_push_notification` | inbound | **NO — none** | **NO** | **NO** | yes | **none** (`_state` unused; parses OTA XML + acks) |
| **E-signature** `esignature_webhook` | inbound | yes (per-provider: DocuSign / Adobe Sign / HelloSign) | **NO** freshness window | **yes** (terminal-state no-op, `update_esignature_workflow_by_external_id`) | **NO — unmounted** (PAP-122) | would mutate workflow status |

---

## 3. Findings

### F1 — Booking.com push has no authentication at all — HIGH (latent)

`booking_push_notification` (route `/api/v1/integrations/booking/push`, live
and mounted) performs **no** signature check, **no** timestamp check, and
**no** dedup. Any caller can POST arbitrary OTA XML and receive a `200`
`<Success/>` ack.

It is **harmless today**: the handler takes `State(_state)` (unused), only
parses the OTA `OTA_HotelResNotifRQ` and returns an ack — zero state mutation.
The risk is entirely forward-looking: the day this receiver is wired to persist
reservations (the obvious next step, mirroring the Airbnb path that enqueues
`SYNC_EXTERNAL` jobs and cancels bookings) it becomes a fully unauthenticated
state-mutation endpoint.

Why it is not fixed in this audit: Booking.com's Connectivity/OTA **push**
authentication model must be confirmed against their current spec (the outbound
Supply-XML client uses HTTP Basic credentials — `BookingClient::generate_auth_header`
— which is *not* the same as an inbound HMAC signature scheme). Shipping a
fabricated `X-Booking-Signature` HMAC here would give false assurance and could
reject legitimate deliveries. Tracked as **R1**.

### F2 — Airbnb inbound lacks a timestamp-freshness window — MEDIUM (defense-in-depth)

`handle_airbnb_webhook` verifies a **body-only** HMAC
(`AirbnbClient::verify_webhook_signature`) — no `{timestamp}.{body}` binding and
no ±tolerance window, unlike Stripe and both portal receivers.

Replay is nonetheless **neutralized** by the persistent dedup ledger
(`airbnb_webhook_events`, `record_airbnb_webhook_event`, with a `synthetic:`
key derived from the signed fields when Airbnb omits `event_id`): a replayed
delivery maps to the same key and is suppressed with a `200` before any job is
enqueued. So this is a **parity / consistency** gap, not an exploitable replay
hole. Closing it (adopting the portal receiver's staged accept-both timestamp
verification) would make the replay posture uniform across receivers. Tracked
as **R2**.

### F3 — E-signature receiver lacks a timestamp-freshness window — MEDIUM (dead code)

`esignature_webhook` has per-provider signature verification and terminal-state
idempotency (`update_esignature_workflow_by_external_id` no-ops once a workflow
is `completed`/`voided`/`declined`), but **no** freshness window — a captured,
non-terminal delivery could be replayed within the signature's validity.

It is currently **UNMOUNTED** (PAP-122 — it writes the migration-less
`esignature_workflows` table and is absent from `router()`), so there is no live
exposure. The freshness window should be added as a **precondition of the
PAP-122 remount** (HelloSign already ships `event_time` in its signed hash;
DocuSign/Adobe would need their timestamp headers folded in). Tracked as **R3**.

### F4 — Layout outbound (#2485), Stripe, and portal receivers are compliant — INFO

The three properties are present (or correctly n/a for a sender). The layout
notifier — the fix that triggered this audit — signs `"{timestamp}.{body}"` and
ships `X-Webhook-Timestamp`; the reality-web receiver enforces a ±300 s window.
No action.

---

## 4. Recommended follow-ups

| Id | Action | Severity | Blocking condition |
| --- | --- | --- | --- |
| **R1** | Booking.com push: add fail-closed signature verification + a dedup ledger, mirroring the Airbnb/Stripe pattern. Requires confirming Booking's inbound OTA push auth scheme first. | HIGH | **Must land before** the receiver is wired to any state mutation. |
| **R2** | Airbnb inbound: adopt the portal receiver's staged accept-both `X-Webhook-Timestamp` freshness verification for full parity. | MEDIUM | None (dedup already covers replay); do opportunistically. |
| **R3** | E-signature: add a timestamp-freshness window (fold provider timestamps into the verified payload). | MEDIUM | Precondition of the PAP-122 remount. |
| **R4** | Extract one shared `verify_timestamped_signature(secret, ts, body, sig, now, tol)` helper into `backend/crates/integrations/` so all inbound receivers share a single reviewed implementation instead of the current near-duplicates (portal connection-scoped, per-portal, Stripe). | LOW (tech-debt) | None. |

## 5. Method / evidence

Reviewed the full set of `post(...)` webhook routes in
`routes/integrations/webhook.rs::router()`, `routes/portal_webhooks.rs::router()`,
`routes/layout/mod.rs`, and their handlers; the provider primitives in
`backend/crates/integrations/{airbnb,esignature,booking,portals}.rs`; and the
repository-layer idempotency guards in
`backend/crates/db/src/repositories/integration.rs`
(`update_esignature_workflow_by_external_id`) and the Airbnb dedup ledger. The
Stripe receiver (`services/stripe.rs` + `handle_payment_webhook`) and the portal
receivers are taken as the parity reference because they already carry all three
properties.
