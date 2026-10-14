# gh-issue-3084-ssrf-nat64-siit-bypass

**Vector:** security
**Score:** 3
**Source:** Issue #3084
**Confidence:** high

## Hypothesis
`is_forbidden_ip` in `backend/crates/common/src/url_validation.rs` unmaps only `::ffff:…` (IPv4-mapped) and `::/96` (IPv4-compatible) IPv6 addresses. The NAT64 well-known prefix `64:ff9b::/96` and the SIIT `::ffff:0:0/96` embedding both carry an IPv4 payload the static check never surfaces, so `http://[64:ff9b::169.254.169.254]/` and `http://[::ffff:0:127.0.0.1]/` reach the cloud-metadata IP and loopback respectively. Extracting the embedded IPv4 from both prefixes and routing it through `check_ipv4` closes the hole with the same shape as the PR #3082 fix.

## Evidence
- `backend/crates/common/src/url_validation.rs:299` — `is_forbidden_ip`; only `to_ipv4_mapped()` + `to_ipv4()` branches handle v4-in-v6
- `backend/crates/common/src/url_validation.rs:525` — `rejects_ipv4_compatible_ipv6_metadata` test guards the compatible form but not NAT64
- Issue #3084 (owner-filed, follow-up to PR #3082 merged 2026-10-14)
- RFC 6052 §2.1 (NAT64 well-known prefix 64:ff9b::/96) and §2.2 note (SIIT ::ffff:0:0/96)

## Files
- `backend/crates/common/src/url_validation.rs:299`
- `backend/crates/common/src/url_validation.rs:525`

## Dependencies

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
1. In `backend/crates/common`, run `cargo test -p ppt-common url_validation::` on `dev`.
2. Add a test that calls the module's public URL validator with `https://[64:ff9b::a9fe:a9fe]/` (and separately with `https://[::ffff:0:7f00:1]/`).
3. Expected: validator rejects both as forbidden-IP. Actual (on `dev`): validator accepts both — the embedded IPv4 (169.254.169.254 / 127.0.0.1) is never surfaced.

## Suggested approach
1. In `is_forbidden_ip` at `backend/crates/common/src/url_validation.rs:299`, after the existing `to_ipv4_mapped()` / `to_ipv4()` branches, inspect `v6.segments()`.
2. If `segs[0..6] == [0x0064, 0xff9b, 0, 0, 0, 0]` (NAT64 well-known prefix per RFC 6052), extract the low 32 bits into an `Ipv4Addr` and `return check_ipv4(v4)`.
3. If `segs[0..6] == [0, 0, 0, 0, 0xffff, 0]` (SIIT `::ffff:0:0/96`), do the same.
4. Add regression tests next to `rejects_ipv4_compatible_ipv6_metadata` at `backend/crates/common/src/url_validation.rs:525`: `rejects_nat64_metadata` (`https://[64:ff9b::169.254.169.254]/`) and `rejects_siit_translated_loopback` (`https://[::ffff:0:127.0.0.1]/`).
5. Keep the extraction inline — do not refactor the three branches into a single helper in this PR; the issue's "longer term" note is explicitly out of scope.
6. Run `cargo test -p ppt-common url_validation::` and confirm both new tests fail on the pre-fix code and pass after the branch additions.
7. Run `cargo fmt --all` and `cargo clippy -p ppt-common -- -D warnings` before pushing.

## Alternatives considered
- **Canonicalise every low-32-bit-embedded IPv6 through one helper** — rejected because the issue explicitly scopes that refactor out ("longer term"); mixing a cleanup with a security fix widens the review surface and the PR #3082 pattern was to add branches, not refactor.
- **Deny-list the two prefixes without extracting the embedded IPv4** — rejected because then any address in `64:ff9b::/96` would be denied uniformly, including IPv4 destinations outside the forbidden set (`64:ff9b::8.8.8.8`); the correct policy is to apply the IPv4 check to the embedded address.

## Root-cause trace
1. Symptom: `https://[64:ff9b::169.254.169.254]/` passes `is_forbidden_ip` and reaches the IMDS metadata endpoint.
2. ← At `backend/crates/common/src/url_validation.rs:299`, the `IpAddr::V6(v6)` branch runs `to_ipv4_mapped()` (returns `None` — not `::ffff:`-prefixed) then `to_ipv4()` (returns `None` — not `::/96`), falls through to the fc00/fe80/ff00 masks (all miss), and returns `Ok(())`.
3. ← PR #3082 (merged 2026-10-14) added the `to_ipv4()` branch to catch `::127.0.0.1`-style addresses but did not add branches for NAT64 or SIIT embeddings — the issue note documents the same `segs[0]` + mask argument.
4. Origin: PR #3082 (shaped the current branch set) + PR #2992 (dedup that first dropped the IPv4-mapped unmapping, which #3082 restored partially).

## Test plan
- [ ] `backend/crates/common/src/url_validation.rs` — `rejects_nat64_metadata` test asserting `https://[64:ff9b::169.254.169.254]/` is rejected
- [ ] `backend/crates/common/src/url_validation.rs` — `rejects_siit_translated_loopback` test asserting `https://[::ffff:0:127.0.0.1]/` is rejected
- [ ] Confirm both tests fail on `dev` HEAD before the branch additions, pass after
- [ ] `cd backend && cargo test -p ppt-common url_validation::`

## Out of scope
- Collapsing the three v4-in-v6 branches into a single helper (issue §"longer term" — defer to its own refactor PR).
- Extending the check to other RFC 6052 prefixes beyond `64:ff9b::/96` and `::ffff:0:0/96` (none are deployed as well-known SSRF-reachable embeddings).
- Any outbound-fetch call-site changes — this plan is purely the static validator.

## After-merge
- Move this file to `plans/_archive/gh-issue-3084-ssrf-nat64-siit-bypass.md`
- Mark the matching `backlog.json` row as `status: "done"`
