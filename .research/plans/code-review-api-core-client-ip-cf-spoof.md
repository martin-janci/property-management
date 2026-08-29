# code-review-api-core-client-ip-cf-spoof

**Vector:** bug
**Score:** 3
**Source:** Tier-1d dispatcher code review 2026-08-29 (`api-core` segment, commit `90ca54c6`); `.research/signals/2026-08-29-api-core-tier1d-client-ip.json`
**Confidence:** medium

## Hypothesis
`resolve_client_ip` in `backend/servers/api-server/src/client_ip.rs:236-252` reintroduces the exact spoof class the file was written to close (issue #2789). The routine trusts `CF-Connecting-IP` whenever the socket peer is in the trusted set, but the default trusted set (`private_defaults` at `client_ip.rs:166-172`) is loopback + RFC1918/RFC4193 — the ordinary `nginx` / container-ingress deployment the module doc calls the common case. A plain reverse proxy does **not** strip an inbound `CF-Connecting-IP`, so any client can forge it; the peer (nginx, private IP) is trusted, so the forged value is returned as the client IP. That poisons audit logs and rotates per-request throttle buckets. Refuse to consult `CF-Connecting-IP` unless the operator has explicitly opted into Cloudflare edges via `TRUSTED_PROXY_CIDRS` — the rightmost-untrusted-XFF fallback already handles the generic proxy case.

## Evidence
- `backend/servers/api-server/src/client_ip.rs:1-14` — file docstring frames the module as the fix for issue #2789: "the API was serving behind an `nginx` reverse proxy in production, which does not strip `X-Forwarded-For` … but presented `CF-Connecting-IP` unconditionally, which let an attacker forge the client IP."
- `backend/servers/api-server/src/client_ip.rs:166-172` — `private_defaults` returns the default trusted set: `127.0.0.0/8, ::1/128, 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16, fc00::/7, fe80::/10` — every private-IP proxy peer is trusted by default.
- `backend/servers/api-server/src/client_ip.rs:236-249` — `resolve_client_ip` computes `if !trusted.contains(peer) { return peer } else { cf_connecting_ip(headers).or_else(rightmost_untrusted_xff).unwrap_or(peer) }`. Any non-Cloudflare deployment with a private-IP proxy hits the else branch and blindly trusts the request's `CF-Connecting-IP`.
- `backend/servers/api-server/src/client_ip.rs:318` — an existing test `prefers_cf_connecting_ip_from_trusted_peer()` explicitly locks in the current (buggy) behavior for a private trusted peer — it needs to flip to negative.
- Contract at `client_ip.rs:228-234` documents the current preference order (CF first, then XFF) — a fix has to update both docstring + code + this test.

## Files
- `backend/servers/api-server/src/client_ip.rs:166`
- `backend/servers/api-server/src/client_ip.rs:236`
- `backend/servers/api-server/src/client_ip.rs:318`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):** `cloud-ok`

Mode: cloud-ok

## Repro steps
1. Set `TRUSTED_PROXY_CIDRS` unset (or to any value that includes private ranges) — the default deployment shape.
2. Send an HTTP request from a private-IP peer (`10.0.0.5`) with headers `X-Forwarded-For: 203.0.113.1` and `CF-Connecting-IP: 198.51.100.99`.
3. Observe `resolve_client_ip` returns `198.51.100.99` — the header the attacker chose — instead of `203.0.113.1` (rightmost untrusted XFF hop) or the socket peer. This is what the audit log records and what the per-client throttle buckets on.
4. Expected: only a peer inside a Cloudflare-published range (opted in via `TRUSTED_PROXY_CIDRS`) should cause `CF-Connecting-IP` to be consulted; a generic private proxy should fall through to `rightmost_untrusted_xff`, returning `203.0.113.1`.

## Suggested approach
1. Introduce a second, narrower trust list dedicated to Cloudflare-style edge preference: `TrustedProxies::cf_edges` (empty by default; populated only from an explicit `TRUSTED_CF_EDGE_CIDRS` env var or an opt-in flag). Do **not** default it to `private_defaults()`.
2. Change `resolve_client_ip` (`client_ip.rs:236-249`): when `trusted.contains(peer)` is true, consult `cf_connecting_ip(headers)` **only** if `cf_edges.contains(peer)` is also true. Otherwise skip straight to `rightmost_untrusted_xff` and finally the socket peer.
3. Update the module docstring (`client_ip.rs:1-14`) and the `resolve_client_ip` contract (`client_ip.rs:228-234`) to reflect the two-tier trust: any private proxy → XFF-only; a Cloudflare-listed proxy → CF header preferred.
4. Add an opt-in path in `TrustedProxies::from_env` (`client_ip.rs:158-165`) so operators wiring up Cloudflare set `TRUSTED_CF_EDGE_CIDRS` (or an equivalent) to their edge ranges; document alongside `TRUSTED_PROXY_CIDRS`.
5. Flip the semantics locked in by `prefers_cf_connecting_ip_from_trusted_peer()` (`client_ip.rs:318`): under the new default, a private trusted peer must NOT prefer `CF-Connecting-IP`. Rename it to `ignores_cf_connecting_ip_from_private_peer_without_cf_optin` and add a paired `prefers_cf_connecting_ip_from_cf_edge_peer` covering the opt-in path.
6. Add a negative test `spoofed_cf_connecting_ip_from_private_peer_is_ignored` asserting that a request from `10.0.0.5` with `CF-Connecting-IP: <forged>` and `X-Forwarded-For: <real>` resolves to the rightmost-untrusted-XFF hop.
7. Run `cargo test -p api-server client_ip` — the whole module's unit tests must pass; no other consumer of the module needs to change.

## Alternatives considered
- **Strip `CF-Connecting-IP` at the ingress** — rejected because it pushes correctness onto operator-controlled proxy configuration; some ingresses (managed k8s, older nginx templates) don't have a straightforward strip step. The server can enforce the invariant unilaterally by refusing to consult the header without opt-in.
- **Require `TRUSTED_PROXY_CIDRS` to be non-empty (no `private_defaults` fallback)** — rejected because that changes the default-deployment story for many callers unrelated to CF (throttle keying, audit logs), risking regression on dev/CI and single-node deployments that legitimately have a private-IP proxy but no CF edge. The narrower two-tier trust keeps the current XFF behavior while closing the CF spoof.

## Root-cause trace
1. Symptom: `client_ip.rs:236-249` returns an attacker-supplied string when a private-IP proxy sits in front.
2. ← `client_ip.rs:245-248` unconditionally calls `cf_connecting_ip(headers)` when the peer is in `trusted`.
3. ← `client_ip.rs:166-172` — `private_defaults()` seeds `trusted` with every RFC1918 range, so any private-IP proxy peer satisfies the guard.
4. ← Contract at `client_ip.rs:228-234` treats "trusted proxy" as one tier, so any trusted peer implicitly authorises `CF-Connecting-IP` reads.
5. Origin: introduced by the issue #2789 remediation itself — the fix collapsed two distinct trust decisions ("we trust this hop to append XFF" and "we trust this hop to have set CF-Connecting-IP after stripping any client-supplied value") into one predicate.

## Test plan
- [ ] `backend/servers/api-server/src/client_ip.rs` — add `spoofed_cf_connecting_ip_from_private_peer_is_ignored` (fails on `dev`; passes after the fix).
- [ ] `backend/servers/api-server/src/client_ip.rs` — retarget `prefers_cf_connecting_ip_from_trusted_peer` to the opt-in `cf_edges` path via a `TRUSTED_CF_EDGE_CIDRS`-populated fixture (test currently passes on the buggy contract; it will fail once the code changes unless retargeted).
- [ ] `cargo test -p api-server client_ip`
- [ ] `cargo test -p api-server` (full crate — regression sweep; the client-ip helper is called from throttle + audit code paths).

## Out of scope
- Refactoring the rest of the `TrustedProxies` API (`parse`, `contains`, `from_env`) beyond adding the second trust list.
- Any change to the `X-Forwarded-For` handling.
- Any operator-facing docs outside the module docstring — deployment docs can follow up.
- Auditing every downstream throttle/audit-log call site that consumes `resolve_client_ip` — the return value shape is unchanged.

## After-merge
- Move this file to `plans/_archive/code-review-api-core-client-ip-cf-spoof.md`
- Mark the matching `backlog.json` row as `status: "done"`
