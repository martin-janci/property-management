# code-review-api-handlers-hellosign-unbound-envelope

**Vector:** security
**Score:** 3
**Source:** Tier-1d api-handlers review 2026-08-26 (dispatcher)
**Confidence:** medium

## Hypothesis
`AuthPolicyEnforcer::verify_hellosign_signature` only MACs `event_time + event_type`, leaving the JSON body's `envelope_id` unsigned. The webhook handler then acts on the unsigned `envelope_id` to resolve the owning org and transition an arbitrary workflow. An attacker who captures one valid HelloSign notification can replay it with a substituted envelope_id, forging a terminal state (signed/voided/declined) on any cross-tenant workflow. Bind envelope_id into the authenticated set (full-body HMAC, matching the DocuSign/Adobe paths) to close the gap.

## Evidence
- `backend/servers/api-server/src/routes/integrations/sync.rs:226` — `verify_hellosign_signature` runs `mac.update(format!("{}{}", event_time, event_type))`. Only the two header fields are covered.
- `backend/servers/api-server/src/routes/integrations/webhook.rs:848-861` — the HELLOSIGN branch validates that tuple, then `esignature_webhook` acts on `payload.envelope_id` (L882-884) to resolve org + mark workflow terminal (L906-967).
- DocuSign and Adobe branches verify the full raw `&body` (sync.rs:199/211) — HelloSign is the outlier.
- The idempotency guard in `backend/crates/db/src/repositories/integration.rs:964-987` (`WHERE external_envelope_id = $1 AND status <> ALL($terminal)`) only blocks re-transitioning an already-terminal workflow; a substituted, not-yet-terminal envelope_id passes.
- No `HttpTimeout`/replay window applies — combined with the sibling `code-review-api-handlers-esign-webhook-replay` finding (no freshness on any esign provider), a single captured delivery becomes an unbounded forgery primitive.

## Files
- `backend/servers/api-server/src/routes/integrations/sync.rs:226`
- `backend/servers/api-server/src/routes/integrations/webhook.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Capture (or synthesize) one valid HelloSign webhook JSON body plus its `X-HelloSign-Event-Time` and `X-HelloSign-Event-Type` headers for an existing workflow in org A.
2. Replay the same headers with a modified body where `envelope_id` is replaced by a workflow id owned by org B.
3. Expected: reject (signature covers envelope_id). Actual (today): request accepted; workflow in org B transitions to the signed/voided/declined state carried by the replayed body.

## Suggested approach
1. In `sync.rs:226`, verify the raw request body via HMAC (`verify_slice(&body, ...)`) rather than the two-field composite. Match the DocuSign/Adobe pattern already in the same file (sync.rs:199/211).
2. In `webhook.rs`, thread the raw body bytes through the HELLOSIGN branch — the axum extractor already has them upstream of `esignature_webhook`; if not, take a `Bytes` extractor and re-parse via `serde_json::from_slice`.
3. Refuse to act on any payload whose resolved workflow's `provider` != the verified provider (defense in depth against forged envelope_id targeting a non-HelloSign workflow).
4. Add a `#[sqlx::test]` in `backend/servers/api-server/tests/suites/integrations_esignature_tests.rs` that seeds two orgs each with a workflow, forges a HELLOSIGN payload for org A whose `envelope_id` points at org B, and asserts a 4xx (unauthorized/forbidden) — failing on main today.
5. Update the sibling `code-review-api-handlers-esign-webhook-replay` plan (or fold both fixes into this PR) by adding a ±5-minute `event_time` freshness check while binding envelope_id.

## Alternatives considered
- **Bind envelope_id into the two-field composite (mac over `event_time + event_type + envelope_id`)** — rejected because it hard-codes the body's JSON layout in code that already exists in three provider variants; the "verify raw body" pattern is already the established convention in this file.
- **Only add a freshness window and rely on HelloSign's idempotency** — rejected because the idempotency guard is scoped to already-terminal workflows; a not-yet-terminal envelope_id substitution passes through unless the envelope_id is authenticated.

## Root-cause trace
1. Symptom: cross-tenant HelloSign webhook replay can drive an arbitrary workflow terminal.
2. ← `esignature_webhook` acts on `payload.envelope_id` at `webhook.rs:882-884` before any provider-specific integrity check on the id.
3. ← `verify_hellosign_signature` at `sync.rs:226` computes MAC over `event_time + event_type` only — envelope_id is not in the authenticated set.
4. Origin: initial HelloSign webhook plumbing (see git blame on `sync.rs::verify_hellosign_signature` / `webhook.rs::esignature_webhook`) — the DocuSign/Adobe paths later adopted full-body HMAC but HelloSign kept the narrower composite.

## Test plan
- [ ] `backend/servers/api-server/tests/suites/integrations_esignature_tests.rs` — new `hellosign_envelope_id_replay_rejected` `#[sqlx::test]` failing on main
- [ ] Regression: same test suite covers DocuSign/Adobe paths remain green after the sync.rs change
- [ ] `cd backend && cargo test -p api-server integrations_esignature`

## Out of scope
- Rotating HelloSign shared secrets / key management workflow.
- Broader webhook queue at-most-once storage layer (would be its own vector).
- HelloSign UI surface changes.

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-hellosign-unbound-envelope.md`
- Mark the matching `backlog.json` row as `status: "done"`
