# code-review-api-handlers-hellosign-unbound-envelope

**Vector:** security
**Score:** 3
**Source:** Tier-1d api-handlers review 2026-08-26 (rotating-expert dev-review, security lens)
**Confidence:** high

## Hypothesis
`verify_hellosign_signature` (`backend/servers/api-server/src/routes/integrations/sync.rs:226`) MACs only `event_time + event_type`, so the HelloSign branch of `esignature_webhook` (`backend/servers/api-server/src/routes/integrations/webhook.rs:848-861`) authenticates a tuple that is disjoint from the field it acts on. `esignature_webhook` resolves the owning org and target workflow from the UNSIGNED `payload.envelope_id` (L882-884) and then drives it to a terminal state (`completed`/`voided`/`declined`, L906-967). An attacker who captures one valid HelloSign notification (e.g. an `all_signed` for their own tenant) can replay it with an arbitrary substituted `envelope_id` in the JSON body — the event_hash still verifies (only covers time+type) and the workflow row is looked up from the attacker-chosen id, forging cross-tenant e-signature completion. The DocuSign and Adobe branches verify the full raw body (`sync.rs:199/211`), so this is HelloSign-only. Fix binds `envelope_id` into the authenticated set — either MAC the full raw body like DocuSign/Adobe, or refuse to act on any payload field not covered by the provider signature, plus reject when the resolved workflow's provider != the verified provider.

## Evidence
- `backend/servers/api-server/src/routes/integrations/sync.rs:226` — `verify_hellosign_signature` MACs only `format!("{}{}", event_time, event_type)`; envelope_id is not part of the signed data.
- `backend/servers/api-server/src/routes/integrations/webhook.rs:848` — HELLOSIGN branch validates `(event_time, event_type, event_hash)` then falls through to `esignature_webhook` which reads `payload.envelope_id` (L882-884) unauthenticated.
- `backend/servers/api-server/src/routes/integrations/webhook.rs:906-967` — the workflow row is resolved from the attacker-chosen `envelope_id` and marked completed/voided/declined per `payload.event_type`.
- `backend/crates/db/src/repositories/integration.rs:964-987` — idempotency guard `WHERE external_envelope_id=$1 AND status <> ALL($terminal)` only blocks re-transitioning already-terminal rows; a substituted, not-yet-terminal `envelope_id` passes.
- `backend/servers/api-server/src/routes/integrations/sync.rs:199,211` — DocuSign/Adobe verify the full raw body via `verify_slice(&body)`; not affected.

## Files
- `backend/servers/api-server/src/routes/integrations/sync.rs:226`
- `backend/servers/api-server/src/routes/integrations/webhook.rs:747`
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
1. Provision two orgs A and B, each with a HelloSign integration configured (same shared `api_key` at the platform level, distinct per-org integration rows). Create a workflow for org A whose `external_envelope_id = env-A`.
2. Attacker (org B) sends a legitimate signed HelloSign notification (`event_time`, `event_type=signature_request_all_signed`, `event_hash` valid) with body `{ "envelope_id": "env-B", ... }` and workflow row for env-B in state `pending`.
3. Attacker replays the SAME headers/hash/time to the api-server, but rewrites the body's `envelope_id` to `env-A` (org A's envelope, still pending).
4. Expected: request rejected with 401 INVALID_SIGNATURE.
5. Actual (current code): `verify_hellosign_signature(api_key, event_time, event_type, event_hash)` still returns true (envelope_id is not part of the MAC input), `esignature_webhook` resolves the workflow from `env-A`, marks org A's workflow `completed` in the DB, and returns 200. Cross-tenant terminal-state forgery.

## Suggested approach
1. **Prefer full-body MAC parity** — extend `verify_hellosign_signature` (`sync.rs:226`) to accept `&body: &[u8]` and either (a) MAC the full raw request body like DocuSign/Adobe (`verify_slice(&body).is_ok()`), or (b) if HelloSign's own scheme is truly `event_time+event_type`, additionally require that the incoming `payload.envelope_id` is present and MAC in a canonical `event_time||event_type||envelope_id` form and cross-check. Option (a) aligns three of four providers on one shape and is the smallest deviation.
2. In `webhook.rs:848-861`, thread the raw body bytes into `verify_hellosign_signature` and abort with 401 before touching `payload.envelope_id`.
3. Add a provider-consistency gate in `esignature_webhook` after resolving the workflow: assert the workflow's stored `provider` equals the verified request provider; on mismatch return 400 (defence in depth for the moment where a plaintext envelope_id from provider X points at a workflow owned by provider Y).
4. Extend `integration.rs:964-987`'s idempotency layer to also record a delivery id (or hash of raw body) so an already-processed delivery is a no-op even if replayed with a different `envelope_id` re-mapping (belt-and-braces).
5. Add TypeSpec / OpenAPI comment noting the tightened contract; no client-visible change.

## Alternatives considered
- **Reject any HelloSign body that references an `envelope_id` not equal to the one implied by the signed set** — rejected because HelloSign's docs only sign `event_time+event_type`; there is no signed `envelope_id` to compare against, so we would be inventing an assertion the provider can never satisfy. Full-body MAC (approach 1a) is the cleaner alignment.
- **Refuse HelloSign entirely and disable the provider until upstream signs the envelope** — rejected because DocuSign/Adobe already show the correct pattern (full-body HMAC) and switching HelloSign to full-body verification is a one-file change; disabling the provider is a much larger customer-facing regression than the fix itself.

## Root-cause trace
1. Symptom: an attacker with a single valid HelloSign notification can drive any tenant's HelloSign workflow to `completed`/`voided`/`declined` by rewriting `envelope_id` in the body.
2. ← `esignature_webhook` in `backend/servers/api-server/src/routes/integrations/webhook.rs:882` reads `payload.envelope_id` from the JSON body and uses it to look up + mutate the workflow row.
3. ← `verify_hellosign_signature` in `backend/servers/api-server/src/routes/integrations/sync.rs:226` MACs `event_time+event_type` only — the field `esignature_webhook` acts on is not in the authenticated set.
4. Origin: the HelloSign branch was added as a shorter shape than DocuSign/Adobe's full-body verify. The two contracts diverged at the moment the third provider landed; no test asserts that the field driving the state change is inside the signed set.

## Test plan
- [ ] Add `backend/servers/api-server/tests/suites/esignature_webhook_hellosign_envelope_binding_tests.rs`: seed workflows for org A (`env-A`) and org B (`env-B`) with the same HelloSign integration secret; construct a valid HelloSign signature for a body targeting `env-B`; POST the same signature headers with a body targeting `env-A`; assert 401 INVALID_SIGNATURE and that org A's workflow status is unchanged.
- [ ] Positive control test in the same file: a body whose `envelope_id` matches the one implied at signature time still succeeds and drives its own workflow to `completed`.
- [ ] Run `cargo test -p api-server --test suite_8` (or the newly-created suite): reproduces the failure on `dev` before the fix, passes after.

## Out of scope
- DocuSign / Adobe / Portal / Airbnb receivers — they already MAC the full body or have signed timestamps and are not affected by this envelope-binding gap.
- Adding replay-tolerance windows for the other providers (tracked separately as `code-review-api-handlers-esign-webhook-replay`).
- Removing HelloSign as a provider; rewriting the JSON payload shape.

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-hellosign-unbound-envelope.md`
- Mark the matching `backlog.json` row as `status: "done"`
