# security-ssrf-toctou-integrations-webhook

**Vector:** security
**Score:** 3
**Source:** PR #2710 (author's own follow-up note naming `integrations/webhook.rs`) + Phase 1.5 rotating expert review 2026-08-09 (signal `code-review-api-core-ssrf-toctou-webhook`) + this run's churn-hotspot on `routes/integrations/webhook.rs` (`repeated-churn` proxy)
**Confidence:** medium

## Hypothesis

`routes/integrations/webhook.rs::test_webhook` builds a `reqwest::Client::builder()` with `redirect::Policy::none()` (good) and `pool_max_idle_per_host(0)` (also good — every request re-resolves) but **no `dns_resolver` guard**, and relies solely on `common::url_validation::validate_external_url(&subscription.url)` at `:528` for SSRF protection. That is the exact string-only pattern PR #2710 removed from `api_call.rs`. The endpoint is currently **unmounted** (per the router docstring at `:79-95` — "ROADMAP(PAP-122): outbound webhook-subscription CRUD unmounted"), so the vulnerability is not reachable via HTTP today. But: (a) the vulnerable code stays live in the tree; (b) the moment Epic-61 lands the migrations that mount the CRUD, the SSRF re-emerges silently; (c) the file is a churn hotspot this run (2799 lines), so drift risk is real. Port the same `SsrfGuardResolver` + `resolve_and_validate_host` pattern used in the `signatures.rs` sibling plan before the endpoint mounts, not after.

## Evidence
- `backend/servers/api-server/src/routes/integrations/webhook.rs:510-528` — `test_webhook` builds `reqwest::Client::builder().redirect(redirect::Policy::none()).pool_max_idle_per_host(0).build()` without a `dns_resolver`; then `common::url_validation::validate_external_url(&subscription.url)` at `:528` — string-only, TOCTOU wide open at connect
- `backend/servers/api-server/src/routes/integrations/webhook.rs:79-95` — router docstring: `"ROADMAP(PAP-122): outbound webhook-subscription CRUD unmounted"` — endpoint currently NOT reachable via HTTP
- PR #2710 body Notes section (merged 2026-08-08T06:17:40Z): "Other outbound-fetch callers (`workflow_executor.rs`, `signatures.rs`, `webhook.rs`) currently only run the static `validate_external_url`; hardening those the same way is out of scope for #2703 and worth a follow-up." — author explicitly names this file
- `backend/servers/api-server/src/services/actions/api_call.rs:41` (`SsrfGuardResolver`) — the fix pattern already exists in-tree

## Files
- `backend/servers/api-server/src/routes/integrations/webhook.rs:510`
- `backend/servers/api-server/src/routes/integrations/webhook.rs:528`
- `backend/servers/api-server/src/services/actions/api_call.rs:41`
- `backend/crates/common/src/url_validation.rs`

## Dependencies
- security-ssrf-toctou-signatures-rebinding

## Required capabilities
- [x] C1 — Systematic debugging (security-adjacent bug)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode:** `Mode: cloud-ok`

## Repro steps

1. Because the endpoint is unmounted, drive the vulnerable code directly from a unit test: instantiate the `test_webhook` handler with a `subscription.url = "https://attacker.example.com/hook"` and inject a stateful DNS resolver (via the shared `SsrfGuardResolver` seam introduced by the sibling `signatures-rebinding` plan) that answers a public IP the first time and `10.0.0.5` the second time.
2. **Expected on today's `dev`:** the handler passes the static `validate_external_url` check and `client.post(url).send()` connects to `10.0.0.5`. The `reqwest` connect succeeds against the private IP (or fails only if `10.0.0.5` doesn't accept connections — in which case the assertion is that the connect ATTEMPT was made against a private IP, not the response).
3. **Expected after the fix:** `client.post(...).send().await` returns an SSRF-guard error before the socket opens; the stateful resolver was called ≥ 2 times.

## Suggested approach

1. Depends on `security-ssrf-toctou-signatures-rebinding` landing first (that plan extracts the shared `common::http_client::build_ssrf_guarded_client` helper from `api_call.rs`). If this plan runs before that one lands, fall back to inlining the pattern by copying the resolver + builder chain from `api_call.rs` and calling out in the commit body that a follow-up refactor should collapse both call sites onto the shared helper.
2. In `routes/integrations/webhook.rs:510-517`, replace the current `Client::builder()` chain with `common::http_client::build_ssrf_guarded_client(SsrfClientConfig { pool_max_idle_per_host: 0, redirect_policy: RedirectPolicy::None, ... })`. Preserve the existing timeout and body caps.
3. Keep the pre-flight `validate_external_url` (`:528`) but also call `common::url_validation::resolve_and_validate_host` so a fresh resolution is validated before the request; both layers share the same resolver.
4. Add a hermetic unit test `test_webhook_rejects_dns_rebinding_to_private_ip_at_connect` colocated with the handler (via `#[cfg(test)] mod tests` — the module currently has none). Uses the shared stateful-resolver test helper from the sibling plan (or a local copy if that plan hasn't landed).
5. Add a second hermetic test asserting `redirect::Policy::none()` remains in effect after the helper swap (i.e. an HTTP 302 to `https://attacker.example.com/y` returns a 302 to the caller, not a follow).
6. Verify: `cd backend && cargo test -p api-server -- integrations::webhook` → exit 0; `cargo fmt --check`; `cargo clippy -p api-server --all-targets -- -D warnings`.
7. Do NOT re-mount the endpoint as part of this plan — mounting is Epic-61's responsibility. This plan closes the security gap so that when Epic-61 mounts the CRUD, the fix is already in.

## Alternatives considered
- **Wait until Epic-61 mounts the endpoint, then fix** — rejected because (a) landing security-adjacent code changes on the same PR that mounts a new surface is riskier than landing them separately; (b) this plan makes the sibling `signatures-rebinding` fix's extracted helper get its second consumer, which is what proves the helper's API is right.
- **Delete the unmounted `test_webhook` handler entirely** — rejected because the endpoint is part of Epic-61's planned CRUD surface (documented in the router docstring); deleting it drops planned work rather than hardening it.

## Root-cause trace

1. Symptom: `test_webhook` would open a socket to a private/internal IP after the pre-flight validator returned OK on the same hostname (were the endpoint reachable).
2. ← Immediate cause at `backend/servers/api-server/src/routes/integrations/webhook.rs:517` — `Client::builder()...build()` returns a client with the system default DNS resolver and no post-resolution guard.
3. ← Upstream cause at `backend/servers/api-server/src/routes/integrations/webhook.rs:528` — `validate_external_url` inspects only the URL string.
4. Origin: the anti-SSRF gate was added to this handler alongside the anti-SSRF gate in `services/actions/api_call.rs`, using the same string-only primitive. PR #2710 fixed `api_call.rs` on 2026-08-07 and its Notes section flagged `webhook.rs` as an unfixed sibling — this plan closes that follow-up.

## Test plan
- [ ] `backend/servers/api-server/src/routes/integrations/webhook.rs::tests::test_webhook_rejects_dns_rebinding_to_private_ip_at_connect` — hermetic stateful resolver, IG3 primary regression
- [ ] `backend/servers/api-server/src/routes/integrations/webhook.rs::tests::test_webhook_still_rejects_redirect_after_ssrf_client_swap` — asserts `redirect::Policy::none()` semantics preserved by the helper migration
- [ ] `cd backend && cargo test -p api-server -- integrations::webhook` → exit 0

## Out of scope
- Mounting the outbound webhook-subscription CRUD (Epic-61 owns that surface).
- Hardening `routes/signatures.rs::store_signed_document` (covered by sibling plan `security-ssrf-toctou-signatures-rebinding`).
- Refactoring the wider `integrations/webhook.rs` (churn refactor is a separate `refactor` vector).

## After-merge
- Move this file to `plans/_archive/security-ssrf-toctou-integrations-webhook.md`
- Mark the matching `backlog.json` row as `status: "done"`
