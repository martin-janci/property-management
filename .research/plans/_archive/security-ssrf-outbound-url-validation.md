# security-ssrf-outbound-url-validation

**Vector:** security
**Score:** 3
**Source:** Issue #439 | signatures.rs:628 | integrations.rs:2743
**Confidence:** high

## Hypothesis
The api-server makes server-side outbound HTTP requests to URLs that originate from user/provider-controlled data, without validating the target host against private/loopback/link-local ranges. Two sinks are unguarded: `store_signed_document` fetches an e-signature provider's `signed_url`, and the webhook-test handler POSTs to a stored `subscription.url`. An authenticated org user (or a malicious/poisoned e-signature provider) can point these at `http://169.254.169.254/...` (cloud metadata), `http://127.0.0.1`, or internal services — classic SSRF. An SSRF guard (`validate_external_url` + `is_blocked_v4`) already exists but is a private method on `ApiCallExecutor`, applied at only one call site; the fix is to extract it into a shared module and apply it at both unguarded sinks before the request fires.

## Evidence
- Issue #439 (P1-05, severity high) — "P1-05 only half-fixed — SSRF still open": the PR #435 commit hardened MIME/size/magic-bytes on the response body but never validated `signed_url`
- `backend/servers/api-server/src/routes/signatures.rs:628` — `client.get(signed_url)` on a `reqwest::Client::new()` (no redirect policy, no host validation); the comment at lines 613-619 claims SSRF hardening but only added MIME/size/magic-byte checks on the response
- `backend/servers/api-server/src/routes/integrations.rs:2743` — `client.post(&subscription.url)` uses the stored webhook URL directly; redirects are disabled (`Policy::none()`) but the host is never checked against private ranges
- `backend/servers/api-server/src/services/actions/api_call.rs:152` — a working `validate_external_url` guard (with `is_blocked_v4` at :212) already exists but is `fn` (private) inside the `ApiCallExecutor` impl and reused only at :251

## Files
- `backend/servers/api-server/src/services/actions/api_call.rs:152`
- `backend/servers/api-server/src/routes/signatures.rs:628`
- `backend/servers/api-server/src/routes/integrations.rs:2743`

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):**
No C4/C5 ticked — the change is server-side Rust with a unit-testable pure validator.

Mode: cloud-ok

## Repro steps
1. As an authenticated org member, create a webhook subscription with `url = http://169.254.169.254/latest/meta-data/` (POST `/api/v1/integrations/webhooks`).
2. Trigger the test-delivery endpoint for that subscription (`integrations.rs` webhook-test route, sink at line 2743).
3. Expected (after fix): the request is rejected with a 4xx before any outbound call; the server never connects to the metadata endpoint. Actual (today): the server issues the POST to `169.254.169.254`, exposing internal/metadata reachability. The same holds for `signatures.rs:628` when the e-signature provider returns a `signed_document_url` pointing at an internal host.

## Suggested approach
1. Create a new shared module `backend/servers/api-server/src/services/ssrf.rs` (and declare `pub mod ssrf;` in `services/mod.rs`); move `validate_external_url` and `is_blocked_v4` (plus any IPv6 blocking logic) out of `api_call.rs:152`/`:212` into it as `pub fn validate_external_url(url: &str) -> Result<(), String>`.
2. Update `ApiCallExecutor` at `api_call.rs:251` to call `crate::services::ssrf::validate_external_url(&url)` so the existing call site keeps its behavior (no functional change there).
3. In `signatures.rs::store_signed_document`, before the `client.get(signed_url)` at line 628, call `ssrf::validate_external_url(signed_url)` and return `Err(...)` on rejection; also build the client with `reqwest::redirect::Policy::none()` to match the webhook path.
4. In `integrations.rs`, before the `client.post(&subscription.url)` at line 2743, call `ssrf::validate_external_url(&subscription.url)` and return a 4xx error on rejection.
5. Ensure the validator resolves the host and blocks RFC-1918 (`10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`), loopback (`127.0.0.0/8`, `::1`), link-local (`169.254.0.0/16`, `fe80::/10`), and unspecified addresses; keep the public-URL happy path passing.
6. Add unit tests for the extracted validator (see Test plan); run `cargo test -p api-server` and `cargo clippy -p api-server`.

## Alternatives considered
- **Block only via `reqwest` redirect policy** — rejected because redirect policy stops redirect-based SSRF only; a `signed_url`/`subscription.url` that points directly at a private host still resolves and connects. Host validation must happen before the request.
- **Add a per-call-site inline private-IP check** — rejected because duplicating the blocklist at each sink is exactly how the codebase ended up with one guarded and two unguarded call sites; a single shared `validate_external_url` is the only way to keep the three sites consistent.

## Root-cause trace
1. Symptom: server issues an outbound HTTP request to an attacker-influenced internal address (e.g. `169.254.169.254`).
2. ← `signatures.rs:628` `client.get(signed_url)` / `integrations.rs:2743` `client.post(&subscription.url)` fire with no host validation.
3. ← The SSRF guard `validate_external_url` was written as a private method on `ApiCallExecutor` (`api_call.rs:152`) instead of a shared helper, so the P1-06 fix protected only the action-runner call site.
4. Origin: PR #435 (the P0/P1 security pass) — its P1-05 fix hardened the signed-document *response* (MIME/size/magic-bytes) but left the URL unvalidated; issue #439 records the gap.

## Test plan
- [ ] `backend/servers/api-server/src/services/ssrf.rs` unit tests: `validate_external_url` rejects `http://127.0.0.1`, `http://10.0.0.1`, `http://169.254.169.254/`, `http://[::1]/`, and accepts a public URL such as `https://example.com/doc.pdf` — these fail on main (no shared validator exists yet) and pass after extraction.
- [ ] Regression: assert `ApiCallExecutor` behavior is unchanged — the existing `api_call.rs` validation test (or a new one) still rejects a private URL through the executor path.
- [ ] `cargo test -p api-server` and `cargo clippy -p api-server -- -D warnings`.

## Out of scope
- The other findings bundled in issue #439 (P1-01 query ordering, P1-04 `format!("{:?}")` audit-hash domain, P0-12 cookie scope, IG3 test gap) — each is independently actionable and tracked on the issue; this plan covers only the P1-05 SSRF sinks.
- DNS-rebinding defense (re-resolving at connect time) — note it as a follow-up; the host-range blocklist closes the direct-SSRF hole this plan targets.
- Per-provider allow-list for e-signature hosts (DocuSign/HelloSign/Adobe Sign) — a hardening enhancement beyond the private-range block.

## After-merge
- Move this file to `plans/_archive/security-ssrf-outbound-url-validation.md`
- Mark the matching `backlog.json` row as `status: "done"`
