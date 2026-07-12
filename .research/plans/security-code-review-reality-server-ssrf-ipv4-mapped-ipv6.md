# security-code-review-reality-server-ssrf-ipv4-mapped-ipv6

**Vector:** security
**Score:** 3
**Source:** hotspot in `backend/servers/reality-server/src/util/url_validator.rs`
**Confidence:** high

## Hypothesis
`validate_fetch_url` (reality-server) blocks private / loopback / link-local / metadata IPs on the `Host::Ipv4` branch (`url_validator.rs:93-144`) but only rejects `is_loopback()`, `fc00::/7`, and `fe80::/10` on `Host::Ipv6` (`:145-164`). It does not handle IPv4-mapped IPv6 literals (`::ffff:a.b.c.d`), IPv4-compatible IPv6 (`::0:0:0:0:0:0:a.b.c.d`), or 6to4 (`2002::/16`), so a URL like `https://[::ffff:169.254.169.254]/` parses as `Host::Ipv6`, hits no rule, and passes validation — a direct SSRF bypass to the cloud-metadata endpoint. The same gap lets `::ffff:127.0.0.1`, `::ffff:10.x.x.x`, and `::ffff:192.168.x.x` reach loopback / private targets the IPv4 branch is careful to block. Fix: after parsing, if `Host::Ipv6(ip)` and `ip.to_ipv4_mapped()` returns `Some(v4)`, delegate to the same private-IPv4 rejection logic; then extend the IPv6 rules to cover the remaining edge cases.

## Evidence
- `backend/servers/reality-server/src/util/url_validator.rs:145-164` — `Host::Ipv6(ip)` arm rejects only `is_loopback`, `fc00::/7` (segments[0] & 0xfe00 == 0xfc00), `fe80::/10` (segments[0] & 0xffc0 == 0xfe80). No IPv4-mapped handling.
- `backend/servers/reality-server/src/util/url_validator.rs:93-144` — the IPv4 branch explicitly blocks 127.0.0.0/8, 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16, 169.254.0.0/16 (link-local incl. cloud metadata 169.254.169.254), and 224.0.0.0/4. This is the correct blocklist — the IPv6 gap can be closed by routing IPv4-mapped through it.
- Signal `code-review-reality-server-ssrf-ipv4-mapped-ipv6` — confidence=high, +3 (source: 2026-07-12 reality-server segment code review).
- `Ipv6Addr::to_ipv4_mapped(&self) -> Option<Ipv4Addr>` — std since 1.63, returns Some for `::ffff:0:0/96` addresses and None otherwise. Perfect fit — no external crate needed.

## Files
- `backend/servers/reality-server/src/util/url_validator.rs:145`
- `backend/servers/reality-server/src/util/url_validator.rs`

## Dependencies
(none)

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
1. Call `validate_fetch_url("https://[::ffff:169.254.169.254]/latest/meta-data/")` — observe `Ok(())` (passes, allowing SSRF to cloud metadata).
2. Call `validate_fetch_url("https://[::ffff:127.0.0.1]/")` — observe `Ok(())` (passes, allowing loopback bypass).
3. Call `validate_fetch_url("http://169.254.169.254/latest/meta-data/")` — observe `Err(PrivateOrReservedIp("169.254.0.0/16 (IPv4 link-local incl. cloud metadata)"))` — the IPv4 branch correctly blocks the plain form.
4. Expected (after fix): all three return `Err(PrivateOrReservedIp(...))` — the IPv4-mapped variants MUST reach the same rejection reason as the plain IPv4 form.

## Suggested approach
1. In `url_validator.rs` at the top of the `Host::Ipv6(ip)` arm (line ~145), call `if let Some(v4) = ip.to_ipv4_mapped() { return check_ipv4(v4); }` where `check_ipv4` is the same private/reserved rejection routine used by the `Host::Ipv4` arm. Extract that logic into a `fn check_ipv4(ip: Ipv4Addr) -> Result<(), UrlValidationError>` if it isn't already reusable — the current `Host::Ipv4` arm's body is the seed.
2. After the ipv4-mapped fast-path, add rejection for `Ipv6Addr::UNSPECIFIED` (`::`) — SSRF-relevant edge case.
3. Add rejection for 6to4 `2002::/16` when the embedded IPv4 (bits 16..48) is private — reuse `check_ipv4` on the extracted address so the same blocklist governs both. This closes the 6to4 tunnel path that could carry a private IPv4 through an IPv6 form.
4. Preserve the existing `is_loopback`, `fc00::/7`, `fe80::/10` rules unchanged — they cover the pure-IPv6 cases.
5. Add a `#[cfg(test)] mod tests` block (or extend existing) in `url_validator.rs` with the *Repro steps* above as unit-test cases plus positive cases (public IPv6 literal `2001:db8::1` still validates OK).
6. Grep the api-server and reality-server for other callers of `validate_fetch_url` — if any local wrapper reimplements URL validation (e.g. for webhook-test POST or signed-document fetch), audit them for the same gap and fix in parallel; if only `validate_fetch_url` is the choke point, done.

## Alternatives considered
- **Reject all IPv6 hosts outright, allow only IPv4** — rejected because reality-server explicitly supports IPv6 fetch targets (portal endpoints on IPv6-only networks); a blanket ban would break real users.
- **Force DNS resolution and re-validate against the resolved IPs only** — rejected as *this* plan's approach because URL literals with an embedded IP already skip DNS. DNS-resolution guarding is a separate defense that the existing `Host::Domain` doc-comment already flags for the worker-side (post-DNS) check; it belongs in that layer, not here.

## Root-cause trace
1. Symptom: `validate_fetch_url("https://[::ffff:169.254.169.254]/")` returns `Ok(())` — falsely allowing SSRF to the AWS/GCP metadata endpoint (from *Repro* step 1).
2. ← Immediate cause at `backend/servers/reality-server/src/util/url_validator.rs:145-164` — the `Host::Ipv6` arm checks segments[0] against `fc00::/7` and `fe80::/10` masks only; the address `::ffff:169.254.169.254` has segments[0]==0, so no rule matches.
3. ← Upstream cause at `backend/servers/reality-server/src/util/url_validator.rs` — the original blocklist author enumerated IPv6-native private ranges (unique-local, link-local) but did not consider that the URL parser treats `::ffff:a.b.c.d` as `Host::Ipv6`, so the IPv4 blocklist never runs.
4. Origin: introduced with `validate_fetch_url` itself (predates this observation window; not any recent PR). Landed as part of an earlier SSRF hardening (see backlog row `security-ssrf-outbound-url-validation`, done 2026-05-25) that focused on the IPv4 axis.

## Test plan
- [ ] New unit test `ipv4_mapped_metadata_ipv6_is_rejected` in `url_validator.rs::tests` — assert `validate_fetch_url("https://[::ffff:169.254.169.254]/")` returns `Err(PrivateOrReservedIp(_))`.
- [ ] `ipv4_mapped_loopback_ipv6_is_rejected` — `[::ffff:127.0.0.1]` → error.
- [ ] `ipv4_mapped_private_10_is_rejected`, `ipv4_mapped_private_192_168_is_rejected`, `ipv4_mapped_private_172_16_is_rejected` — the three RFC 1918 ranges via IPv4-mapped form.
- [ ] `ipv6_unspecified_is_rejected` — `::` → error.
- [ ] `ipv6_6to4_embedding_private_ipv4_is_rejected` — `2002:a00::` (embeds 10.0.0.0) → error.
- [ ] Non-regression positive: `public_ipv6_literal_is_ok` — `2001:db8::1` → `Ok(())`; and `public_ipv4_literal_is_ok` — `1.1.1.1` → `Ok(())`.
- [ ] Command: `cargo test -p reality-server url_validator`

## Out of scope
- DNS-resolution-time re-validation (belongs in the worker layer, already flagged in the `Host::Domain` doc comment).
- IPv6 broadcast / multicast (`ff00::/8`) — separate hardening, low SSRF risk on cloud-egress paths, out of scope here.
- Backporting to `api-server`'s outbound URL guards if they have their own copy — do that as a follow-up if grep finds duplicated logic; keep this PR focused on `url_validator.rs`.

## After-merge
- Move this file to `plans/_archive/security-code-review-reality-server-ssrf-ipv4-mapped-ipv6.md`
- Mark backlog row `security-code-review-reality-server-ssrf-ipv4-mapped-ipv6` as `status: "done"`
