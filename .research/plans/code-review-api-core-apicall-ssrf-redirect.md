# code-review-api-core-apicall-ssrf-redirect

**Vector:** security
**Score:** 3
**Source:** rotating-expert-review api-core 2026-07-10 (originating) + 2026-07-13 (reconfirmation)
**Confidence:** high

## Hypothesis

The `ApiCallExecutor` in `api_call.rs` validates the caller-supplied URL against internal-IP rules once, then issues the outbound request through a `reqwest::Client` built without a redirect policy. `reqwest` defaults to following up to 10 redirects, so a permitted external endpoint that responds `30x` to an internal target (169.254.169.254 metadata, 10.x/127.x hosts) is followed with no re-validation. The response body is parsed and returned to the workflow context, enabling SSRF exfiltration. Smallest fix: build the client with `.redirect(Policy::none())` and either reject 3xx responses or re-run `validate_external_url` on the `Location` header before manually issuing the next hop.

## Evidence

- `backend/servers/api-server/src/services/actions/api_call.rs:106-110` — client builder chains only `.timeout(...)` and `.user_agent(...)`; no `.redirect(...)` clause
- `backend/servers/api-server/src/services/actions/api_call.rs:251` — `validate_external_url()` runs once on the pre-request URL only
- `backend/servers/api-server/src/services/actions/api_call.rs:307` — `request.send().await` transparently follows redirects with no post-hop validation
- `backend/servers/api-server/src/services/actions/mod.rs` — registers `ApiCallExecutor` in the default `ActionRegistry`
- `backend/servers/api-server/src/services/workflow_executor.rs:275` — spawns `WorkflowExecutorTask.run` against a tenant/admin-configured `api_call` workflow action URL (production reachability confirmed)

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
- [x] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):**
Mode: cloud-ok

## Repro steps

1. Stand up an integration test with a tokio local server that on `GET /trap` returns `HTTP 302 Location: http://127.0.0.1:9/`. Configure `ApiCallExecutor` with a `permitted_hosts` list that allows the local server host (or bypass validation the same way the current single-shot check does).
2. Invoke `ApiCallExecutor.execute` against `/trap`. Expected: request is refused after the redirect (`SsrfBlocked` or a re-validation error surfacing the internal target). Actual today: request follows the 302 to `127.0.0.1:9`, either returns a body or a connection error — either way, `validate_external_url` never sees the internal URL.
3. Repeat with `Location: http://169.254.169.254/latest/meta-data/`. Expected: rejected. Actual: request is issued (may hang until timeout in local test — the failure mode is the *attempt*, not the response).

## Suggested approach

1. In `api_call.rs:106-110`, add `.redirect(reqwest::redirect::Policy::none())` to the `reqwest::Client::builder()` chain. This turns 3xx into observable responses instead of transparent hops.
2. After `request.send().await` (line ~307), branch on `response.status().is_redirection()`. On redirect, extract the `Location` header, resolve it against the request URL (relative → absolute), re-run `validate_external_url` on the resolved target, and only then issue the next hop (bounded loop, cap at 3 hops).
3. If the hop budget is exhausted or any hop fails validation, return the existing `SsrfBlocked` variant (or introduce a `RedirectBlocked` variant if the callers distinguish).
4. Cover with unit tests using `wiremock` (already a dev-dep in this workspace, cf. reality-server SSO tests): one test asserting a 302→internal is blocked, one asserting a 302→external allowed target is followed and the body returned, one asserting the redirect cap trips at 4 hops.
5. Sanity-check the two companion findings referenced in the 07-10 pass — `apicall-ssrf-dns-bypass` and `apicall-unbounded-body` — are separate backlog items and are **out of scope** for this plan (see *Out of scope*), so this PR stays focused.

## Alternatives considered

- **Just deny all redirects (Policy::none, no re-validation loop)** — rejected because legitimate external APIs often redirect (e.g. GitHub API `/user` → `/users/:name`, marketing/CDN endpoints). Blanket denial would break real workflows and push workflow authors to work around it, undermining trust in the guardrail.
- **Resolve host to IP up front and pin the connection to the resolved IP (dns-pinning)** — rejected as the fix for this ticket because it belongs to the DNS-rebind finding (`apicall-ssrf-dns-bypass`), which is tracked separately. Applying both fixes in one PR would tangle two independently-verifiable behaviours and complicate review; the redirect fix should land first because it is the simpler, more contained change.

## Root-cause trace

1. Symptom: workflow-authored `api_call` action with an external-facing URL is used to fetch internal 169.254/10.x/127.x targets via 3xx redirect chain
2. ← `reqwest::Client::send` follows redirects transparently (default `Policy::limited(10)`) — `api_call.rs:307`
3. ← client builder in `api_call.rs:106-110` does not set `.redirect(Policy::none())`
4. ← `validate_external_url` is called only on the pre-request URL at `api_call.rs:251`, never on the followed hops
5. Origin: workflow-action executor introduced without a redirect policy (initial commit of `services/actions/api_call.rs`); first reported by rotating-expert-review api-core on 2026-07-10 and reconfirmed 2026-07-13

## Test plan

- [ ] `backend/servers/api-server/src/services/actions/api_call.rs` — inline `#[cfg(test)]` module: `redirect_to_internal_is_blocked` (302 → 169.254.169.254 → error)
- [ ] `backend/servers/api-server/src/services/actions/api_call.rs` — `redirect_to_external_is_allowed` (302 → whitelisted external → body returned)
- [ ] `backend/servers/api-server/src/services/actions/api_call.rs` — `redirect_hop_cap` (chain of 4 external redirects → cap-exceeded error)
- [ ] Command: `cargo test -p api-server services::actions::api_call -- --nocapture`

## Out of scope

- DNS-rebind SSRF (`apicall-ssrf-dns-bypass`) — separate backlog item, separate PR
- Unbounded response body (`apicall-unbounded-body`) — separate backlog item, separate PR
- Redesign of the `permitted_hosts` allowlist mechanism (currently a per-tenant static list) — this plan preserves the current shape

## After-merge

- Move this file to `plans/_archive/code-review-api-core-apicall-ssrf-redirect.md`
- Mark the matching `backlog.json` row as `status: "done"`
- Re-check the two companion findings on the next api-core review pass; open follow-up plans if still present
