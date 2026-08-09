# security-ssrf-toctou-signatures-rebinding

**Vector:** security
**Score:** 3
**Source:** PR #2710 (author's own follow-up note) + Phase 1.5 rotating expert review 2026-08-09 (segment `api-core`, signal `code-review-api-core-ssrf-toctou-signatures`)
**Confidence:** high

## Hypothesis

The e-signature webhook handler in `routes/signatures.rs::store_signed_document` has the exact same SSRF DNS-rebinding TOCTOU bypass that PR #2710 fixed in `services/actions/api_call.rs`. It validates the `signed_url` with the static string check `common::url_validation::validate_external_url` and then hands the same hostname to a bare `reqwest::Client::new()` — no `SsrfGuardResolver` at connect time, no redirect re-validation. An attacker controlling DNS for the provider-supplied hostname can pass validation with a public IP and connect to `169.254.169.254` (cloud IMDS) or the internal network a millisecond later. The router is mounted at `/api/v1/signature-requests/webhook/{provider}` (`lib.rs:169`), so the path is live in production. The fix is to reuse the in-tree `SsrfGuardResolver` + `redirect::Policy::custom` pattern that already exists in `services/actions/api_call.rs`.

## Evidence
- `backend/servers/api-server/src/routes/signatures.rs:1008` — `common::url_validation::validate_external_url(signed_url)` static check only
- `backend/servers/api-server/src/routes/signatures.rs:1016` — `let client = reqwest::Client::new();` — no `dns_resolver`, no `redirect::Policy` — TOCTOU wide open
- `backend/servers/api-server/src/routes/signatures.rs:882` — `store_signed_document(&state, &signature_request, signed_url).await` in `handle_webhook`
- `backend/servers/api-server/src/lib.rs:169` — `.nest("/api/v1/signature-requests", routes::signatures::router())` (path reachable via HTTP)
- `backend/servers/api-server/src/services/actions/api_call.rs:41` (`struct SsrfGuardResolver`), `:207` (`redirect::Policy::custom`), `:222` (`.dns_resolver(Arc::new(SsrfGuardResolver { ... }))`) — the exact fix pattern already in-tree from PR #2710

## Files
- `backend/servers/api-server/src/routes/signatures.rs:1008`
- `backend/servers/api-server/src/routes/signatures.rs:1016`
- `backend/servers/api-server/src/services/actions/api_call.rs:41`
- `backend/crates/common/src/url_validation.rs`

## Dependencies

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

1. Set up a hermetic stateful DNS resolver (like `api_call::tests::execute_rejects_dns_rebinding_to_private_ip_at_connect`) that answers `attacker.example.com` with a **public IP** on its first resolution (pre-flight validation call site) and with `10.0.0.5` on its second resolution (connect-time call site).
2. Craft an e-signature webhook payload where `signed_document_url = "https://attacker.example.com/x.pdf"` and pass it through the mounted `POST /api/v1/signature-requests/webhook/{provider}` handler under this resolver.
3. **Expected on today's `dev`:** `store_signed_document` passes the static `validate_external_url` check, then `reqwest::Client::new().get(signed_url).send().await` **connects to `10.0.0.5`** and the handler proceeds to buffer the response as a signed PDF. That is the TOCTOU bypass. **Expected after the fix:** connection aborts at connect time with an SSRF-guard error before any byte is read; the resolver is consulted ≥ 2 times (pre-flight + connect).

## Suggested approach

1. Promote the `SsrfGuardResolver` + `build_ssrf_guarded_client` machinery in `services/actions/api_call.rs:41-222` into a reusable helper in `backend/crates/common/src/http_client.rs` (new module) — parameterised by resolver and redirect limit. `api_call.rs` becomes a caller of the shared helper instead of owning its own copy.
2. In `routes/signatures.rs:1016`, replace `reqwest::Client::new()` with `common::http_client::build_ssrf_guarded_client()` (or an inline equivalent while the helper is being extracted). Keep the 60-second timeout and 50 MiB body cap intact.
3. In `routes/signatures.rs:1008`, keep the static pre-flight `validate_external_url` (fail-fast) but ALSO call `common::url_validation::resolve_and_validate_host` (added in PR #2710) so a fresh resolution is validated before the request is built. Both layers share the same resolver.
4. Add IG3 regression test `store_signed_document_rejects_dns_rebinding_to_private_ip_at_connect` (in a new `backend/servers/api-server/tests/suites/signatures_ssrf_tests.rs` wired into a suite). Follows the shape of `api_call::tests::execute_rejects_dns_rebinding_to_private_ip_at_connect`; the resolver is stateful and asserts it was consulted ≥ 2 times.
5. Add a hermetic unit test asserting that a redirect to `http://10.0.0.5/y.pdf` is rejected by the custom redirect policy without following.
6. Verify: `cd backend && cargo test -p api-server --test suite_<N> -- signatures_ssrf` → exit 0. Then `cargo fmt --check` and `cargo clippy -p api-server --all-targets -- -D warnings`.
7. Do NOT touch the other outbound-fetch call sites in this plan (`services/workflow_executor.rs` was audited and does not construct its own `reqwest` client — dispatch is delegated to the already-hardened `ApiCallExecutor`; the `routes/integrations/webhook.rs::test_webhook` is unmounted and covered by a separate plan).

## Alternatives considered
- **Deny hostnames with no `.` (private-suffix heuristic)** — rejected because the failure mode is DNS rebinding, not a static hostname pattern; the attacker owns a real domain. A hostname check that passes at validation time is exactly what the exploit relies on.
- **Skip the shared helper, inline the SsrfGuardResolver in signatures.rs** — rejected because we already have three outbound-fetch call sites that need this (api_call.rs done, signatures.rs here, integrations/webhook.rs coming) and copy-pasting a security primitive drifts. The shared helper is the right factoring.

## Root-cause trace

1. Symptom: `store_signed_document` opens a socket to a private IP (`169.254.169.254` or `10.0.0.0/8`) after the pre-flight validator returned OK on the same hostname.
2. ← Immediate cause at `backend/servers/api-server/src/routes/signatures.rs:1016` — `reqwest::Client::new()` uses the system default DNS resolver with no post-resolution validation.
3. ← Upstream cause at `backend/servers/api-server/src/routes/signatures.rs:1008` — `validate_external_url` inspects only the URL string, not the eventually-resolved socket address.
4. Origin: the SSRF gate was added as a P1-05 follow-up (see the comment block at `:1000-1006`) but the author used the string-only validator; the DNS-rebinding class was not called out at introduction time. PR #2710 (2026-08-07) hardened `api_call.rs` and its author explicitly named `signatures.rs`-family call sites as an unfixed follow-up in the Notes section of the PR body.

## Test plan
- [ ] `backend/servers/api-server/tests/suites/signatures_ssrf_tests.rs::store_signed_document_rejects_dns_rebinding_to_private_ip_at_connect` — hermetic stateful resolver, IG3 primary regression (fails on `dev`, passes after)
- [ ] `backend/servers/api-server/tests/suites/signatures_ssrf_tests.rs::store_signed_document_rejects_private_ip_on_redirect` — asserts custom redirect policy rejects an internal-IP redirect target
- [ ] `cd backend && cargo test -p api-server --test suite_<N> -- signatures_ssrf` → exit 0

## Out of scope
- Hardening `routes/integrations/webhook.rs::test_webhook` (covered by sibling plan `security-ssrf-toctou-integrations-webhook`; that endpoint is currently unmounted per its router docstring, lower urgency).
- Auditing all other outbound-fetch sites in the codebase for the same class (a broader "SSRF audit" plan; this one is scoped to the live path only).
- Refactoring `store_signed_document` beyond the SSRF gate (its 50 MiB body cap and PDF magic-byte check are correct and out of scope).

## After-merge
- Move this file to `plans/_archive/security-ssrf-toctou-signatures-rebinding.md`
- Mark the matching `backlog.json` row as `status: "done"`
