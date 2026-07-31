# code-review-reality-server-ssrf-ipv4-mapped-ipv6

**Vector:** security
**Score:** 3
**Source:** dispatcher tier1d review .research/signals/2026-07-31-reality-server-tier1d.json
**Confidence:** high

## Hypothesis
`validate_fetch_url` in `backend/servers/reality-server/src/util/url_validator.rs:145-165` rejects the standard IPv6 private/loopback ranges but never converts IPv4-mapped IPv6 (`::ffff:a.b.c.d`) back to the wrapped v4 address before the IPv4 checks run. `Ipv6Addr::is_loopback()` returns false for `::ffff:127.0.0.1`, and the `fc00::/7` / `fe80::/10` masks never match a mapped address. A user-controlled URL of the form `http://[::ffff:127.0.0.1]/` (or the equivalent mapped-RFC1918 address) therefore passes validation and reaches the outbound fetch worker, giving reality-server a classic SSRF bypass into loopback / RFC1918 through the OTA/webhook fetch path.

## Evidence
- `backend/servers/reality-server/src/util/url_validator.rs:145-165` — the `Host::Ipv6` branch only checks `is_loopback()` (which is `::1`-specific), `fc00::/7`, and `fe80::/10`. It never calls `Ipv6Addr::to_ipv4_mapped()` (or the older `to_ipv4()`) to funnel mapped addresses through the same rejection list as `Host::Ipv4`.
- Rust stdlib: `Ipv6Addr::is_loopback` returns `false` for `::ffff:127.0.0.1` (only `::1` matches); `Ipv6Addr::to_ipv4_mapped` returns `Some(Ipv4Addr)` iff the top 80 bits are zero and bits 80-96 are `ffff`.
- Dispatcher signal `code-review-reality-server-ssrf-ipv4-mapped-ipv6` (2026-07-31, confidence=high, score_delta=+3).
- No existing test in `url_validator.rs`'s `#[cfg(test)]` block covers a mapped-IPv6 loopback or RFC1918 address (confirmed via inspection of the `#[cfg(test)]` region below line ~175).

## Files
- `backend/servers/reality-server/src/util/url_validator.rs:145`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug vector)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):** no C4/C5 → `cloud-ok`

Mode: cloud-ok

## Repro steps
1. Check out `dev` at HEAD, run `cd backend && cargo test -p reality-server url_validator -- --nocapture` — passes (no coverage for mapped v6).
2. Add a unit test to `backend/servers/reality-server/src/util/url_validator.rs` (`#[cfg(test)] mod tests`) asserting `validate_fetch_url("http://[::ffff:127.0.0.1]/")` returns `Err(UrlValidationError::PrivateOrReservedIp(_))`. Run — expected FAIL, observed FAIL (test proves the bypass).
3. Re-run after applying the fix — expected PASS.

## Suggested approach
1. In `validate_fetch_url` at the top of the `Host::Ipv6(ip)` arm (line 145+), call `ip.to_ipv4_mapped()`; if it returns `Some(v4)`, dispatch to the existing `Host::Ipv4` rejection logic (extract the IPv4 checks into `fn reject_private_ipv4(ip: Ipv4Addr) -> Result<(), UrlValidationError>` and call it from both arms). Also handle `to_ipv4_compatible` variants where relevant, though those are deprecated.
2. Preserve the existing three IPv6-native checks (`is_loopback`, `fc00::/7`, `fe80::/10`) for cases the mapped-conversion doesn't cover.
3. Add unit tests for: `::ffff:127.0.0.1` (mapped loopback), `::ffff:10.0.0.1` (mapped RFC1918), `::ffff:169.254.169.254` (mapped AWS metadata), `::ffff:0.0.0.0` (mapped unspecified), and a negative case `::ffff:1.2.3.4` (must be accepted — a routable public v4 wrapped as v6 is still routable).
4. Grep the reality-server codebase for other users of `Ipv6Addr` to ensure no other validator has the same shape: `rg 'Ipv6Addr' backend/servers/reality-server/`.

## Alternatives considered
- **Block all IPv6 outbound entirely** — rejected because reality-server does legitimately fetch OTA / listing photos over v6 in some staging environments; the fix should be a narrow bypass patch, not an availability regression.
- **Rely on kernel/socket-level source-address filtering** — rejected because it relies on host-network config the app has no way to assert, and the fix must be in the URL-validation layer so `SKIP_NETWORK=1` tests can pin it.

## Root-cause trace
1. Symptom: `validate_fetch_url("http://[::ffff:127.0.0.1]/")` returns `Ok(_)` when it should be rejected as reaching a loopback address.
2. ← `backend/servers/reality-server/src/util/url_validator.rs:146` — `Host::Ipv6(ip)` arm checks `ip.is_loopback()`, which is `false` for mapped-v4 form.
3. ← `backend/servers/reality-server/src/util/url_validator.rs:151-164` — the fc00::/7 and fe80::/10 mask checks operate on `segments[0]`, which for `::ffff:*` is 0x0000, so both masks miss.
4. Origin: the initial commit that introduced `validate_fetch_url`'s IPv6 branch — it never enumerated the mapped-address bypass. The pattern was documented as a CVE class (CVE-2021-42574 family / OWASP SSRF ch. 3) but the writer of this validator didn't include `to_ipv4_mapped` in the rejection tree.

## Test plan
- [ ] `backend/servers/reality-server/src/util/url_validator.rs` — new `#[test]` `mapped_ipv6_loopback_rejected` asserting `validate_fetch_url("http://[::ffff:127.0.0.1]/")` errors with `PrivateOrReservedIp`
- [ ] `backend/servers/reality-server/src/util/url_validator.rs` — new `#[test]` `mapped_ipv6_rfc1918_rejected` asserting mapped `::ffff:10.0.0.1` errors
- [ ] `backend/servers/reality-server/src/util/url_validator.rs` — new `#[test]` `mapped_ipv6_metadata_rejected` asserting mapped `::ffff:169.254.169.254` errors
- [ ] `backend/servers/reality-server/src/util/url_validator.rs` — negative `#[test]` `mapped_ipv6_public_accepted` asserting `::ffff:1.2.3.4` passes (must not over-reject)
- [ ] `cd backend && cargo test -p reality-server url_validator`
- [ ] `cd backend && cargo clippy -p reality-server --all-targets -- -D warnings`

## Out of scope
- IPv6 DNS-rebind protection (worker-time re-resolution is deliberately deferred per the existing `Host::Domain` comment; that is a separate epic).
- Auditing other reality-server URL entry points beyond `validate_fetch_url` — this plan is a targeted patch, not a broader SSRF sweep. A follow-up scan (`rg 'Ipv6Addr'`) is called out in *Suggested approach* step 4 for signal-generation, not for in-PR patching.
- IPv4-compatible (deprecated) `::/96` addresses — vanishingly small real-world risk, skip unless a follow-up finding surfaces one.

## After-merge
- Move this file to `plans/_archive/code-review-reality-server-ssrf-ipv4-mapped-ipv6.md`
- Mark the matching `backlog.json` row as `status: "done"`
