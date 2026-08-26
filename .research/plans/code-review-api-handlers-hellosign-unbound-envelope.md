# code-review-api-handlers-hellosign-unbound-envelope

**Vector:** security
**Score:** 3
**Source:** signals/2026-08-26-api-handlers-tier1d.json (rotating-expert-review, security lens)
**Confidence:** high

## Hypothesis
The HelloSign branch of the e-signature webhook verifies only `event_time + event_type` (`verify_hellosign_signature`, sync.rs:226), leaving `payload.envelope_id` unauthenticated. The receiver then resolves the owning org from that same unsigned field (webhook.rs:882-884) and drives the workflow into a terminal state (webhook.rs:906-967). An attacker who intercepts a single legitimate HelloSign event body can substitute an arbitrary `envelope_id` and forge a cross-tenant workflow state change; the DocuSign / Adobe branches use `verify_slice(&body)` and are not affected. Fix: bind `envelope_id` into the authenticated set — either verify the full raw body (matching DocuSign / Adobe) or reject payload fields not covered by the provider signature; also assert that the resolved workflow's `provider` equals the verified provider.

## Evidence
- `backend/servers/api-server/src/routes/integrations/sync.rs:226` — `verify_hellosign_signature()` MACs only `event_time + event_type` (`mac.update(format!("{}{}", event_time, event_type))`); envelope_id is NOT covered.
- `backend/servers/api-server/src/routes/integrations/webhook.rs:848-861` — HELLOSIGN branch calls that verifier then acts on `payload.envelope_id` (webhook.rs:882-884) with `find_esignature_workflow_by_external_id(envelope_id)` and drives status (webhook.rs:906-967).
- Contrast `sync.rs:199` (`verify_docusign_signature(secret, payload, signature)`) and `sync.rs:211` (`verify_adobe_sign_signature`) — both `verify_slice(&body)` over the whole raw body, so envelope_id inside the body is bound.
- The idempotency guard in `backend/crates/db/src/repositories/integration.rs:964-987` (`WHERE external_envelope_id = $1 AND status <> ALL($terminal)`) does NOT block substitution: a not-yet-terminal envelope id still passes.
- Comment in webhook.rs (~L915) calls the envelope_id "provider-signed" — that is only true for DocuSign / Adobe today; enforcing it for HelloSign is the fix.

## Files
- `backend/servers/api-server/src/routes/integrations/sync.rs:226`
- `backend/servers/api-server/src/routes/integrations/webhook.rs:848`
- `backend/crates/db/src/repositories/integration.rs:964`

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
1. Configure a HelloSign integration for org A and obtain one legitimate webhook body (any `signature_request_*` event). Note its `event_time`, `event_type`, `event_hash`.
2. Create an unrelated in-progress e-signature workflow in org B; note its `external_envelope_id`.
3. Replay the org-A body against `POST /api/v1/integrations/webhooks/{workflow_id}?provider=hellosign` but rewrite `payload.envelope_id` to org B's envelope id. Leave `event_time`, `event_type`, `event_hash` unchanged.
4. Expected: 401/403 (envelope_id not covered by signature; forged payload rejected). Actual: 2xx — the receiver resolves org B's workflow via `find_esignature_workflow_by_external_id` and drives its status to `completed` / `voided` / `declined` per `event_type`, changing state across tenants.

## Suggested approach
1. In `backend/servers/api-server/src/routes/integrations/sync.rs`, add a second HelloSign verifier that MACs the raw request body (`verify_hellosign_body_signature(api_key, body, event_hash)`) — mirroring `verify_docusign_signature` — and keep `verify_hellosign_signature` deprecated for callers still on the tuple form.
2. In `backend/servers/api-server/src/routes/integrations/webhook.rs` (HELLOSIGN branch at :848), thread the raw `&body` through and call the body-signed verifier. Keep the current tuple-signed check as an additional freshness input (see step 3) but do not rely on it for envelope_id binding.
3. In the same HELLOSIGN branch, apply the freshness check tracked by the sibling `code-review-api-handlers-esign-webhook-replay` finding: reject when `event_time` differs from server time by more than `HELLOSIGN_WEBHOOK_TOLERANCE_SECS` (env, default 300s, matching `BOOKING_WEBHOOK_TOLERANCE_SECS`). Scope creep is intentional — the two findings share the same call path and are cheap to fix together; keep them behind a small `#[cfg(test)]` seam so the freshness change lands with its own repro.
4. In the resolver after `find_esignature_workflow_by_external_id`, assert `workflow.provider == verified_provider` (the `provider` bound at webhook.rs router). On mismatch return `403 PROVIDER_MISMATCH`. This is defense-in-depth for any future receiver that still exposes an unsigned envelope id.
5. Update the misleading comment near webhook.rs:915 — "provider-signed envelope id" is now true across all three receivers; keep the note but drop the trap.
6. Regression tests as in the Test plan.
7. Manual smoke via `stack up pm-local`: pipe a legitimate HelloSign body through the receiver twice — once unmodified (expect 2xx), once with the envelope-id rewrite (expect 401/403).

## Alternatives considered
- **Verify only `event_time + envelope_id + event_type` (three-field tuple).** — rejected because it drifts from the DocuSign / Adobe body-hash pattern, still leaves other payload fields unauthenticated (e.g. `signer_email`, `signature_request_id` used for logging/audit), and locks the fix into HelloSign's ad-hoc tuple form; a full-body HMAC is the industry pattern and matches sibling receivers.
- **Enforce provider-mismatch on the resolved workflow but keep the tuple signature.** — rejected because provider-mismatch stops cross-provider substitution but NOT cross-tenant substitution within HelloSign itself (an attacker with a valid HelloSign event can still forge any HelloSign workflow's terminal state). The binding of envelope_id into the signature is the root fix; provider-mismatch is a cheap belt-and-braces addition.

## Root-cause trace
1. Symptom: an attacker replay of a valid HelloSign event body with a substituted `envelope_id` drives an unrelated tenant's `esignature_workflows` row to `completed` / `voided` / `declined`.
2. ← webhook.rs:906-967 applies `new_status` to the row returned by `find_esignature_workflow_by_external_id(envelope_id)` — envelope_id comes from the parsed JSON body, not from anything the signature covers.
3. ← webhook.rs:852 the HELLOSIGN branch verifies `verify_hellosign_signature(&api_key, event_time, event_type, event_hash)` — envelope_id is not passed in.
4. ← sync.rs:235 the MAC input is only `format!("{}{}", event_time, event_type)`, matching HelloSign's documented "event_hash" contract for that field tuple but leaving the rest of the payload unauthenticated when we chose to consume it.
5. Origin: `verify_hellosign_signature` and the HELLOSIGN branch predate the DocuSign / Adobe move to `verify_slice(&body)` and were never revised when the receiver started resolving org via envelope_id.

## Test plan
- [ ] `backend/servers/api-server/tests/suites/esignature_webhook_hellosign_envelope_binding_tests.rs` — new integration suite (wired into `backend/servers/api-server/tests/suite_4.rs` next to `esignature_webhook_idempotency_tests`): with a real Postgres via the harness, seed two workflows in different orgs, replay a valid HelloSign body against org A's endpoint with org B's envelope id substituted, assert 401/403 and that org B's row is unchanged (status + updated_at).
- [ ] Second case in the same suite: legitimate body, unmodified — assert 2xx and org A's row transitions to the expected terminal state (the happy path must not regress).
- [ ] Third case: valid body whose `event_time` is > `HELLOSIGN_WEBHOOK_TOLERANCE_SECS` in the past — assert 401 (pins the freshness check from step 3 above).
- [ ] Unit test for `verify_hellosign_body_signature` in sync.rs's `#[cfg(test)]` block (matches the shape of `verify_docusign_signature` tests if present, or the pattern in `esignature_nonce_replay_tests`).
- [ ] Local: `cargo test -p api-server --test suite_4 esignature_webhook_hellosign` (from `backend/`).
- [ ] Local: `cargo test -p api-server --test suite_4` full to catch regressions in the sibling `esignature_webhook_idempotency_tests` suite.

## Out of scope
- DocuSign / Adobe verifier changes — they already MAC the raw body.
- Rework of the idempotency guard (`integration.rs:964-987`) — the guard is correct for its stated job (block terminal re-transition); envelope-id binding is the right layer for this fix.
- HelloSign event-type coverage widening (adding new `event_type` → status mappings) — the fix does not add or remove any status transitions.
- The sibling `code-review-api-handlers-esign-webhook-replay` finding for DocuSign / Adobe replay windows — a separate plan will land those. Only the HelloSign freshness window is included here because it shares the same call path.

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-hellosign-unbound-envelope.md`
- Mark the matching `backlog.json` row as `status: "done"`
