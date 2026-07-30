# code-review-reality-server-ssrf-ipv4-mapped-ipv6

**Vector:** bug
**Score:** 6
**Source:** tier-1d dev-review 2026-07-30 (reality-server, RE-RAISE from 2026-07-12)
**Confidence:** high

## Hypothesis
`validate_fetch_url()` in reality-server's `url_validator.rs` blocks IPv4 private ranges on the `Host::Ipv4` arm and blocks IPv6 loopback/ULA/link-local on the `Host::Ipv6` arm — but never normalises IPv4-mapped IPv6 addresses (`::ffff:0:0/96`) back to their IPv4 form before running the v4 checks. An attacker who can influence any URL that flows through this validator can bypass every private-v4 guard by encoding the target as `::ffff:169.254.169.254` (AWS/GCP/Azure metadata), `::ffff:127.0.0.1` (loopback), or any RFC1918 range. The fix is a one-line normalisation: in the `Host::Ipv6` arm, call `ip.to_ipv4_mapped()` first and, if it returns `Some(v4)`, fall through to the same v4 rejection logic that guards the `Host::Ipv4` arm. This is the same defense pattern the api-server SSRF hardening (already merged in `services/actions/api_call.rs`) uses.

## Evidence
- `backend/servers/reality-server/src/util/url_validator.rs:141-158` — `Host::Ipv6` arm only checks `is_loopback()`, `fc00::/7`, `fe80::/10`; no v4-mapped normalisation before the arm returns `Ok`.
- `backend/servers/reality-server/src/util/url_validator.rs:100-133` — `Host::Ipv4` arm correctly rejects `10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`, `100.64.0.0/10`, `169.254.0.0/16`, `127.0.0.0/8`, `0.0.0.0/8`, broadcast, multicast — all bypassable via `::ffff:<v4>` today.
- Tier-1d RE-RAISE evidence (2026-07-30): first emitted 2026-07-12, decayed out of `backlog.json`, still unfixed at HEAD `b1f6ad5`.
- Cross-signal confirmation: `code-review-reality-server-url-validator-ipv4-mapped-ipv6-ssrf` (tier-1d slice 3, independent) flags the same file:line with the same root cause — two blind passes converged on the identical finding.
- Related archive: `.research/plans/_archive/security-ssrf-outbound-url-validation.md` shipped the api-server sinks (signatures.rs, integrations.rs webhook) via `validate_external_url`; reality-server's separate implementation was never brought to parity.

## Files
- `backend/servers/reality-server/src/util/url_validator.rs:150`
- `backend/servers/reality-server/src/util/url_validator.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode:** `Mode: cloud-ok`

## Repro steps
1. Build reality-server (`cd backend && cargo test -p reality-server --lib url_validator`).
2. In a Rust test, call `validate_fetch_url("http://[::ffff:169.254.169.254]/latest/meta-data/")`.
3. Expected: `Err(UrlValidationError::PrivateOrReservedIp(_))`. Actual: `Ok(_)` — request would be dispatched, hitting cloud metadata.
4. Also test `http://[::ffff:127.0.0.1]/`, `http://[::ffff:10.0.0.1]/`, `http://[::ffff:192.168.1.1]/` — all currently pass validation.

## Suggested approach
1. In `backend/servers/reality-server/src/util/url_validator.rs`, modify the `Host::Ipv6(ip)` arm at ~line 145. Before running the existing loopback/ULA/link-local checks, call `ip.to_ipv4_mapped()`. If it returns `Some(v4)`, delegate to the same v4-rejection logic used by `Host::Ipv4(ip)` (either by extracting a shared `reject_private_v4(v4: Ipv4Addr) -> Result<(), UrlValidationError>` helper and calling it from both arms, or by re-invoking the match with `Host::Ipv4(v4)`).
2. Ensure the shared helper covers ALL current v4 rejections: `10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`, `100.64.0.0/10`, `169.254.0.0/16`, `127.0.0.0/8`, `0.0.0.0/8`, `255.255.255.255` (broadcast), `224.0.0.0/4` (multicast).
3. Also consider `::` (unspecified) and `::/128` — reject as unspecified.
4. Add tests (see *Test plan*).
5. Run `cargo test -p reality-server` locally; run `cargo clippy -p reality-server --all-targets -- -D warnings`.
6. No API surface change; no migration; no client change.

## Alternatives considered
- **Rewrite around a third-party allowlist crate (e.g. `ssrf-guard`)** — rejected because it adds a runtime dependency for one function of ~40 lines, and the fix is a well-understood one-liner. The current custom implementation already covers the intended surface — it's just missing the v4-mapped normalisation.
- **Add the normalisation inside the caller sites instead of the validator** — rejected because there are multiple callers (search-alerts drainer, imports, agency_imports) and the whole point of a validator utility is to be the single choke point. Fix at the choke point, once.

## Root-cause trace
1. Symptom: `validate_fetch_url("http://[::ffff:169.254.169.254]/")` returns `Ok(_)`, so the caller dispatches to cloud metadata.
2. ← `url_validator.rs:145` — `Host::Ipv6(ip)` arm never checks `ip.to_ipv4_mapped()`.
3. ← Initial implementation (first emitted 2026-07-12) mirrored a v4-only guard; the v6 arm was tacked on without cross-mapping.
4. Origin: the reality-server `url_validator.rs` was added standalone without inheriting the api-server `services/actions/api_call.rs::validate_external_url` helper, which does normalise the v4-mapped case.

## Test plan
- [ ] Add unit tests in `backend/servers/reality-server/src/util/url_validator.rs`'s existing `#[cfg(test)]` block covering:
  - `::ffff:169.254.169.254` → `Err(PrivateOrReservedIp(_))`
  - `::ffff:127.0.0.1` → `Err(_)`
  - `::ffff:10.0.0.1`, `::ffff:172.16.0.1`, `::ffff:192.168.1.1` → `Err(_)`
  - `::ffff:100.64.0.1` → `Err(_)` (CGNAT)
  - `::ffff:0.0.0.0` → `Err(_)` (unspecified via mapped)
  - `::1` → `Err(_)` (regression — still rejected)
  - Positive: `::ffff:8.8.8.8` → `Ok(_)` (public v4-mapped still allowed)
  - Positive: `[2001:db8::1]` → `Ok(_)` (public v6 still allowed)
- [ ] Command: `cd backend && cargo test -p reality-server --lib util::url_validator`.
- [ ] Regression: `cd backend && cargo test -p reality-server` (full crate).

## Out of scope
- Migrating callers to the api-server `validate_external_url` implementation — a broader refactor; different plan.
- Adding DNS-resolution-time SSRF checks (the `Host::Domain` branch's TODO). Different follow-up.
- Cross-server code deduplication of the two `url_validator` implementations.

## After-merge
- Move this file to `plans/_archive/code-review-reality-server-ssrf-ipv4-mapped-ipv6.md`
- Mark the matching `backlog.json` row as `status: "done"`
