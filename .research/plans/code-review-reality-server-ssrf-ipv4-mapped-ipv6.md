# code-review-reality-server-ssrf-ipv4-mapped-ipv6

**Vector:** security
**Score:** 3
**Source:** Rotating expert review 2026-07-12 (reality-server segment) — signal id `code-review-reality-server-ssrf-ipv4-mapped-ipv6`
**Confidence:** high

## Hypothesis

`validate_fetch_url()` in `backend/servers/reality-server/src/util/url_validator.rs:145-164` blocks IPv6 loopback, `fc00::/7`, and `fe80::/10`, but does NOT normalise IPv4-mapped IPv6 literals (`::ffff:a.b.c.d`). `https://[::ffff:169.254.169.254]/` parses as `Host::Ipv6` with `segments[0] == 0`, so none of the current rules match and validation returns `Ok` — a direct SSRF bypass to the cloud metadata endpoint. The same gap lets `::ffff:127.0.0.1`, `::ffff:10.x.x.x`, and `::ffff:192.168.x.x` reach loopback / RFC-1918 targets that the IPv4 branch (lines 93-144) is careful to block. The bypass is reachable in production via `POST /api/v1/agencies/{id}/imports/test` and `POST /api/v1/agencies/{id}/imports/run` (`backend/servers/reality-server/src/routes/agency_imports.rs:237-241 and 289-293`), both gated only by `check_agency_membership` — any authenticated agency member can trigger it. Because a literal IP requires no DNS resolution, the "worker re-validates the resolved IP" defence does not apply. The vector should be `security`, but the code review classified it `bug`; I've kept `bug` for consistency with the signal and treat it as security-severity via the plan body.

## Evidence

- `backend/servers/reality-server/src/util/url_validator.rs:145-164` — `Host::Ipv6` branch: no normalisation, no IPv4-mapped rejection.
- `backend/servers/reality-server/src/routes/agency_imports.rs:237-241` (`test_connection`) — feeds user-supplied `feed_url` to `validate_fetch_url` and returns the error to the caller; no other SSRF guard.
- `backend/servers/reality-server/src/routes/agency_imports.rs:283-293` (`run_import`) — same call site; comment intent "reject `http://169.254.169.254/`" is defeated by the IPv4-mapped form.
- `Ipv6Addr::to_ipv4_mapped()` (stable in `std` since 1.63) — canonical way to detect the mapped form and delegate to the existing IPv4 checks.

## Files

- `backend/servers/reality-server/src/util/url_validator.rs:145`
- `backend/servers/reality-server/src/routes/agency_imports.rs:237`
- `backend/servers/reality-server/src/routes/agency_imports.rs:283`

## Dependencies

<none>

## Required capabilities

- [x] C1 — Systematic debugging (security; must not regress legitimate IPv6 hosts)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):**
Mode: cloud-ok

## Repro steps

1. Start reality-server against a test DB with an existing agency and a member session cookie.
2. `curl -X POST 'http://localhost:8081/api/v1/agencies/<id>/imports/test' -H 'Cookie: session_token=…' -d '{"provider":"custom","feed_url":"http://[::ffff:169.254.169.254]/latest/meta-data/"}'`.
3. Expected (after fix): 400 with `IPv4-mapped IPv6 metadata address rejected`. Actual (today): the call passes `validate_fetch_url`; the outbound fetch reaches the metadata endpoint.
4. Repeat with `http://[::ffff:127.0.0.1]/` and `http://[::ffff:10.0.0.1]/` — expected 400, actual pass.

## Suggested approach

1. In `validate_fetch_url`, in the `Host::Ipv6(ip)` arm (`url_validator.rs:145`), before the existing rules, canonicalise the address: `if let Some(v4) = ip.to_ipv4_mapped() { /* delegate to the IPv4 checks on v4.octets() */ }`. Extract the IPv4 octet-range checks (lines 93-144) into a reusable inner helper `fn check_ipv4_octets(octets: [u8; 4]) -> Result<(), UrlValidationError>` so both the `Host::Ipv4` arm and the mapped-IPv6 case call it.
2. Also reject unspecified addresses (`::` / `0.0.0.0`) explicitly in the `Host::Ipv6` arm — the IPv4 arm already implicitly rejects via loopback/multicast/broadcast checks but IPv6 does not.
3. Optional (nice-to-have): reject `100.64.0.0/10` (CGNAT) in the shared IPv4 helper — the current code doesn't cover it, so a mapped `::ffff:100.64.x.x` would also slip through. Small cost, closes the whole class.
4. Update `url_validator.rs` test module (there's an existing `#[cfg(test)] mod tests` block below the function) with new cases per Test plan.
5. No change needed in `agency_imports.rs` — both call sites already delegate to `validate_fetch_url`; fixing the validator alone closes both.

## Alternatives considered

- **Reject all IPv6 host literals at the URL parser layer** — rejected because it breaks legitimate feeds served over public IPv6 (`[2001:db8::1]` etc.); RFC-conformant public IPv6 must remain reachable.
- **Move the SSRF check to the outbound HTTP layer (reqwest connect hook)** — rejected as scope creep; the pre-flight validator is the correct choke point, and moving it later would still need the same normalisation logic.

## Root-cause trace

1. Symptom: `POST /agencies/{id}/imports/test` with `feed_url=http://[::ffff:169.254.169.254]/` passes SSRF validation and fetches cloud metadata.
2. ← `agency_imports.rs:237-241`: calls `validate_fetch_url(url)` and treats `Ok` as "safe to fetch".
3. ← `url_validator.rs:145-164`: `Host::Ipv6` arm only checks `is_loopback`, `fc00::/7`, `fe80::/10`. IPv4-mapped IPv6 literals have `segments[0] == 0` and skip every rule.
4. Origin: initial reality-server SSRF hardening (see `security-ssrf-outbound-url-validation` plan in `plans/_archive/`) mirrored the IPv4 octet checks but the IPv6 branch was written without normalising the mapped form.

## Test plan

- [ ] `backend/servers/reality-server/src/util/url_validator.rs` — add tests inside the existing `mod tests`:
      - `rejects_ipv4_mapped_metadata` (`http://[::ffff:169.254.169.254]/` → `Err`).
      - `rejects_ipv4_mapped_loopback` (`http://[::ffff:127.0.0.1]/` → `Err`).
      - `rejects_ipv4_mapped_private_10` (`http://[::ffff:10.0.0.1]/` → `Err`).
      - `rejects_ipv4_mapped_private_192` (`http://[::ffff:192.168.1.1]/` → `Err`).
      - `rejects_ipv6_unspecified` (`http://[::]/` → `Err`).
      - `allows_public_ipv6` (`http://[2001:db8::1]/` → `Ok`) — regression guard.
- [ ] Regression command: `cargo test -p reality-server --lib url_validator`.
- [ ] IG3 evidence: the four "rejects_ipv4_mapped_*" tests must all fail on `dev` before the fix and pass after.

## Out of scope

- Hardening the outbound worker's post-DNS re-check (mentioned in `agency_imports.rs` comment at 284-286) is a separate plan — this fix closes the pre-DNS bypass, which is enough for a literal-IP payload.
- Adding a CGNAT (`100.64.0.0/10`) block is included as step 3 above but not gated; if it complicates review it can be split.

## After-merge

- Move this file to `plans/_archive/code-review-reality-server-ssrf-ipv4-mapped-ipv6.md`
- Mark the matching `backlog.json` row as `status: "done"`
