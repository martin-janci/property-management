# code-review-reality-server-ssrf-ipv4-mapped-ipv6

**Vector:** security
**Score:** 3
**Source:** rotating-expert-review
**Confidence:** high

## Hypothesis
`validate_fetch_url` in reality-server's `util/url_validator.rs` rejects IPv4 loopback / private / link-local ranges on the IPv4 branch and rejects loopback / fc00::/7 / fe80::/10 on the IPv6 branch, but does not canonicalize IPv4-mapped IPv6 literals (`::ffff:a.b.c.d`). A request URL of `https://[::ffff:169.254.169.254]/latest/meta-data/` parses as `Host::Ipv6` with `segments[0]==0`; no rule matches; the URL passes validation and the agency-imports worker fetches the AWS/EC2 metadata endpoint. Fix: canonicalize the IPv6 address to its embedded IPv4 (via `to_ipv4_mapped()`) before running the octet checks.

## Evidence
- backend/servers/reality-server/src/util/url_validator.rs:145-164 — Host::Ipv6 branch only rejects `is_loopback()`, fc00::/7, fe80::/10.
- backend/servers/reality-server/src/routes/agency_imports.rs:237-241, 289-293 — `test_connection` and `run_import` feed the user-supplied `feed_url` through `validate_fetch_url` as the sole SSRF guard; both are gated only by `check_agency_membership` (any authenticated agency member).
- Inline comment at agency_imports.rs:284-286 explicitly intends to reject `http://169.254.169.254/` — the IPv4-mapped IPv6 form defeats that intent; because a literal IP requires no DNS resolution, the 'worker re-validates the resolved IP' mitigation does not apply.

## Files
- `backend/servers/reality-server/src/util/url_validator.rs:145`
- `backend/servers/reality-server/src/routes/agency_imports.rs`

## Dependencies

## Required capabilities
- [ ] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

Mode: cloud-ok

## Repro steps
1. Log in as any authenticated agency member.
2. `POST /api/v1/agency/imports/test_connection` with body `{ "feed_url": "http://[::ffff:169.254.169.254]/latest/meta-data/" }`.
3. Expected: `400 INVALID_URL` (SSRF blocked). Actual: `validate_fetch_url` accepts the URL and the worker issues the request — the cloud-metadata credentials endpoint is reachable.

## Suggested approach
1. In `validate_fetch_url` (`backend/servers/reality-server/src/util/url_validator.rs:145`), before the IPv6 range checks, call `ip.to_ipv4_mapped()` — if `Some(ipv4)` returned, run the existing IPv4 octet checks against that `ipv4` and reject if it matches loopback / private / link-local / metadata ranges.
2. Also reject the IPv6/IPv4 unspecified address (`::`, `0.0.0.0`) explicitly and consider CGNAT `100.64.0.0/10` while touching the IPv4 branch.
3. Extract a small `ipv4_is_blocked(&Ipv4Addr) -> bool` helper so IPv4 and IPv4-mapped-IPv6 share one predicate (no drift).
4. Add a unit test table in `url_validator.rs` covering: `::ffff:127.0.0.1`, `::ffff:10.0.0.1`, `::ffff:169.254.169.254`, `::ffff:192.168.1.1` (all rejected) plus a valid public IPv4-mapped case (accepted).
5. Add an integration test hitting the `test_connection` route with the metadata URL — asserts `400 INVALID_URL`.

## Alternatives considered
- **String-match `::ffff:` prefix on the URL** — rejected because it misses percent-encoded / bracketless variants and duplicates the range-check logic.
- **DNS-only blocklist** — rejected because a literal IPv6/IPv4-mapped literal skips DNS entirely; range checks against the canonicalized IP are the only sound layer.

## Root-cause trace
1. Symptom: `POST /agency/imports/test_connection {feed_url: http://[::ffff:169.254.169.254]/}` bypasses the SSRF guard and reaches IMDS.
2. ← Immediate cause at url_validator.rs:145 — Host::Ipv6 branch does not canonicalize IPv4-mapped forms before running range rules.
3. ← Upstream cause: the IPv4 branch (lines 93-144) enumerates loopback/private/link-local ranges; the IPv6 branch enumerates only IPv6-native ranges, so IPv4 targets reached via the mapped-address literal see no rule.
4. Origin: the reality-server SSRF hardening PR that added `validate_fetch_url` — introduced the two branches without a canonicalization step.

## Test plan
- [ ] Unit tests in `backend/servers/reality-server/src/util/url_validator.rs` covering the 4 mapped-form variants + the valid mapped-public case.
- [ ] Integration test `agency_imports_test_connection_rejects_ipv4_mapped_metadata` in `backend/servers/reality-server/tests/` — 400 on `[::ffff:169.254.169.254]`.
- [ ] Run `cd backend && cargo test -p reality-server -- url_validator ssrf` locally.

## Out of scope
DNS rebinding defense (would need runtime IP re-check post-resolve), IPv6-native metadata endpoints (AWS uses IPv4 IMDS), and refactoring the agency-imports auth model.

## After-merge
- Move this file to `plans/_archive/code-review-reality-server-ssrf-ipv4-mapped-ipv6.md`
- Mark the matching `backlog.json` row as `status: "done"`
