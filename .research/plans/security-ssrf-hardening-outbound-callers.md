## security-ssrf-hardening-outbound-callers

**Vector:** security
**Score:** 3
**Source:** PR #2710
**Confidence:** high

## Hypothesis
PR #2710 closed a DNS-rebinding TOCTOU on `services/actions/api_call.rs` by wiring a `reqwest::dns::Resolve`-based `SsrfGuardResolver` that re-validates every resolved IP at connect time. Three sibling outbound HTTP callers — `routes/signatures.rs`, `routes/integrations/webhook.rs`, and the `integrations` crate's `workflow_executor.rs` — still rely only on the pre-flight `common::url_validation::validate_external_url` guard and are therefore bypassable by the same DNS-rebinding attack (attacker publishes a public A record for the pre-flight check, then swaps it to a private/internal IP before `reqwest` opens the socket). Lifting `SsrfGuardResolver` + `real_ip_resolver` into a shared `common::url_validation::build_ssrf_guarded_client(...)` factory and wiring it into each caller closes the gap with a small, mechanical change and no behavior regression for legitimate outbound calls.

## Evidence
- PR #2710 body Notes section: "Other outbound-fetch callers (workflow_executor.rs, signatures.rs, webhook.rs) currently only run the static validate_external_url; hardening those the same way is out of scope for #2703 and worth a follow-up."
- `backend/servers/api-server/src/services/actions/api_call.rs:16-45,197-222` — `SsrfGuardResolver` + connect-time wiring landed by #2710.
- `backend/servers/api-server/src/routes/signatures.rs:1008` — `validate_external_url` pre-flight only.
- `backend/servers/api-server/src/routes/integrations/webhook.rs:510,528` — `reqwest::Client::builder()` with no `dns_resolver`, `validate_external_url` pre-flight only.
- `backend/crates/integrations/src/workflow_executor.rs:397` — same pattern (`validate_external_url` pre-flight, no connect-time resolver).

## Files
- `backend/servers/api-server/src/services/actions/api_call.rs`
- `backend/servers/api-server/src/routes/signatures.rs`
- `backend/servers/api-server/src/routes/integrations/webhook.rs`
- `backend/crates/integrations/src/workflow_executor.rs`

## Dependencies

## Required capabilities
- [ ] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

Mode: cloud-ok

## Repro steps
1. Start a stateful DNS resolver stub that returns a public IP (`93.184.216.34`) on the first `Resolve` call for `attacker.example` and then swaps to a private IP (`127.0.0.1`) on subsequent calls.
2. Point one of the three sibling callers at `https://attacker.example/…` — e.g. call `signatures.rs::fetch_external_signed_url("https://attacker.example/x")` under a test harness, or fire a webhook via `integrations::webhook.rs::post_webhook_payload(...)` with `subscription.url = "https://attacker.example/hook"`.
3. Expected: outbound connection refused because the connect-time resolver re-validates and rejects the swapped-to private IP.
4. Actual (on `dev`): request completes against `127.0.0.1` — the pre-flight `validate_external_url` saw the earlier public A-record answer and the default `reqwest` resolver used the later private-IP answer.

## Suggested approach
1. In `backend/crates/common/src/url_validation.rs`, add a public `build_ssrf_guarded_client(timeout: Duration, user_agent: &str) -> reqwest::Client` factory that internalises `SsrfGuardResolver` (lift the impl from `api_call.rs` verbatim, keep the same `Resolve` semantics), the redirect-URL validator, and the default timeout.
2. Refactor `services/actions/api_call.rs:218-236` to call the new factory; keep behavior byte-identical (same timeout, same UA, same redirect policy).
3. In `routes/signatures.rs`, replace the direct `reqwest::get`/`Client::builder()` that follows `validate_external_url(signed_url)` with the shared factory. Preserve the existing `validate_external_url` fail-fast.
4. In `routes/integrations/webhook.rs:510-528`, replace the `reqwest::Client::builder()` with the shared factory; keep the existing `validate_external_url` call in `subscription.url` handling.
5. In `crates/integrations/src/workflow_executor.rs:397`, replace the raw `reqwest` client used after `validate_external_url` with the shared factory.
6. Re-export the factory from `common::url_validation` so all callers use the same import path; verify with `cargo check -p api-server -p integrations`.
7. Do NOT touch behavior on the legitimate-outbound path (same A record for pre-flight and connect) — only the DNS-rebinding path changes outcome.

## Alternatives considered
- **Inline the connect-time resolver at each caller** — rejected because it duplicates the ~30 lines of `SsrfGuardResolver` glue across four files, invites drift when the SSRF policy evolves, and defeats the point of `common::url_validation` being the one authority on outbound URL policy.
- **Wrap `reqwest` at HTTP-middleware layer (tower service)** — rejected because `reqwest` does not surface a resolved-IP hook at that layer; the `dns::Resolve` trait is the only clean seam for connect-time re-validation, and it's already the pattern #2710 chose.

## Root-cause trace
1. Symptom: outbound HTTP request from a sibling caller lands on a private IP despite `validate_external_url` accepting only the public hostname resolution.
2. ← `reqwest::Client::builder()...build()` at `routes/integrations/webhook.rs:510` uses `reqwest`'s default `HickoryDns`/`GaiResolver`, which does its own DNS lookup at connect time — separate from the pre-flight one.
3. ← `common::url_validation::validate_external_url(&subscription.url)` at `webhook.rs:528` resolves the hostname once, checks the result, and returns; that resolution is discarded before `reqwest` re-resolves.
4. Origin: The static-only pre-flight pattern predates #2710; every outbound caller (including `api_call.rs` until #2710) shared it. #2710 fixed only `api_call.rs`; siblings still carry the latent TOCTOU.

## Test plan
- [ ] Port the hermetic stateful-resolver rebind test from `api_call.rs::tests::execute_rejects_dns_rebinding_to_private_ip_at_connect` into one integration test per sibling caller. Suggested locations: `backend/crates/integrations/tests/workflow_webhook_ssrf_tests.rs` (extend if present, create otherwise), `backend/servers/api-server/tests/signatures_ssrf_tests.rs`.
- [ ] Regression: `SsrfGuardResolver` still allows a legitimate public → public resolution (no false-positive block on well-behaved hosts).
- [ ] Command: `cd backend && cargo test -p integrations workflow_webhook_ssrf && cargo test -p api-server signatures_ssrf`.

## Out of scope
- Rewriting `validate_external_url` policy (URL scheme, port allow-list, CIDR block set) — the existing set is intentional and out of scope for this plan.
- Adding rate limits or per-caller circuit breakers to the shared client — orthogonal hardening, separate plan.
- Touching outbound HTTP paths that don't call `validate_external_url` first (e.g. S3/MinIO SDK clients — they already run against a fixed-endpoint URL and are not user-controlled).

## After-merge
- Move this file to `plans/_archive/security-ssrf-hardening-outbound-callers.md`
- Mark the matching `backlog.json` row as `status: "done"`
