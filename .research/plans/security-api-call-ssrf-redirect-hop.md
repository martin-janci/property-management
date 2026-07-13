# security-api-call-ssrf-redirect-hop

**Vector:** security
**Score:** 3
**Source:** signals/2026-07-10-api-core.json (id `code-review-api-core-apicall-ssrf-redirect`), re-confirmed in signals/2026-07-13-api-core.json
**Confidence:** high

## Hypothesis

`ApiCallExecutor::new()` in `backend/servers/api-server/src/services/actions/api_call.rs:106` builds `reqwest::Client` with only `.timeout()` + `.user_agent()` — no `.redirect(Policy::…)`, so reqwest defaults to following up to 10 hops. `validate_external_url()` runs once at line 251 on the initial URL only. An attacker-controlled external endpoint that responds `302 Location: http://169.254.169.254/latest/meta-data/` (or `http://10.0.0.1/…`) is followed transparently, and the response body is read into the workflow execution row (line 322). Fix: install a custom redirect `Policy` that re-runs `validate_external_url` on every hop and rejects internal targets. Any authenticated workflow author with rights to configure an `api_call` workflow action reaches this sink in production (registered at `services/actions/mod.rs:206`).

## Evidence

- `backend/servers/api-server/src/services/actions/api_call.rs:106-110` — `reqwest::Client::builder().timeout(...).user_agent(...).build()`; no `.redirect(Policy::none())` or custom hop policy.
- `backend/servers/api-server/src/services/actions/api_call.rs:250-253` — `Self::validate_external_url(&url)` runs once on the initial user-templated URL; no per-hop revalidation exists in the module.
- `backend/servers/api-server/src/services/actions/api_call.rs:307` — `request.send().await` transparently follows 3xx hops with the default `Policy::limited(10)`.
- `backend/servers/api-server/src/services/actions/mod.rs:206` — `registry.register(Box::new(ApiCallExecutor::new()))` — reachability confirmed end-to-end from the workflow executor.
- Dispatcher Tier-1d dev-review re-confirmed HIGH on both 2026-07-10 and 2026-07-13 (3 days unfixed); dispatcher buffer is 0/72 so the finding was never converted to an assignment via that path.

## Files

- `backend/servers/api-server/src/services/actions/api_call.rs`
- `backend/servers/api-server/src/services/actions/mod.rs`

## Dependencies

<!-- empty — no other backlog items must land before this one. -->

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

1. Start a small local HTTP server (e.g. `python3 -m http.server 9091`) that responds `302 Location: http://169.254.169.254/latest/meta-data/` for `GET /trap`.
2. As an authenticated user, POST an `api_call` workflow action with `url = "http://127.0.0.1:9091/trap"`. Since `127.0.0.1` is a literal internal IP, `validate_external_url` rejects the initial URL — so instead point it at any allowed external host (e.g. `http://httpbin.org/redirect-to?url=http://169.254.169.254/latest/meta-data/`) and trigger the action from a workflow.
3. Expected (fixed): the executor returns an internal-target error before the metadata endpoint is dialed. Observed (current): reqwest follows the 302 and the metadata-endpoint response body is stored on the workflow execution record.

## Suggested approach

1. In `ApiCallExecutor::new()` (line 106), swap the default builder for one that installs a custom redirect policy: `.redirect(reqwest::redirect::Policy::custom(|attempt| { ... }))`.
2. Inside the closure, call the same `validate_external_url` guard on `attempt.url().as_str()` (make it `pub(super)` or lift it to a module-level helper so the closure can call it). On error, return `attempt.error(err_msg)` — reqwest will surface the failure to the caller.
3. Cap the total number of hops at 3 (well under the 10 default) via `Policy::limited(3)` composed with the custom check — Rust's builder API allows a chained policy via a wrapper closure that runs both.
4. Add a small helper `validate_external_url_public(url: &str)` (or make the existing method `pub(crate)`) so the closure can reuse the exact same allow/block matrix.
5. Add an integration test under `backend/servers/api-server/tests/` that spins a `tokio` echo server returning `302 Location: http://127.0.0.1:NNN/pwn` for `/trap`, points an `ApiCallExecutor` at it via a permitted upstream URL, and asserts the executor rejects with `Blocked host` or `Private/internal IP address not allowed` (matching the existing error text at line 199 / 170).
6. Update the sibling finding `security-api-call-ssrf-dns-rebinding`'s plan not to duplicate the closure — but LAND them in the same PR if the reviewer prefers a single security-fix commit.
7. Cross-reference: `backend/crates/common/src/url_validation.rs` (added in PR #450) may already have a hardened `validate_external_url` — prefer reusing it over the local copy at `api_call.rs:152`.

## Alternatives considered

- **`.redirect(Policy::none())` + manual hop-following in the executor** — rejected because it forces the executor to reimplement redirect state (cookies, method transformation, `Location` resolution) and the redirect surface then diverges from every other reqwest caller in the codebase.
- **Pin the connection to the resolved IP via a custom `dns_resolver`** — rejected as insufficient on its own for the redirect-hop finding (it does not stop 3xx hops from pointing at a NEW hostname); it is the right fix for the sibling DNS-rebinding plan and belongs there.

## Root-cause trace

1. Symptom: workflow execution row on the api-server contains a body fetched from `http://169.254.169.254/latest/meta-data/` after a permitted-external initial URL.
2. ← `backend/servers/api-server/src/services/actions/api_call.rs:307` — `request.send().await` returns after transparently following the 3xx hop.
3. ← `backend/servers/api-server/src/services/actions/api_call.rs:106-110` — the client was built without a redirect policy, so reqwest defaulted to `Policy::limited(10)` with NO per-hop validation.
4. Origin: `ApiCallExecutor::new()` was authored (see git blame on `api_call.rs:106`) with the reqwest defaults never questioned; PR #450 hardened `validate_external_url` but only applied the guard at the initial-URL boundary in `execute()`.

## Test plan

- [ ] New integration test `backend/servers/api-server/tests/api_call_ssrf_redirect_hop_tests.rs` — spins a local `tokio` HTTP server that 302s to `http://127.0.0.1:NNN/…` and asserts the executor errors before dialing the second hop.
- [ ] Regression assertion: the error surfaced to the caller is the same `Blocked host: …` / `Private/internal IP address not allowed: …` text produced by `validate_external_url` (matches lines 170/199), not a raw `reqwest::Error`.
- [ ] Command: `cargo test -p api-server --test api_call_ssrf_redirect_hop_tests` (add `--features test-utils` if the crate gates a mock upstream helper).

## Out of scope

- The DNS-rebinding finding (sibling plan `security-api-call-ssrf-dns-rebinding`) — separate fix landing separately unless the reviewer requests a bundle.
- The unbounded `response.text()` DoS at line 322 — tracked as `bug-api-call-response-body-unbounded` (score 2, not eligible for fast-track promotion this run).
- Refactoring `validate_external_url` out of `api_call.rs` into `backend/crates/common/src/url_validation.rs` — worth doing but not required for this fix. If time permits, prefer reusing the common-crate helper; otherwise leave the duplication note in a follow-up.

## After-merge

- Move this file to `plans/_archive/security-api-call-ssrf-redirect-hop.md`.
- Mark `backlog.json` row `security-api-call-ssrf-redirect-hop` as `status: "done"` with resolving PR reference.
- Trigger the dispatcher Tier-1d re-review of `api-core` to confirm the signal no longer fires (expected zero for `code-review-api-core-apicall-ssrf-redirect`).
