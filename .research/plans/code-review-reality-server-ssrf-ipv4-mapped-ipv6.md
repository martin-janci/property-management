# code-review-reality-server-ssrf-ipv4-mapped-ipv6

**Vector:** security
**Score:** 3
**Source:** signal `code-review-reality-server-ssrf-ipv4-mapped-ipv6` (dispatcher tier-1d review 2026-07-31) | reality-server/src/util/url_validator.rs:145-164
**Confidence:** high

## Hypothesis
`validate_fetch_url` in `reality-server` protects the outbound-fetch sinks (`agency_imports.rs:238`, `agency_imports.rs:290`) against SSRF by rejecting private/loopback/link-local IPv4 and a narrow set of IPv6 ranges — but the IPv6 branch only checks `is_loopback()` (matches `::1` alone), `fc00::/7`, and `fe80::/10`. It does NOT reject IPv4-mapped IPv6 addresses (`::ffff:a.b.c.d`), so an attacker can bypass every IPv4 blocklist entry (loopback, RFC1918, link-local AWS metadata, broadcast, multicast) by mapping the target into IPv6 form — e.g. `https://[::ffff:169.254.169.254]/latest/meta-data/` still passes the validator. The fix is to unmap IPv4-in-IPv6 addresses at the top of the `Host::Ipv6` arm and re-run the existing `Host::Ipv4` checks against the recovered octets.

## Evidence
- `backend/servers/reality-server/src/util/url_validator.rs:145-164` — the `Host::Ipv6` arm rejects only `is_loopback()` / `fc00::/7` / `fe80::/10`; nothing catches IPv4-mapped or IPv4-compatible addresses. `std::net::Ipv6Addr::is_loopback` is true only for `::1`, so `::ffff:127.0.0.1` returns `false` and slips through.
- `backend/servers/reality-server/src/util/url_validator.rs:73-173` — `validate_fetch_url` is the sole SSRF guard on the outbound-fetch path; there is no post-DNS re-check downstream.
- `backend/servers/reality-server/src/routes/agency_imports.rs:238,290` — both fetch sinks call `validate_fetch_url(url)` and short-circuit on error, so the validator's blindness is the whole gap.
- `backend/servers/reality-server/src/util/url_validator.rs:210-302` — existing tests cover `https://127.0.0.1/`, `https://10.0.0.5/`, `https://[::1]/`, and `http://169.254.169.254/…` (bare IPv4), but no test exercises `::ffff:127.0.0.1` / `::ffff:169.254.169.254` / `::ffff:10.0.0.5`.
- `std::net::Ipv6Addr::to_ipv4_mapped` (stable since 1.63) returns `Some(Ipv4Addr)` for `::ffff:0:0/96` and `None` otherwise — the correct API for recovering the mapped IPv4 before applying the v4 rule set.

## Files
- `backend/servers/reality-server/src/util/url_validator.rs`
- `backend/servers/reality-server/src/routes/agency_imports.rs`

## Dependencies
<!-- no upstream tasks; the validator is self-contained -->

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):**
No C4/C5 ticked — the change is a pure Rust host-validation guard with unit tests, no browser or device required.

Mode: cloud-ok

## Repro steps
1. Add a failing test to `backend/servers/reality-server/src/util/url_validator.rs` under `mod tests`:
   ```rust
   #[test]
   fn rejects_ipv4_mapped_loopback() {
       let _g = EnvGuard::set("production");
       assert!(matches!(
           validate_fetch_url("https://[::ffff:127.0.0.1]/"),
           Err(UrlValidationError::PrivateOrReservedIp(_))
       ));
   }

   #[test]
   fn rejects_ipv4_mapped_aws_metadata() {
       let _g = EnvGuard::set("production");
       assert!(matches!(
           validate_fetch_url("https://[::ffff:169.254.169.254]/latest/meta-data/"),
           Err(UrlValidationError::PrivateOrReservedIp(_))
       ));
   }
   ```
2. Run `cargo test -p reality-server url_validator`. Expected on `main`: both tests **fail** — `validate_fetch_url` returns `Ok(_)`, proving the SSRF bypass.
3. After the fix: both tests pass and no existing test in `url_validator.rs` regresses.

## Suggested approach
1. In `validate_fetch_url` at `backend/servers/reality-server/src/util/url_validator.rs:145`, at the top of the `Host::Ipv6(ip)` arm, call `ip.to_ipv4_mapped()` (stable since Rust 1.63).
2. If `Some(v4)` is returned, apply the existing IPv4 rejection ladder (loopback / `10.0.0.0/8` / `172.16.0.0/12` / `192.168.0.0/16` / `169.254.0.0/16` / `100.64.0.0/10` / `0.0.0.0/8` / `255.255.255.255` / multicast) against `v4.octets()` and return the same `PrivateOrReservedIp` variants — one clean way is to extract the existing v4 body into `fn check_ipv4_private(octets: [u8;4]) -> Result<(), UrlValidationError>` and call it from both `Host::Ipv4` and the unmapped branch, so the two paths can never drift.
3. Keep the existing v6-native checks (`is_loopback` / `fc00::/7` / `fe80::/10`) after the unmap branch — an unmapped v4 short-circuits early.
4. Add three regression tests covering `::ffff:127.0.0.1`, `::ffff:169.254.169.254`, and `::ffff:10.0.0.5`; also add one positive test for a legitimate public IPv6 (`2606:4700:4700::1111`) to prove non-mapped v6 still passes.
5. `cargo fmt --all && cargo clippy -p reality-server --all-targets -- -D warnings && cargo test -p reality-server url_validator`.

## Alternatives considered
- **Also reject IPv4-compatible addresses (`::0:0:a.b.c.d`, `0:0:0:0:0:0:a.b.c.d`)** — rejected as a separate change because `Ipv6Addr::to_ipv4()` (deprecated) conflates the two families and `to_ipv4_mapped()` is the single well-defined API. IPv4-compat is obsolete (RFC 4291) and produces the all-zeros prefix, which already fails the `0.0.0.0/8` v4 check *if* we route through the same helper — keeping this plan tight on `::ffff:` and letting the helper cover both is the simpler contract.
- **Post-DNS re-resolution guard at fetch time** — rejected because the validator is deliberately pre-DNS (comment at `url_validator.rs:165-169` explains the tradeoff); adding a post-resolve check is a separate, larger change with retry/redirect semantics. The IPv4-mapped bypass is a validator bug, and the fix must live in the validator.

## Root-cause trace
1. Symptom: `validate_fetch_url("https://[::ffff:127.0.0.1]/")` returns `Ok(_)` in production, allowing the caller (`agency_imports.rs:238` and `:290`) to fetch against loopback and cloud-metadata targets.
2. ← Immediate cause at `backend/servers/reality-server/src/util/url_validator.rs:145-164`: the `Host::Ipv6` arm has no IPv4-mapped branch; `Ipv6Addr::is_loopback` matches `::1` only.
3. ← Upstream cause: the validator was written v4-first (see the exhaustive v4 ladder at :91-143) and the v6 arm was added as a smaller list without cross-referencing that `Host::Ipv6` for `::ffff:127.0.0.1` short-circuits before any v4 check runs — the two ladders were never unified.
4. Origin: commit that introduced `validate_fetch_url` with the split v4/v6 arms — trace via `git log -- backend/servers/reality-server/src/util/url_validator.rs`. The narrower fc00/fe80 v6 list has been there since the module's initial landing.

## Test plan
- [ ] `cargo test -p reality-server url_validator::tests::rejects_ipv4_mapped_loopback`
- [ ] `cargo test -p reality-server url_validator::tests::rejects_ipv4_mapped_aws_metadata`
- [ ] `cargo test -p reality-server url_validator::tests::rejects_ipv4_mapped_private_10`
- [ ] `cargo test -p reality-server url_validator::tests::accepts_public_ipv6` (positive control)
- [ ] `cargo test -p reality-server` (full crate — no regressions in the existing 9 tests)

## Out of scope
- Post-DNS re-resolution guard at fetch time (separate concern; validator is pre-DNS by design).
- IPv4-compatible address family (`::a.b.c.d`) unless it falls out naturally from a shared `check_ipv4_private` helper — obsolete per RFC 4291.
- Any change to `agency_imports.rs` beyond a possible one-line comment noting the tightened validator surface; the sinks already gate on the error.
- Any change to `api-server`'s separate `validate_external_url` (that path is tracked by the archived `security-ssrf-outbound-url-validation` plan and lives in a different crate/binary).

## After-merge
- Move this file to `plans/_archive/code-review-reality-server-ssrf-ipv4-mapped-ipv6.md`
- Mark the matching `backlog.json` row as `status: "done"`
