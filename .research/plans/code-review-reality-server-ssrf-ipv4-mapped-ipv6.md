# code-review-reality-server-ssrf-ipv4-mapped-ipv6

**Vector:** security
**Score:** 3
**Source:** code-review 2026-07-31 (reality-server Tier-1d dev-review) · hotspot in `backend/servers/reality-server/src/util/url_validator.rs:145`
**Confidence:** high

## Hypothesis
The `Host::Ipv6` branch of `validate_fetch_url` rejects only three IPv6 forms — `is_loopback()` (`::1`), `fc00::/7`, and `fe80::/10` — and never rejects IPv4-mapped IPv6 addresses (`::ffff:a.b.c.d`) or the deprecated IPv4-compatible form. `std::net::Ipv6Addr::is_loopback` returns `false` for `::ffff:127.0.0.1`, and the fc00/fe80 segment masks do not match a mapped address either, so `https://[::ffff:127.0.0.1]/`, `https://[::ffff:10.0.0.5]/`, `https://[::ffff:169.254.169.254]/`, etc. slip past the guard as "allowed" fetch URLs. Because the guard is the single choke-point for outbound reality-server fetches, this re-opens the SSRF class the `Ipv4` branch above already blocks. Fix by unwrapping every `Ipv6` host: if it has an IPv4 embedding (`ip.to_ipv4_mapped()` or `ip.to_ipv4()`), route the check through the existing IPv4 rules; if any octet-based rule matches, deny with a specific error string.

## Evidence
- `backend/servers/reality-server/src/util/url_validator.rs:145-165` — `Host::Ipv6(ip)` branch guards only `ip.is_loopback()`, `fc00::/7`, `fe80::/10`; nothing addresses `::ffff:*` or IPv4-compat.
- `backend/servers/reality-server/src/util/url_validator.rs:93-143` — `Host::Ipv4(ip)` branch already denies `is_loopback()`, `is_private()`, `is_link_local()`, `is_broadcast()`, and the AWS metadata IP explicitly.
- Rust `std::net::Ipv6Addr` doc: `is_loopback()` returns `false` for `::ffff:127.0.0.1` (the "mapped loopback"); the mapping is intentional so callers must convert.
- Existing test surface `validate_fetch_url` has cases for `[::1]` (line 298) and `10.0.0.5` (line 289) but no `::ffff:*` case — the bypass is not currently covered.
- code-review-2026-07-31 signal `code-review-reality-server-ssrf-ipv4-mapped-ipv6` (score_delta=+3, confidence=high) folded into `.research/backlog.json` (`code-review-reality-server-ssrf-ipv4-mapped-ipv6`, score=3).

## Files
- `backend/servers/reality-server/src/util/url_validator.rs`

## Dependencies
- (none)

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

Mode: cloud-ok

## Repro steps
1. From the reality-server crate, add a test that calls `validate_fetch_url("https://[::ffff:127.0.0.1]/x")` and asserts it returns `Err(UrlValidationError::_)` naming the mapped-loopback rule.
2. On `dev` (pre-fix), the test **passes as Ok** — the guard does not reject the mapped-loopback host. Expected after the fix: the test fails on `dev` and passes only once the mapped-IPv6 branch routes through the IPv4 rules.

## Suggested approach
1. Extend `Host::Ipv6(ip)` at `url_validator.rs:145` — if `ip.to_ipv4_mapped()` is `Some(v4)` (or fall back to `to_ipv4()` for IPv4-compat, guarded to reject the deprecated form entirely), re-enter the same rule-set the `Host::Ipv4(ip)` branch uses (or extract the v4 rule closure to a private fn and call it from both arms).
2. Explicitly deny **all** IPv4-compat addresses (`::0:0:0:0:0:0:a.b.c.d` where the top 96 bits are zero and the low 32 bits are non-zero non-mapped) with a distinct error string — the form is deprecated and has no legitimate outbound use.
3. Keep the existing `is_loopback` / `fc00::/7` / `fe80::/10` checks unchanged (they cover the pure-IPv6 cases).
4. Add tests covering `::ffff:127.0.0.1`, `::ffff:10.0.0.5`, `::ffff:169.254.169.254`, `::ffff:0.0.0.0`, and the corresponding negative cases (`::ffff:8.8.8.8` should still succeed).
5. Run `cargo test -p reality-server url_validator` locally to confirm the new cases fail on `dev` and pass on the branch.
6. Re-run the full `reality-server` unit-test suite once green.
7. Update the module-level doc-comment (`// fc00::/7 ...`) to name the new mapped-IPv6 rule alongside the fc00/fe80 rules.

## Alternatives considered
- **Reject every non-native-IPv6 host outright** — rejected because there are legitimate operational cases (managed Kubernetes egress via mapped addresses) and blocking them would be a behaviour change beyond the security scope; the mapped→IPv4 rule-reuse is minimal and matches the intent of the existing v4 guard.
- **Blocklist only the specific IPs called out** (`::ffff:127.0.0.1`, `::ffff:169.254.169.254`) — rejected because that leaks the private-range coverage (`::ffff:10.0.0.5`, `::ffff:192.168.*`) that the v4 branch already denies; a rule-reuse path stays consistent with `is_private`/`is_link_local`.

## Root-cause trace
1. Symptom: An authenticated (or unauthenticated, depending on caller) request that funnels into `validate_fetch_url` with `https://[::ffff:127.0.0.1]/x` is accepted, letting a downstream `client.get(...)` reach a loopback / RFC1918 target the guard was intended to block.
2. ← Immediate cause at `backend/servers/reality-server/src/util/url_validator.rs:146` — the `is_loopback()` branch on an `Ipv6Addr` returns `false` for `::ffff:127.0.0.1` (the address is a mapped-loopback, not a true `::1`), so the guard falls through to the fc00/fe80 checks.
3. ← Upstream cause at `backend/servers/reality-server/src/util/url_validator.rs:145` — the `Host::Ipv6(ip)` arm treats IPv6 as a distinct address space and never unwraps mapped addresses back to their embedded IPv4 form; the shared IPv4 rules at lines 93-143 are unreachable from this branch.
4. Origin: introduced when the IPv6 branch was added to `validate_fetch_url` (initial reality-server SSRF-hardening commit); the IPv4 branch and the IPv6 branch were written independently, and no test exercised the mapped form.

## Test plan
- [ ] Add `#[test] fn rejects_ipv4_mapped_ipv6_loopback()` in `backend/servers/reality-server/src/util/url_validator.rs` — asserts `validate_fetch_url("https://[::ffff:127.0.0.1]/")` is `Err`.
- [ ] Add `#[test] fn rejects_ipv4_mapped_ipv6_private()` — asserts mapped-RFC1918 (`::ffff:10.0.0.5`) is `Err`.
- [ ] Add `#[test] fn rejects_ipv4_mapped_ipv6_aws_metadata()` — asserts `::ffff:169.254.169.254` is `Err`.
- [ ] Add `#[test] fn allows_ipv4_mapped_ipv6_public()` — asserts `::ffff:8.8.8.8` still returns `Ok`.
- [ ] Run: `cd backend && cargo test -p reality-server url_validator`.
- [ ] Full crate sweep: `cd backend && cargo test -p reality-server`.

## Out of scope
- Auditing other reality-server modules for direct `reqwest::Client` usage that bypasses `validate_fetch_url` (separate finding tracked under `code-review-reality-server-raw-db-error-leak-multi` / other reality-server tier-1d signals).
- Changing the api-server's `backend/crates/common/src/url_validation.rs` (different SSRF guard, already patched by PR #450).
- Adding IPv6 route-filtering at the network layer (out of code scope).

## After-merge
- Move this file to `plans/_archive/code-review-reality-server-ssrf-ipv4-mapped-ipv6.md`
- Mark the matching `backlog.json` row (`code-review-reality-server-ssrf-ipv4-mapped-ipv6`) as `status: "done"`
