# code-review-api-handlers-hellosign-unbound-envelope

**Vector:** security
**Score:** 3
**Source:** commit 6f5e7cf (HEAD as of 2026-08-27) · signals/2026-08-26-api-handlers-tier1d.json
**Confidence:** medium

## Hypothesis

The HelloSign branch of the e-signature webhook authenticates only `event_time + event_type` in the HMAC, leaving `envelope_id` (and every other JSON field the handler acts on) unsigned. An attacker who captures one valid HelloSign notification can replay it with an arbitrary substituted `envelope_id`; the signature still verifies and the handler resolves the owning org from the attacker-chosen envelope, forcing a foreign tenant's e-signature workflow into a terminal signed/voided/declined state. The fix is to include the full raw request body in the HelloSign HMAC input (mirroring the DocuSign/Adobe paths in the same file) and to reject payloads whose resolved workflow.provider disagrees with the verified provider.

## Evidence

- `backend/servers/api-server/src/routes/integrations/sync.rs:226` — `verify_hellosign_signature()` MACs only `event_time + event_type` (`mac.update(format!("{}{}", event_time, event_type))`, L235); envelope_id is NOT covered.
- `backend/servers/api-server/src/routes/integrations/webhook.rs:848-861` — HELLOSIGN branch validates that tuple, then `esignature_webhook` acts on the UNSIGNED `payload.envelope_id` (L882-884) to resolve the owning org and mark that workflow completed/voided/declined (L906-967).
- DocuSign / Adobe branches at `sync.rs:199/211` `verify_slice` over the full raw `&body`, so they bind `envelope_id`. The asymmetry is the smoking gun.
- Idempotency guard at `backend/crates/db/src/repositories/integration.rs:964-987` only refuses to re-transition an already-terminal workflow — a substituted, not-yet-terminal envelope_id passes through and is updated.
- Signal: `code-review-api-handlers-hellosign-unbound-envelope` (tier-1d rotating expert review, api-handlers segment, 2026-08-26).

## Files

- `backend/servers/api-server/src/routes/integrations/sync.rs`
- `backend/servers/api-server/src/routes/integrations/webhook.rs`
- `backend/crates/db/src/repositories/integration.rs`

## Dependencies

_none_

## Required capabilities

- [x] C1 — Systematic debugging (bug/security)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps

1. Capture (or synthesize under the shared HelloSign API key) a real HelloSign `event_hash` for a benign event on some workflow-A envelope (event_time T, event_type `signature_request_all_signed`) — the hash is `HMAC_SHA256(api_key, event_time || event_type)`.
2. Send `POST /integrations/webhooks/hellosign` with headers `X-Hellosign-Event-Time: T`, `X-Hellosign-Event-Type: signature_request_all_signed`, `X-Hellosign-Event-Hash: <same hash>` and a JSON body whose `envelope_id` names an unrelated tenant B's active e-signature workflow.
3. Expected: 401 / 403 / 400 — envelope_id is not covered by the signature.
4. Actual (today, on `dev` HEAD 6f5e7cf): 200; the target workflow is transitioned into the terminal state named by the event_type, and the tenant-B org is charged the state transition.

## Suggested approach

1. In `backend/servers/api-server/src/routes/integrations/sync.rs:220-240`, replace `mac.update(format!("{}{}", event_time, event_type))` with `mac.update(&body)` (the raw request bytes), matching `verify_docusign_signature` / `verify_adobe_signature` upstream in the same file. Update the doc-comment on `verify_hellosign_signature` to state that the raw body is now signed.
2. Propagate the raw body into the HelloSign call site in `backend/servers/api-server/src/routes/integrations/webhook.rs:848-861` (the handler already has the raw body in scope for the other providers — pass the same `&body` slice).
3. Add a defence-in-depth check in `esignature_webhook` after workflow resolution: if the DB-loaded workflow's `provider` disagrees with the verified provider from the header set, respond 400 and log; that catches any future signature-scope regression on any provider, not only HelloSign.
4. Add a `#[sqlx::test]` regression under `backend/servers/api-server/tests/` (new file `integrations_hellosign_envelope_bind.rs`) that seeds two orgs with an e-sig workflow each, computes a valid signature for org-A's event, submits it with org-B's envelope_id in the body, and asserts the request is rejected AND org-B's workflow status is unchanged. Add a positive counterpart that succeeds when envelope_id matches the signed body.
5. Sweep the same file for `verify_*` MAC inputs and grep `mac.update(format!` to catch any other provider that MACs a concatenation instead of raw bytes; document the sweep result in the PR.
6. Run `cargo test -p api-server integrations_hellosign_envelope_bind` locally to confirm the failing-on-main case turns green after the sync.rs change.
7. Update the HelloSign integration doc under `docs/api/` (search for the current mention of the webhook contract) so future authors know the raw body is signed.

## Alternatives considered

- **Refuse to act on any payload field not covered by the provider signature (whitelist)** — rejected because HelloSign only ships one signature covering the header tuple; the handler *has* to read fields from the body to route the event. Widening the signed set is strictly stronger.
- **Add an application-layer HMAC over the body with a second shared secret** — rejected because HelloSign does not offer a second-secret channel, so this would either require adopters to double-configure or leave the second secret unused. The provider's own signature-over-body is available via raw-body MAC and closes the gap without new infrastructure.

## Root-cause trace

1. Symptom: cross-tenant e-signature workflow forced into terminal state by a replayed webhook.
2. ← `esignature_webhook` at `backend/servers/api-server/src/routes/integrations/webhook.rs:882-884` reads `payload.envelope_id` and uses it to resolve the org, without confirming that field was covered by the provider signature.
3. ← `verify_hellosign_signature` at `backend/servers/api-server/src/routes/integrations/sync.rs:235` MACs `event_time + event_type` only — an implicit contract mismatch with the two sibling providers (docusign / adobe) which sign the raw body.
4. Origin: the HelloSign branch was added later than DocuSign/Adobe and reused the header tuple pattern from HelloSign's docs without extending the signature scope to the body; git blame on `sync.rs` around the `verify_hellosign_signature` block will name the introducing PR.

## Test plan

- [ ] `backend/servers/api-server/tests/integrations_hellosign_envelope_bind.rs` — the failing-on-main test described in *Repro steps*.
- [ ] Positive test: legitimate HelloSign notification (body envelope_id matches sender's org) still transitions the workflow.
- [ ] `cargo test -p api-server integrations_hellosign_envelope_bind` — runs both cases; must be red on `dev` before the fix and green after.
- [ ] `cargo test -p api-server --tests integrations::` — smoke the rest of the integrations suite to catch regressions in the shared verify helpers.

## Out of scope

- DocuSign / Adobe signature-scope changes — they already sign the raw body.
- Rotating the HelloSign shared secret — an ops task, not a code change.
- Per-provider rate limiting on the webhook endpoint.
- Backfill / audit of any historical cross-tenant transitions this bug may have permitted; that is a security-incident response question for a separate ticket.

## After-merge

- Move this file to `plans/_archive/code-review-api-handlers-hellosign-unbound-envelope.md`
- Mark `code-review-api-handlers-hellosign-unbound-envelope` in `backlog.json` as `status: "done"`
