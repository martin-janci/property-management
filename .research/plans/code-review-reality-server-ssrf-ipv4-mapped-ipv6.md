# code-review-reality-server-ssrf-ipv4-mapped-ipv6

**Vector:** security
**Score:** 3
**Source:** rotating-expert-review 2026-07-31 (reality-server tier-1d)
**Confidence:** high

## Hypothesis
`validate_fetch_url` in `backend/servers/reality-server/src/util/url_validator.rs` rejects loopback (::1), unique-local (fc00::/7) and link-local (fe80::/10) IPv6 addresses, but never canonicalizes IPv4-mapped IPv6 literals (`::ffff:a.b.c.d`) or IPv4-compatible addresses before re-checking the IPv4 blocklist. An attacker can encode any blocked IPv4 target (127.0.0.1, 10.0.0.0/8, and — critically — the AWS/GCP metadata IP 169.254.169.254) as a mapped IPv6 URL literal to defeat the entire SSRF guard. Because IP literals are not re-resolved by the worker, the "worker re-validates resolved IP" defense-in-depth does not apply. The fix is a two-line change: after matching `Host::Ipv6`, call `ip.to_ipv4_mapped()` / `to_ipv4()` and re-run the IPv4 blocklist on the embedded address; also reject the unspecified address `::`.

## Evidence
- `backend/servers/reality-server/src/util/url_validator.rs:145-164` — Host::Ipv6 branch only rejects `is_loopback()` (::1), `fc00::/7`, `fe80::/10`; no IPv4-mapped canonicalization.
- `backend/servers/reality-server/src/routes/agency_imports.rs:238` (test_connection) and `:290` (run_import) — user-controlled `data.feed_url` reaches this guard as the ONLY pre-fetch SSRF gate.
- Bypass payloads confirmed reachable: `https://[::ffff:169.254.169.254]/latest/meta-data/`, `https://[::ffff:127.0.0.1]/`, `https://[::ffff:10.0.0.5]/feed.xml`.
- Rust std: `Ipv6Addr::is_loopback` returns false for `::ffff:127.0.0.1`; the fc00/fe80 segment[0] masks (0xfe00/0xffc0) never match `::ffff:...` (segment[0] = 0).
- Signal id: `code-review-reality-server-ssrf-ipv4-mapped-ipv6` (rotating-expert-review 2026-07-31, expert=security, confidence=high, score_delta=3).

## Files
- `backend/servers/reality-server/src/util/url_validator.rs:145`
- `backend/servers/reality-server/src/routes/agency_imports.rs:238`
- `backend/servers/reality-server/src/routes/agency_imports.rs:290`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (security bug — verify blocklist coverage against every canonical form)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

Rust-only change verifiable via `cargo test -p reality-server url_validator`; no browser, no ADB.

## Repro steps
1. Add a unit test in `backend/servers/reality-server/src/util/url_validator.rs` (the file already carries a `#[cfg(test)] mod tests` block at line 175) that calls `validate_fetch_url("https://[::ffff:169.254.169.254]/latest/meta-data/")` and asserts `Err(UrlValidationError::PrivateOrReservedIp(_))`.
2. Run `cargo test -p reality-server url_validator`. **Expected on `dev` today:** the assertion fails — the URL is accepted, proving the SSRF-guard bypass. **After the fix:** the same assertion passes with a `PrivateOrReservedIp("169.254.169.254 (link-local metadata)"))`-style error.

## Suggested approach
1. In `url_validator.rs`, add a shared IPv4-classification helper (or extract the existing `Host::Ipv4` body into `fn classify_ipv4(ip: Ipv4Addr) -> Result<(), UrlValidationError>`) so IPv4 rejection logic isn't duplicated across the two arms.
2. In `Host::Ipv6(ip)` (line 145), *before* the loopback / fc00 / fe80 checks, canonicalize: `if let Some(v4) = ip.to_ipv4_mapped() { return classify_ipv4(v4); }` (Rust 1.75+ has `to_ipv4_mapped` stable — matches `rust-version = 1.75` in `backend/Cargo.toml`). This also handles IPv4-compatible addresses (`::a.b.c.d`) — reject `to_ipv4()` results whose upper 96 bits are zero *and* not the unspecified `::`.
3. Add explicit rejection of the unspecified IPv6 address `::` (`ip.is_unspecified()`) with a `PrivateOrReservedIp("::")` error.
4. Add unit tests in the existing `#[cfg(test)] mod tests` block covering: `[::ffff:169.254.169.254]`, `[::ffff:127.0.0.1]`, `[::ffff:10.0.0.5]`, `[::]`, and one *positive* case (`[2001:db8::1]`) to prove the guard hasn't over-rejected legitimate public IPv6.
5. Run `cargo test -p reality-server url_validator` — expect all 5 new tests to pass and the existing suite to remain green.
6. Run `cargo clippy -p reality-server -- -D warnings` and `cargo fmt --all` to satisfy CI.
7. Do NOT modify the two call sites in `agency_imports.rs` (they already delegate to `validate_fetch_url` — the fix is entirely inside the validator).

## Alternatives considered
- **Deep-diff the Public Suffix List / IANA reserved-range table into a data file** — rejected because the bug isn't a missing range; it's a missing canonicalization step. Every existing range check is already correct for the *canonical* form; converting mapped IPv6 → IPv4 restores those checks with no data churn.
- **Add DNS-rebinding defence in the worker as the primary fix** — rejected because the SSRF-guard bypass fires on *literal IPs*, which are never DNS-resolved. Worker-side re-validation is important defence-in-depth (already documented as a TODO at agency_imports.rs:287) but does not close this specific hole.

## Root-cause trace
1. Symptom: `validate_fetch_url("https://[::ffff:169.254.169.254]/...")` returns Ok instead of Err(PrivateOrReservedIp).
2. ← `Host::Ipv6` arm at `backend/servers/reality-server/src/util/url_validator.rs:145` treats a mapped literal as "just an IPv6 address" — the loopback / fc00 / fe80 checks all evaluate to false for `::ffff:*`.
3. ← The validator's shape assumes each `Host::` variant carries a *distinct* address family; `::ffff:127.0.0.1` violates that assumption at the type level (it's IPv6 syntactically, IPv4 semantically), and no canonicalization bridges the two.
4. Origin: introduced together with the whole `validate_fetch_url` helper — commit history for this file predates the routine's cursor; PR is unknown but the helper has been present in this form since (at least) the round-9 SSRF audit that added the SECURITY H3 comment at agency_imports.rs:232. Same-shape bug in every branch of the IPv6 arm.

## Test plan
- [ ] Unit test `test_rejects_ipv4_mapped_metadata_ip` in `backend/servers/reality-server/src/util/url_validator.rs` — `validate_fetch_url("https://[::ffff:169.254.169.254]/latest/meta-data/")` returns `Err(UrlValidationError::PrivateOrReservedIp(_))`. Fails on `dev` today (regression test — IG3).
- [ ] Unit test `test_rejects_ipv4_mapped_loopback` — `validate_fetch_url("https://[::ffff:127.0.0.1]/")` returns Err.
- [ ] Unit test `test_rejects_ipv4_mapped_private` — `validate_fetch_url("https://[::ffff:10.0.0.5]/feed.xml")` returns Err.
- [ ] Unit test `test_rejects_unspecified_ipv6` — `validate_fetch_url("https://[::]/")` returns Err.
- [ ] Unit test `test_accepts_public_ipv6` — `validate_fetch_url("https://[2001:db8::1]/")` returns Ok (guard didn't over-reject).
- [ ] Exact command: `cargo test -p reality-server url_validator` — expect all 5 new tests green plus the pre-existing suite unchanged.

## Out of scope
- Worker-side re-validation after DNS resolution (the TODO at `agency_imports.rs:287`) — that closes a different attack (DNS rebinding on hostnames), not the IPv4-mapped bypass. File a follow-up if desired.
- Rate-limiting or auth changes on `test_connection` / `run_import` — the endpoints are already gated by `check_agency_membership`.
- Any change to the public IPv4 blocklist ranges — the existing ones are complete for canonical form.

## After-merge
- Move this file to `plans/_archive/code-review-reality-server-ssrf-ipv4-mapped-ipv6.md`
- Mark the matching `backlog.json` row (`code-review-reality-server-ssrf-ipv4-mapped-ipv6`) as `status: "done"`
