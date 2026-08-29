# code-review-api-core-client-ip-cf-spoof

**Vector:** bug
**Score:** 3
**Source:** dispatcher Tier-1d api-core review 2026-08-29 (signals/2026-08-29-api-core-tier1d-client-ip.json)
**Confidence:** medium

## Hypothesis
`resolve_client_ip` consults `CF-Connecting-IP` whenever the socket peer is in the default trusted set (loopback + RFC1918/RFC4193 — `client_ip.rs:167-173`). That is the exact nginx/container-ingress deployment the module's own docs call the common case, and a plain reverse proxy does not strip an inbound `CF-Connecting-IP` header. So on any non-Cloudflare edge that still runs with the private-range default trust set, any client can set `CF-Connecting-IP: <forged>` and have it returned as the client IP — reintroducing the #2789 spoof class (audit-log poisoning + per-request throttle-bucket rotation) that the rightmost-untrusted-XFF logic (`client_ip.rs:212-223`) otherwise closes. The fix is to gate the `CF-Connecting-IP` consult behind an explicit Cloudflare edge signal (a dedicated config flag or CF IP-range check) instead of the generic private-range default trust set; when the edge is a generic reverse proxy, fall through to the existing rightmost-untrusted-XFF resolution.

## Evidence
- `backend/servers/api-server/src/client_ip.rs:236-252` — `resolve_client_ip` consults `cf_connecting_ip(headers)` whenever `peer_in_trusted_set` is true, with no Cloudflare-specific gate.
- `backend/servers/api-server/src/client_ip.rs:167-173` — `private_defaults()` = loopback + RFC1918/RFC4193; this is the same trust profile the module doc-comment identifies as the common nginx/container-ingress deployment.
- `backend/servers/api-server/src/client_ip.rs:198-201` — `cf_connecting_ip` reads the raw header; there is no assertion that the trusted edge is actually Cloudflare.
- `backend/servers/api-server/src/client_ip.rs:212-223` — the rightmost-untrusted-XFF resolver is the intended safe path for a non-Cloudflare private-range proxy, but the `CF-Connecting-IP` short-circuit at :245-248 bypasses it.
- Historical: PR #2789 closed the identical spoof class in the XFF path; this issue is a residual attack surface in the sibling `CF-Connecting-IP` path against the same threat model (audit-log poisoning + per-request throttle-bucket rotation).

## Files
- `backend/servers/api-server/src/client_ip.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (security regression class)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):** no C4, no C5 → `Mode: cloud-ok`

Mode: cloud-ok

## Repro steps
1. Build the existing test suite: `cargo test -p api-server --lib client_ip::tests` — the `prefers_cf_connecting_ip_from_trusted_peer` case at `client_ip.rs:318-324` passes because it is written from the operator viewpoint (a trusted edge legitimately setting `CF-Connecting-IP`).
2. Add a regression test that models the attacker viewpoint: peer is a generic reverse-proxy in the private-range default trust set (e.g. `10.0.0.5:8080`), no explicit Cloudflare gate is configured, and the request carries `CF-Connecting-IP: 203.0.113.99` alongside an `X-Forwarded-For` that would resolve to the true edge-facing client (say `198.51.100.7`).
3. Expected after fix: `resolve_client_ip` returns `198.51.100.7` (the untrusted-XFF resolution) — the forged `CF-Connecting-IP` is ignored because no Cloudflare gate opted in.
4. Actual today: `resolve_client_ip` returns `203.0.113.99` (the forged value), silently succeeding the spoof.

## Suggested approach
1. Introduce an explicit "trust CF-Connecting-IP" opt-in on the trust config. Simplest shape: add a `trust_cf_connecting_ip: bool` (or `cf_edge_ranges: Vec<IpNet>`) field to the `TrustedProxies` struct; default `false` so the private-range default set never activates it.
2. In `resolve_client_ip`, gate the `cf_connecting_ip(headers)` short-circuit on the new opt-in: consult the header only when the peer is trusted AND CF trust is explicitly enabled (or the peer address is inside `cf_edge_ranges`).
3. When the CF gate is off, fall through to the existing `XForwardedFor` untrusted-hop resolution — the safe behavior the tests at `:325-393` already validate.
4. Wire the new option through the same config seam that produces `TrustedProxies::private_defaults()`; in `main.rs`/`config.rs` add the env var (e.g. `TRUSTED_PROXY_TRUST_CF_HEADER=true`) with default `false` so existing non-Cloudflare deployments become safe without operator action.
5. Add the regression test in step 2 above under `client_ip::tests`, and rename/adjust `prefers_cf_connecting_ip_from_trusted_peer` to explicitly enable the new opt-in (so its intent — "a legitimate Cloudflare edge is configured" — is spelled out).
6. Update the module doc-comment (`client_ip.rs:7-15`) to state that `CF-Connecting-IP` requires an explicit Cloudflare opt-in; the private-range default trust set alone is not sufficient.

## Alternatives considered
- **Always ignore `CF-Connecting-IP` when the peer is only in the default private-range set** (i.e. hard-code the same gate rather than making it configurable) — rejected because operators that *do* front the api-server with Cloudflare via a private-range hop (a common docker-compose / k8s ingress + Cloudflare Tunnel setup) would silently drop the correct edge IP; an explicit opt-in is a one-line config change that preserves both deployments.
- **Strip `CF-Connecting-IP` upstream in nginx** — rejected as the primary fix because it pushes safety onto the operator's reverse-proxy config (easy to forget, invisible to the api-server), whereas the Rust-side gate is the guarantee that never regresses regardless of edge config.

## Root-cause trace
1. Symptom: on a private-range reverse proxy without a Cloudflare gate, a forged `CF-Connecting-IP` becomes the returned client IP (spoofed audit-log entries, rotated throttle buckets).
2. ← `resolve_client_ip` at `backend/servers/api-server/src/client_ip.rs:245-248` unconditionally prefers `cf_connecting_ip(headers)` whenever `peer_in_trusted_set` is true.
3. ← `TrustedProxies::private_defaults()` at `client_ip.rs:167-173` uses loopback + RFC1918/RFC4193 — the profile the doc-comment names as the common non-Cloudflare deployment; there is no signal to `resolve_client_ip` distinguishing a Cloudflare edge from a generic private-range proxy.
4. Origin: the `CF-Connecting-IP` short-circuit predates PR #2789's XFF hardening; #2789 fixed the parallel spoof class on the XFF path but left the CF path on the pre-#2789 trust-on-peer model.

## Test plan
- [ ] `backend/servers/api-server/src/client_ip.rs` — new `#[test]` `rejects_forged_cf_connecting_ip_without_explicit_cf_gate`: peer in `private_defaults`, `CF-Connecting-IP: 203.0.113.99`, `X-Forwarded-For: 198.51.100.7, 10.0.0.5` → `resolve_client_ip` returns `198.51.100.7` (untrusted-XFF), not `203.0.113.99`.
- [ ] Regression (existing): `prefers_cf_connecting_ip_from_trusted_peer` still passes with the new opt-in explicitly enabled, proving the legitimate Cloudflare edge behavior is preserved.
- [ ] Regression: no header at all with peer in the default trust set still returns the peer address (existing `no_forwarded_headers_returns_peer_address` behavior preserved).
- [ ] Command: `cargo test -p api-server --lib client_ip::tests`

## Out of scope
- Rewriting the entire trust-config schema (only the one field / env var this fix requires).
- Auto-detecting whether the peer is actually a Cloudflare edge from live CF IP ranges (a Cloudflare-published range file would be nice but is deployment-config, not a compile-time asset).
- Reworking `cf_connecting_ip` parsing itself (single-address extraction is fine; the defect is upstream trust, not parsing).
- The parallel `X-Real-IP` handling path (not in this signal's evidence; separate finding if warranted).

## After-merge
- Move this file to `plans/_archive/code-review-api-core-client-ip-cf-spoof.md`
- Mark the matching `backlog.json` row as `status: "done"`
