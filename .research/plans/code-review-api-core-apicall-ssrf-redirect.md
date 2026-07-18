# code-review-api-core-apicall-ssrf-redirect

**Vector:** security
**Score:** 3
**Source:** rotating-expert-review (tier1d 2026-07-18); prior emits 2026-07-10 and 2026-07-13
**Confidence:** high

## Hypothesis
The workflow `api_call` action builds its `reqwest::Client` with the default redirect policy (up to 10 hops), and `validate_external_url()` only vets the initial URL — never re-runs on redirect targets. An attacker who controls (or influences) an outbound URL a tenant configures can point at a public host that 302s to `http://169.254.169.254/…` (cloud metadata) or an internal host, bypassing every private-IP guard. Two safe fixes exist; the smallest is disabling redirect follow on that client and returning the 3xx response to the caller.

## Evidence
- `backend/servers/api-server/src/services/actions/api_call.rs:106` — `reqwest::Client::builder().timeout(60s).user_agent(...).build()` with no `.redirect(...)`; reqwest's default is `Policy::limited(10)`.
- `backend/servers/api-server/src/services/actions/api_call.rs:151-159` — `validate_external_url` parses only the passed-in URL string; nothing revalidates redirect targets.
- `backend/servers/api-server/src/services/actions/api_call.rs:251` — call site validates URL once before `client.request(method, url).send()`.
- Signal re-emitted 3× (2026-07-10, 2026-07-13, 2026-07-18) without landing in the backlog — this run is the first ingest.

## Files
- `backend/servers/api-server/src/services/actions/api_call.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Stand up a small HTTP tarpit that 302-redirects `GET /probe` → `http://169.254.169.254/latest/meta-data/`.
2. Configure a workflow `api_call` action pointing at `http://tarpit.example.com/probe`.
3. Trigger the workflow.
4. Expected: the request is refused (`validate_external_url` catches the redirect target) or completes as a 3xx without following. Actual: reqwest follows the 302, hits the metadata endpoint, and returns its body up the call stack.

## Suggested approach
1. In `api_call.rs:106`, add `.redirect(reqwest::redirect::Policy::none())` to the `Client::builder()` chain. Simplest fix — any 3xx now surfaces to the caller as-is instead of being followed transparently.
2. Update the response shape: if the response status is 3xx, log the `Location` header and either surface it to the workflow (as data) or fail the action explicitly — the caller must decide how to handle a redirect, not the client.
3. If the product requires following redirects, introduce a custom `redirect::Policy::custom(|attempt| { validate_external_url(attempt.url()).map_err(...) ; ... })` closure that revalidates every hop through the same private-IP / hostname guard.
4. Add an SSRF regression test in `backend/servers/api-server/tests/` that spins up an in-process 302-redirect server pointing at `127.0.0.1:1/…` and asserts the action fails (no follow) or that the metadata IP is rejected on validation.
5. Run `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p api-server` before opening the PR.

## Alternatives considered
- **Post-hoc: validate `response.url()` after the request completes** — rejected because the request has already been sent; the metadata service is contacted before the check. SSRF is about the pre-flight, not the post-mortem.
- **Block all redirects at the reqwest global level** — rejected because other subsystems (integrations/booking, favorites, etc.) legitimately follow redirects on trusted endpoints; the fix must be scoped to the workflow `api_call` client.

## Root-cause trace
1. Symptom: workflow `api_call` action can reach internal / cloud-metadata addresses despite `validate_external_url` allow-list.
2. ← `reqwest` client at `backend/servers/api-server/src/services/actions/api_call.rs:106` inherits default `Policy::limited(10)`.
3. ← `validate_external_url` at `api_call.rs:151-159` runs only on the caller-provided URL string, not on redirect hops (no per-hop callback wired).
4. Origin: the `api_call` action module was introduced with the workflow engine (multiple commits pre-2026-06). No redirect policy has been set since.

## Test plan
- [ ] `backend/servers/api-server/tests/api_call_ssrf.rs` — new integration test: in-process HTTP server returns `302 Location: http://169.254.169.254/latest/meta-data/`; the action-invocation call must not follow it.
- [ ] Add a test that a plain 302 → allowed public URL is either surfaced verbatim or rejected — whichever the fix elects — to lock in the chosen behavior.
- [ ] Local command: `cd backend && cargo test -p api-server api_call_ssrf`.

## Out of scope
- Rewriting `validate_external_url`'s allow-list — that gate stays intact; this plan only closes the redirect-hop bypass.
- Auditing other reqwest clients in the tree — tracked separately by `code-review-api-core-api-call-ssrf-hostname` (backlog).

## After-merge
- Move this file to `plans/_archive/code-review-api-core-apicall-ssrf-redirect.md`
- Mark backlog row `code-review-api-core-apicall-ssrf-redirect` as `status: "done"`
