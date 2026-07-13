# security-api-call-ssrf-dns-rebinding

**Vector:** security
**Score:** 3
**Source:** signals/2026-07-10-api-core.json (id `code-review-api-core-apicall-ssrf-dns-bypass`), re-confirmed in signals/2026-07-13-api-core.json
**Confidence:** high

## Hypothesis

`validate_external_url` in `backend/servers/api-server/src/services/actions/api_call.rs:150-207` only rejects internal targets when `host.parse::<std::net::IpAddr>()` succeeds — i.e., only for literal-IP inputs (line 179). Any DNS hostname skips the `is_blocked_v4/v6` branch entirely; validation returns `Ok(())` even if the hostname resolves to `169.254.169.254`, `10.0.0.1`, `127.0.0.1`, or `fd00::/8`. Reqwest then dials the resolved internal IP — classic DNS-based SSRF, and open to DNS-rebinding (TOCTOU: guard vs. actual dial). Fix: install a `reqwest::dns::Resolve` on the client that resolves the hostname up-front, validates every resolved `IpAddr` against `is_blocked_v4/v6`, and pins the connection to the first-validated IP so a rebinding attacker cannot swap addresses between validation and dial.

## Evidence

- `backend/servers/api-server/src/services/actions/api_call.rs:179` — `if let Ok(ip) = host.parse::<std::net::IpAddr>()` — the `is_blocked_v4/v6` guard body only runs on literal-IP hosts. DNS names fall straight through to `Ok(())` at line 208.
- `backend/servers/api-server/src/services/actions/api_call.rs:150-207` — no DNS resolution step, no post-resolve IP validation, no custom `dns_resolver` on the reqwest client.
- Concrete bypass: an attacker registers `attacker.example.com` with `A 169.254.169.254`. `validate_external_url("http://attacker.example.com/…")` returns `Ok(())` because the host is not a literal IP; reqwest then resolves the hostname and dials `169.254.169.254` — the cloud-metadata endpoint that `blocked_hosts` at line 166 was explicitly protecting.
- Same reachability chain as the redirect-hop finding: `services/actions/mod.rs:206` registers `ApiCallExecutor`; authenticated workflow authors reach the sink through configured `api_call` workflow actions.
- Dispatcher Tier-1d dev-review re-confirmed HIGH on 2026-07-10 and 2026-07-13 (3 days unfixed); dispatcher buffer is 0/72 (`buffer-low` fire) so the finding was never converted to an assignment via that path.

## Files

- `backend/servers/api-server/src/services/actions/api_call.rs`
- `backend/crates/common/src/url_validation.rs`

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

1. Add a temporary integration-test `/etc/hosts`-equivalent by running a mock resolver: a small `tokio` DNS shim that answers `attacker.local A 127.0.0.1` (or use a `hickory-resolver` `Resolver` configured with a static override).
2. Call `ApiCallExecutor::execute()` with `url = "http://attacker.local/"`.
3. Expected (fixed): the executor returns `Private/internal IP address not allowed: 127.0.0.1` (matching the current error text at line 199) — resolved BEFORE the dial.
4. Observed (current): validation returns `Ok(())` (host is not a literal IP), reqwest resolves and dials `127.0.0.1`, and the workflow execution row records the response — SSRF confirmed.

## Suggested approach

1. Split `validate_external_url` (`api_call.rs:152`) into two halves: `validate_scheme_and_syntax(url)` (unchanged) and `validate_resolved_ip(ip: IpAddr)` (the existing `is_blocked_v4/v6` matrix, callable on any resolved IP).
2. Introduce a `PinnedDnsResolver` that wraps `hickory-resolver` (or reqwest's built-in trust-dns feature): on `resolve(name)`, resolve to a `Vec<IpAddr>`, call `validate_resolved_ip` on each, filter out disallowed ones, and if the remaining set is empty return an error; if non-empty return only the first allowed IP as the resolution result.
3. Build the `reqwest::Client` in `ApiCallExecutor::new()` (line 106) with `.dns_resolver(Arc::new(PinnedDnsResolver::new()))`. This closes the TOCTOU window: reqwest dials only the pre-validated IP, so a DNS-rebinding attacker cannot swap addresses between the guard and the dial.
4. Keep the `host.parse::<IpAddr>()` guard at line 179 in place — it remains the fast path for literal-IP inputs. The DNS resolver becomes the second line of defense for hostname inputs.
5. Cross-reference: `backend/crates/common/src/url_validation.rs` may already carry a hardened `validate_external_url` from PR #450 — reuse it if the logic is equivalent; otherwise lift `is_blocked_v4/v6` to the common crate so `api_call.rs` and any future SSRF sinks import one canonical guard.
6. Add an integration test that installs a static-override `PinnedDnsResolver` mapping `test-rebind.local → 127.0.0.1`, calls `ApiCallExecutor::execute("http://test-rebind.local/")`, and asserts an internal-IP error before the socket dial.
7. Follow-up (not blocking): document in `api_call.rs` module docs that any new sink using an `HTTP` client must go through the same `PinnedDnsResolver` — this is the pattern the codebase should standardize on.

## Alternatives considered

- **Blocklist all DNS names (allow only literal-IP inputs)** — rejected because it breaks every legitimate integration; users configure `https://hooks.slack.com/…` and similar hostnames every day.
- **Resolve the host and validate IPs, but let reqwest re-resolve on its own** — rejected because it leaves the TOCTOU / rebinding window open: reqwest's next resolution can return a DIFFERENT IP than the one you validated. Pinning the resolved IP into the resolver output is the only race-free fix.

## Root-cause trace

1. Symptom: workflow execution row contains a response fetched from an internal IP after the user supplied an external hostname.
2. ← `backend/servers/api-server/src/services/actions/api_call.rs:307` — `request.send().await` dials the IP returned by reqwest's default (system) resolver.
3. ← `backend/servers/api-server/src/services/actions/api_call.rs:179` — the guard at `if let Ok(ip) = host.parse::<IpAddr>()` skipped its body because the host was a DNS name; validation returned `Ok(())` at line 208 without ever consulting the resolved IPs.
4. Origin: `validate_external_url` was originally designed for literal-IP inputs; the DNS-resolution path was never covered in review. The subsequent hardening in PR #450 (`backend/crates/common/src/url_validation.rs`) extended the IP-literal matrix but did not add the DNS-resolve step.

## Test plan

- [ ] New integration test `backend/servers/api-server/tests/api_call_ssrf_dns_rebinding_tests.rs` — installs a `PinnedDnsResolver` variant that maps `test-rebind.local` to `127.0.0.1`; calls `ApiCallExecutor::execute("http://test-rebind.local/")`; asserts the error is `Private/internal IP address not allowed: 127.0.0.1` and no TCP dial to `127.0.0.1:NNN` occurred (verify via a listener that fails the test if it accepts a connection).
- [ ] Regression test: `test-rebind.local → 8.8.8.8` (allowed public IP) should still succeed and reach the mock upstream — proves the resolver does not over-block.
- [ ] Command: `cargo test -p api-server --test api_call_ssrf_dns_rebinding_tests`.

## Out of scope

- The redirect-hop finding (`security-api-call-ssrf-redirect-hop`) — separate plan, may land in the same PR at reviewer's discretion.
- The unbounded `response.text()` DoS at line 322 — tracked as `bug-api-call-response-body-unbounded` (score 2, not eligible for fast-track promotion this run).
- Lifting `validate_external_url` fully into `backend/crates/common/src/url_validation.rs` — recommended as a follow-up refactor, not blocking this security fix.

## After-merge

- Move this file to `plans/_archive/security-api-call-ssrf-dns-rebinding.md`.
- Mark `backlog.json` row `security-api-call-ssrf-dns-rebinding` as `status: "done"` with resolving PR reference.
- Trigger the dispatcher Tier-1d re-review of `api-core` to confirm the signal no longer fires (expected zero for `code-review-api-core-apicall-ssrf-dns-bypass`).
