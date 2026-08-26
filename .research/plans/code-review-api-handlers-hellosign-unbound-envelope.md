# code-review-api-handlers-hellosign-unbound-envelope

**Vector:** bug
**Score:** 3
**Source:** hotspot in backend/servers/api-server/src/routes/integrations/sync.rs:226
**Confidence:** high

## Hypothesis

`verify_hellosign_signature` (sync.rs:226-240) HMACs only `event_time + event_type`; the `envelope_id` the receiver then acts on (webhook.rs:882-884 / 906-967) is NOT part of the signed data. An attacker who obtains one valid HelloSign notification can replay it with any `envelope_id` substituted in the JSON body — the event_hash still verifies (it covers time+type only) and the org is resolved from the attacker-chosen envelope_id, so an arbitrary, cross-tenant e-signature workflow gets forged into a terminal signed/voided state. The DocuSign and Adobe branches are not affected: they `verify_slice` over the full raw `&body` (sync.rs:199, 211), which binds envelope_id. The smallest correct fix is to bind envelope_id into the authenticated set — either verify the full raw body (matching DocuSign/Adobe), or refuse to act on any payload field not covered by the provider signature, and reject when the resolved workflow's provider differs from the verified provider.

## Evidence

- `backend/servers/api-server/src/routes/integrations/sync.rs:226` — `verify_hellosign_signature(api_key, event_time, event_type, event_hash) -> bool`; L235: `mac.update(format!("{}{}", event_time, event_type).as_bytes());` — envelope_id absent from MAC input.
- `backend/servers/api-server/src/routes/integrations/webhook.rs:848-861` — HELLOSIGN branch validates the (time,type,hash) tuple.
- `backend/servers/api-server/src/routes/integrations/webhook.rs:882-884` — `esignature_webhook` then acts on the UNSIGNED `payload.envelope_id` to resolve the owning org and mark that workflow completed/voided/declined (L906-967).
- `backend/servers/api-server/src/routes/integrations/sync.rs:199,211` — DocuSign / Adobe verify_slice over the full raw `&body`, which binds envelope_id (contrast).
- `backend/crates/db/src/repositories/integration.rs:964-987` — idempotency guard (`WHERE external_envelope_id = $1 AND status <> ALL($terminal)`) only blocks re-transitioning an already-terminal workflow; a substituted, not-yet-terminal envelope_id passes and is updated.

## Files

- `backend/servers/api-server/src/routes/integrations/sync.rs:226`
- `backend/servers/api-server/src/routes/integrations/webhook.rs`
- `backend/crates/db/src/repositories/integration.rs`

## Dependencies

## Required capabilities

- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):**
Mode: cloud-ok

## Repro steps

1. Capture one valid HelloSign `signature_request_all_signed` webhook delivery for tenant A (headers: `X-Hellosign-Signature: <hex>`; body: `{"event": {"event_time":"...", "event_type":"signature_request_all_signed", "event_hash":"<hex>"}, "signature_request": {...}, "envelope_id":"env-A"}`).
2. POST the same headers + `event_time`/`event_type`/`event_hash` to `/api/v1/webhooks/esignature/hellosign`, but rewrite the JSON body so `envelope_id` points at tenant B's workflow: `"envelope_id":"env-B"`.
3. Expected: 401 (envelope_id not part of the signed set → signature invalid, or provider-mismatch rejection). Actual: 200; `esign_workflows.status` for env-B is flipped to `completed`/`voided`/`declined` per event_type.

## Suggested approach

1. Change `verify_hellosign_signature` in `sync.rs:226` to accept the raw request body (matching DocuSign/Adobe callers) and `mac.update(&body)` over the full body. Keep the `event_time`/`event_type` fields on the extracted `HelloSignEvent` struct for downstream logic, but drop them from the MAC input.
2. In `webhook.rs`, thread the raw body bytes into the HELLOSIGN branch (extract `Bytes` alongside the `Json<T>` extractor or re-serialize from the parsed payload only after signature verification — verifying over re-serialized JSON is unsafe, so lift the raw-body extractor to the top of `esignature_webhook`).
3. Add a provider-consistency guard just before the DB update at `webhook.rs:882-884`: resolve the workflow row by `external_envelope_id`, and if its `provider` column ≠ the verified branch (`ESignatureProvider::HelloSign`), return 401 without mutation. This defends against future MAC gaps and mixed-provider payloads.
4. Add a bounded freshness check: reject the delivery if `now_utc - event_time > 5 min` tolerance (this partly overlaps with the sibling `code-review-api-handlers-esign-webhook-replay` plan; keep the check here so it lands with the primary fix, and the sibling plan can broaden it to DocuSign/Adobe).
5. Add integration tests under `backend/servers/api-server/tests/` — see *Test plan* — covering (a) valid-signature happy path, (b) substituted-envelope forgery rejected, (c) provider-mismatch rejected, (d) expired-timestamp rejected.
6. Do not change the storage schema; the existing `esign_workflows.provider` column already carries the needed metadata.
7. Do not touch DocuSign / Adobe verification code — they already bind the body.

## Alternatives considered

- **Reject the payload's `envelope_id` in favour of a URL-scoped `/webhooks/esignature/hellosign/{envelope_id}`** — rejected because the HelloSign dashboard configures a single webhook URL globally; requiring per-envelope URLs is not supported by the provider.
- **Persist a nonce/dedupe key derived from event_hash and reject repeats** — rejected because it patches the replay symptom without closing the forgery gap: the *first* delivery of a forged (event_hash, substituted_envelope_id) pair still succeeds.

## Root-cause trace

1. Symptom: attacker replays a captured HelloSign delivery with a substituted `envelope_id` and drives another tenant's workflow into `completed`/`voided`/`declined`.
2. ← immediate cause at `backend/servers/api-server/src/routes/integrations/webhook.rs:882-884` — `esignature_webhook` trusts `payload.envelope_id` to resolve the workflow row before checking whether the signature covered that field.
3. ← upstream cause at `backend/servers/api-server/src/routes/integrations/sync.rs:235` — the HMAC is computed over only `event_time + event_type`; `envelope_id` was omitted when the HelloSign helper was first added, breaking parity with the DocuSign/Adobe helpers that verify the full body.
4. Origin: commit that introduced the current HelloSign verify helper (predates 2026-05); the file has evolved without the receiver ever binding envelope_id to the signature.

## Test plan

- [ ] `backend/servers/api-server/tests/esign_hellosign_envelope_binding_tests.rs` — new integration test: valid signature + substituted envelope_id → 401 and target workflow untouched.
- [ ] `backend/servers/api-server/tests/esign_hellosign_provider_mismatch_tests.rs` — new: HelloSign-verified payload naming a DocuSign-provider workflow → 401.
- [ ] `backend/servers/api-server/tests/esign_hellosign_replay_freshness_tests.rs` — new: valid signature but `event_time` > 5 min old → 401.
- [ ] Existing HelloSign happy-path test in the same suite must still pass (drives the migration from field-tuple MAC to full-body MAC).
- [ ] Run: `cargo test -p api-server --test 'esign_hellosign_*'` and `cargo test -p api-server esign` for the broader receiver suite.

## Out of scope

- Broadening the freshness check to DocuSign and Adobe (covered by `code-review-api-handlers-esign-webhook-replay`).
- Reworking the `esign_workflows` provider column or the idempotency guard in `integration.rs:964-987`.
- Any HelloSign SDK / signature-computation change outside `verify_hellosign_signature`.

## After-merge

- Move this file to `plans/_archive/code-review-api-handlers-hellosign-unbound-envelope.md`
- Mark the matching `backlog.json` row as `status: "done"`
