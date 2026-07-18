# code-review-api-core-ssrf-validator-drift

**Vector:** security
**Score:** 2
**Source:** manual-static-review-2026-07-18-api-core (Phase 1.5)
**Confidence:** high

## Hypothesis
The workflow action executor `ApiCallExecutor` at `backend/servers/api-server/src/services/actions/api_call.rs:100` maintains its own private SSRF validator (`validate_external_url` at line 152, `is_blocked_v4` at line 212) that duplicates and *drifts from* the canonical `common::url_validation::validate_external_url` at `backend/crates/common/src/url_validation.rs:89`. The shared crate's module doc (`url_validation.rs:1-17`) explicitly requires every handler/service that accepts a user-supplied URL to call the shared function. The local copy accepts plain `http://` in production (the shared version blocks it unless `RUST_ENV=development`), omits the IPv4 multicast range (`224.0.0.0/4`), and — because the built `reqwest::Client` uses the default redirect policy — an allow-listed external host can `302` to `169.254.169.254` (or any other blocked private target) and the initial-URL check is bypassed on the redirect leg. The smallest fix is to replace the local validator with a call to `common::url_validation::validate_external_url` and lock `reqwest::Client` to `Policy::none()` so the caller sees the `302` and can (re-)validate the location header.

## Evidence
- `backend/servers/api-server/src/services/actions/api_call.rs:152` — local `validate_external_url` parallel to the canonical shared version at `backend/crates/common/src/url_validation.rs:89`.
- `backend/servers/api-server/src/services/actions/api_call.rs:243` — scheme check `!url.starts_with("http://") && !url.starts_with("https://")` unconditionally accepts `http://`; shared version rejects `http://` unless `RUST_ENV=development` (see `url_validation.rs:95-101`).
- `backend/servers/api-server/src/services/actions/api_call.rs:212` — `is_blocked_v4` covers private / loopback / link-local / broadcast / documentation / unspecified / 169.254 / 0.0.0.0/8 / CGNAT but **misses `is_multicast()` (224.0.0.0/4)**; shared `check_ipv4` blocks it explicitly at `url_validation.rs:198-200`.
- `backend/servers/api-server/src/services/actions/api_call.rs:100` — `reqwest::Client::builder()...build()` uses the default redirect policy (up to 10 auto-follows), so a permitted external host can 302 into a blocked target the initial-URL guard rejected; shared crate doc explicitly warns about DNS-rebinding at `url_validation.rs:15-17`, and follow-the-redirect is the same category of hole.
- `backend/crates/common/src/url_validation.rs:1-17` — canonical crate doc: "Any handler or service that accepts a user-supplied (or provider-supplied) URL and then fetches it server-side **MUST** call [`validate_external_url`] before issuing the request."

## Files
- `backend/servers/api-server/src/services/actions/api_call.rs:152`
- `backend/servers/api-server/src/services/actions/api_call.rs:212`
- `backend/servers/api-server/src/services/actions/api_call.rs:243`
- `backend/servers/api-server/src/services/actions/api_call.rs:100`
- `backend/crates/common/src/url_validation.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (security fix — bug-adjacent)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):** neither C4 nor C5 is ticked → `cloud-ok`.

Mode: cloud-ok

## Repro steps
1. Author a workflow whose API-call action targets `http://169.254.169.254/latest/meta-data/` directly. Under the local validator the scheme check at line 243 passes, but the IP-range guard at line 212 catches `169.254.169.254` because it's in the `blocked_hosts` array (line 159–167) and the 169.254/16 branch (line 219–220). So the direct attack is already blocked — good baseline.
2. Now author a workflow whose API-call action targets `http://attacker.example.com/redirect-to-metadata` where `attacker.example.com` returns `302 Location: http://169.254.169.254/latest/meta-data/`. The initial-URL check at line 243/152 approves `attacker.example.com`, and because `reqwest::Client` at line 100 has no redirect-policy override, the client silently follows the 302 to `169.254.169.254` and returns the metadata payload to the workflow (visible in the action's `ActionResult.output`). **This is the primary SSRF vector today.**
3. Second, author a workflow whose API-call action targets `http://224.0.0.1/`. The scheme check at line 243 passes; `is_blocked_v4` at line 212 does NOT include multicast, so the call proceeds — the IPv4 multicast range is reachable.
4. Third, in production (RUST_ENV != development), submit `http://known-external.example.com/`. The local validator accepts it; the shared validator would reject it as `PlainHttpNotAllowed`. The workflow can now transmit request bodies over unencrypted HTTP, exposing them to on-path attackers.
5. Expected after fix: cases 2, 3, and 4 all return an `ActionError::ConfigurationError` with the shared validator's error message (`URL points to a forbidden network range: 224.0.0.0/4 (multicast)`, `URL scheme 'http' is not allowed`, or — for the redirect case — `URL points to a forbidden network range: 169.254.0.0/16 (link-local / cloud metadata)`).

## Suggested approach
1. In `backend/servers/api-server/src/services/actions/api_call.rs`, delete the local `validate_external_url` at line 152–209 and the helper `is_blocked_v4` at line 212–225.
2. In the same file, replace the call site (`execute()` around line 240–260) with `common::url_validation::validate_external_url(&url).map_err(|e| ActionError::ConfigurationError(e.to_string()))?;`. The scheme-only pre-check at line 243–248 can be dropped — the shared version's `DisallowedScheme` and `PlainHttpNotAllowed` cover it.
3. In `impl ApiCallExecutor::new()` at line 105–113, change `reqwest::Client::builder()` to `.redirect(reqwest::redirect::Policy::none())` so the client stops at each 3xx and returns a `reqwest::Response` with the `Location` header for the caller to inspect. This is the least-invasive shape; the workflow-action semantic today is "one HTTP call in, one response out" — following redirects transparently is not a documented contract.
4. If step 3 is too disruptive for existing workflows that rely on transparent redirects, use `reqwest::redirect::Policy::custom(|attempt| { revalidate the location URL with common::url_validation; if OK follow, else stop })` — but the simpler `Policy::none()` is preferred: it makes the redirect visible in the action output and lets workflow authors decide whether to follow.
5. Add `Cargo.toml` workspace-dep for `common` under the `api-server` crate if it isn't already there (`grep 'common' backend/servers/api-server/Cargo.toml` first; typically it's already listed because api-server uses `TenantContext`).
6. Add unit tests under `backend/servers/api-server/src/services/actions/api_call.rs` (`#[cfg(test)] mod tests`) that mirror the four Repro cases: (a) allow `https://example.com`, (b) reject `http://attacker.example.com` in prod, (c) reject `http://224.0.0.1/`, (d) confirm redirect to `169.254.169.254` produces a stopped-at-redirect result (integration-style, either wiremock or a hand-rolled `hyper::Server` binding to `127.0.0.1:0`).
7. Run `cargo test -p api-server actions::api_call` and `cargo clippy -p api-server -- -D warnings` to verify.

## Alternatives considered
- **Add the missing rules to the local validator instead of deleting it** — rejected because the drift will re-open the next time the shared validator gains a rule (e.g. the DNS-rebinding hardening called out in `url_validation.rs:15-17`). Two SSRF codebases are strictly worse than one; the whole point of `common` is to be the single source of SSRF truth.
- **Delegate SSRF policy to reqwest itself via a preflight resolver** — rejected because reqwest doesn't natively expose a "reject internal targets" hook; a custom redirect `Policy` is the closest built-in and only handles the 3xx side, not the initial URL.

## Root-cause trace
1. Symptom: workflow with an API-call action targeting an allow-listed external host that 302-redirects to `169.254.169.254/latest/meta-data/` exfiltrates cloud-metadata into `ActionResult.output`.
2. ← Immediate cause at `backend/servers/api-server/src/services/actions/api_call.rs:100` — `reqwest::Client::builder()...build()` uses the default `Policy::limited(10)`, silently following the 302 to the blocked target.
3. ← Upstream cause at `backend/servers/api-server/src/services/actions/api_call.rs:152` — the SSRF check is a *pre-request* string validation on the caller-supplied URL only. It never sees the redirect location, so the guard is defeated by any 3xx response.
4. ← Meta-cause at `backend/servers/api-server/src/services/actions/api_call.rs:152` — the local validator exists at all. The canonical `common::url_validation::validate_external_url` was introduced to solve exactly this problem (its doc comment mandates it), but the workflow-action module was not migrated. Drift set in the moment two SSRF codebases coexisted.
5. Origin: the local validator predates the shared crate. `common::url_validation` was extracted from the earlier IDOR/SSRF hardening pass (see the `security-ssrf-outbound-url-validation` archived plan at `.research/plans/_archive/security-ssrf-outbound-url-validation.md`), but this file wasn't updated in that sweep.

## Test plan
- [ ] Unit test `rejects_http_in_prod` — `validate_external_url("http://example.com")` under `RUST_ENV=""` returns `ActionError::ConfigurationError` with `PlainHttpNotAllowed`.
- [ ] Unit test `rejects_multicast_ipv4` — `validate_external_url("http://224.0.0.1/")` returns `ActionError::ConfigurationError` with a multicast reason.
- [ ] Integration test `blocks_redirect_to_metadata` — spawn a local `hyper` server on 127.0.0.1 that returns `302 Location: http://169.254.169.254/`, then call the workflow action against the local server. Expect `ActionError::ExecutionError` (stopped at 302) with the redirect URL surfaced in the error message; assert no HTTP call reaches `169.254.169.254`.
- [ ] Regression test `still_allows_public_https` — `https://example.com/feed` continues to execute successfully (baseline).
- [ ] Run locally: `cd backend && cargo test -p api-server actions::api_call` and `cargo clippy -p api-server -- -D warnings`.

## Out of scope
- DNS-rebinding hardening at fetch time (documented open gap in `url_validation.rs:15-17`) — a separate plan; requires resolver-level revalidation and touches every caller of `validate_external_url`, not just `ApiCallExecutor`.
- Migrating other callers of `common::url_validation::validate_external_url` — this plan is scoped to `ApiCallExecutor` only. If a future review finds another divergent SSRF check, file it as its own plan.
- Renaming / refactoring the workflow action executor abstraction — the fix is a 3-file diff (`api_call.rs`, `Cargo.toml` if needed, test file). No architectural change.

## After-merge
- Move this file to `plans/_archive/code-review-api-core-ssrf-validator-drift.md`
- Mark the matching `backlog.json` row (`code-review-api-core-ssrf-validator-drift`) as `status: "done"`
- Add a `sources` entry with the merged PR number to that row
