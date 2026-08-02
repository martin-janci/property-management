# code-review-api-core-api-call-ssrf-redirect

**Vector:** security
**Score:** 3
**Source:** rotating-expert-review api-core 2026-08-02
**Confidence:** medium

## Hypothesis

The workflow API-call action (`ApiCallExecutor` in `services/actions/api_call.rs`) builds its `reqwest::Client` without setting a redirect policy, so the client silently follows up to 10 HTTP redirects (`reqwest`'s default `Policy::limited(10)`). The SSRF guard (`validate_external_url` at line 152) only checks the initial URL passed to `execute()`, so an attacker-controlled external URL that returns `302 Location: http://169.254.169.254/latest/meta-data/…` (AWS/GCP metadata) or `http://127.0.0.1:<port>`, `[::1]`, `10.*`, `169.254.*`, or a `.internal` DNS name has the client follow the redirect and hit the internal target — returning the response body back to the workflow. The fix is one line: apply `.redirect(reqwest::redirect::Policy::none())` on the builder (the same pattern the sibling `routes/integrations/webhook.rs:512` already uses).

## Evidence

- `backend/servers/api-server/src/services/actions/api_call.rs:106-113` — `ApiCallExecutor::new()` builds the client with `.timeout(...)`, `.user_agent(...)`, `.build()` — no `.redirect(...)` call.
- `backend/servers/api-server/src/services/actions/api_call.rs:152` — `validate_external_url()` runs pre-request only; there is no redirect callback / post-redirect validation.
- `backend/servers/api-server/src/services/actions/api_call.rs:251` — `execute()` calls `Self::validate_external_url(&url)` once against the caller-supplied URL, then hands the URL to `self.client.request(...)`; the follow-redirects behaviour is inside `reqwest` past that point.
- `backend/servers/api-server/src/routes/integrations/webhook.rs:512` — sibling HTTP-out path uses `.redirect(reqwest::redirect::Policy::none())` on the same builder, confirming this is the team's established pattern for authenticated outbound calls.

## Files

- `backend/servers/api-server/src/services/actions/api_call.rs`

## Dependencies

<!-- No blockers — self-contained one-line client hardening + regression test. -->

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

1. Stand up a small HTTP test server bound to `127.0.0.1:8090` whose only behaviour is `HTTP/1.1 302 Found\r\nLocation: http://169.254.169.254/latest/meta-data/iam/security-credentials/\r\n\r\n` (or, for a repro without the cloud endpoint present, `Location: http://127.0.0.1:<some internal port>`).
2. Store a Workflow API-call action whose `url` field is `http://127.0.0.1:8090/`. Feed it through the action-executor path that constructs `ApiCallExecutor::new()` and calls `execute()`.
3. Expected (after fix): the request fails at the `302` — `reqwest` returns the redirect response body/status directly and the workflow observes a `302` outcome; the client never touches `169.254.169.254`. Actual (today): the client follows the redirect, dials the internal target, and the workflow receives the metadata response body — `validate_external_url` never re-evaluates the redirect location.

## Suggested approach

1. In `ApiCallExecutor::new()` (backend/servers/api-server/src/services/actions/api_call.rs:106) add `.redirect(reqwest::redirect::Policy::none())` on the `reqwest::Client::builder()` chain, before `.build()`. Mirror `backend/servers/api-server/src/routes/integrations/webhook.rs:512` for consistency.
2. In `execute()` (around :251), decide how to handle a `3xx` response now that the client no longer follows. Two options; pick per team preference:
   - **Reject 3xx as failure:** if `resp.status().is_redirection()` return `ActionError::…` with a clear message ("API-call action does not follow redirects; target returned 3xx to `<Location>`"). Preferred — closest to fail-closed semantics.
   - **Return the 3xx verbatim to the caller:** treat it like any other status and let the workflow decide.
3. If option 2a above is chosen, thread a short constant/string through the existing error mapping — no new error variant needed.
4. Add a regression test that spins up a local HTTP server returning `302 Location: http://127.0.0.1:<internal-port>/secret`, invokes `ApiCallExecutor::execute()` with a URL pointing at that server, and asserts the executor does NOT dial the internal port. `mockito` or `wiremock` are already in the workspace test deps — pick the one the crate already uses.
5. `cargo fmt && cargo clippy -p api-server --all-targets -- -D warnings` and `cargo test -p api-server --test <suite>` for the new test.

## Alternatives considered

- **Set `Policy::limited(0)` instead of `Policy::none()`** — rejected because `Policy::none()` is the explicit semantic ("do not follow redirects"), reads cleaner in review, and matches the sibling webhook client verbatim; `limited(0)` is equivalent behaviour but semantically muddier.
- **Add a per-redirect callback (`Policy::custom(...)`) that re-runs `validate_external_url` on every hop** — rejected because it adds branching complexity (URL parsing on every redirect, ambiguous logging behaviour, larger test surface) for a workflow-action path where following redirects has no legitimate use case; the simpler `Policy::none()` covers the threat model.

## Root-cause trace

1. Symptom: workflow API-call action fetches an internal / cloud-metadata URL despite `validate_external_url` marking it as external.
2. ← `backend/servers/api-server/src/services/actions/api_call.rs:251` — `execute()` validates the URL once, then hands it to `self.client.request(...)`.
3. ← `backend/servers/api-server/src/services/actions/api_call.rs:106-113` — `Client::builder()` never calls `.redirect(...)`, so `reqwest` applies the default `Policy::limited(10)` and follows up to 10 hops silently.
4. Origin: the executor was authored without a redirect policy from the start; the SSRF guard was added on the caller-supplied URL only. The class of bug is "input-validated once, then handed to a library that re-fetches from a controlled-by-the-response location" — the team fixed the same shape in `routes/integrations/webhook.rs:512` and did not backport.

## Test plan

- [ ] `backend/servers/api-server/tests/suites/actions_api_call_tests.rs` (or the existing suite closest to `ApiCallExecutor`) — new test `api_call_does_not_follow_redirects_to_internal` that fails on `dev` and passes with the one-line fix.
- [ ] Regression pattern: `mockito`/`wiremock` server returns `302 Location: http://<internal-target>/`; assert `ApiCallExecutor::execute()` either returns a `3xx` outcome to the workflow (option 2b) or errors out (option 2a) but does NOT issue a request to the redirect target.
- [ ] `cd backend && cargo test -p api-server actions_api_call` (or the exact new suite name).

## Out of scope

- Broader SSRF hardening across every `reqwest::Client` in the workspace — this plan touches only the workflow API-call executor. If audit sweeps warrant a wider pass, file a follow-up.
- Extending `validate_external_url` itself (DNS-rebinding TOCTOU, additional CIDR blocks) — out of scope; the redirect-policy fix removes the attack vector this plan addresses. Rebinding is a separate class.
- Any Workflow action other than API-call.

## After-merge

- Move this file to `plans/_archive/code-review-api-core-api-call-ssrf-redirect.md`
- Mark the matching `backlog.json` row as `status: "done"`
