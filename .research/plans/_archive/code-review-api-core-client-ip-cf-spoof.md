# code-review-api-core-client-ip-cf-spoof

**Vector:** security
**Score:** 3
**Source:** dispatcher Tier-1d rotating-expert-review 2026-08-29 (api-core client-ip) — signal `code-review-api-core-client-ip-cf-spoof`
**Confidence:** medium

## Hypothesis
`resolve_client_ip` treats `CF-Connecting-IP` as authoritative whenever the socket peer is in `TrustedProxies`, but the default trusted set is loopback + RFC1918/RFC4193 — the exact private-range nginx / container-ingress topology the module docs call the common case. A plain reverse proxy does NOT strip inbound `CF-Connecting-IP`, so any client can send `CF-Connecting-IP: <forged>` and, because the peer (nginx, private range) is trusted, the forged value becomes the resolved client IP. That reintroduces the `#2789` audit-log-poisoning + per-request throttle-bucket-rotation class the rightmost-untrusted-XFF path (`client_ip.rs:212-223`) otherwise closes. Fix: only consult `CF-Connecting-IP` when the trusted edge is explicitly Cloudflare (a dedicated flag or a Cloudflare-CIDR gate), otherwise fall through to XFF.

## Evidence
- `backend/servers/api-server/src/client_ip.rs:236-249` — `resolve_client_ip` calls `cf_connecting_ip(headers).or_else(|| rightmost_untrusted_xff(...))` on any `trusted.contains(peer)`; there is no Cloudflare-specific gate.
- `backend/servers/api-server/src/client_ip.rs:169-173` — `private_defaults()` is `127.0.0.0/8, ::1/128, 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16, fc00::/7, fe80::/10` — a generic reverse-proxy topology.
- `backend/servers/api-server/src/client_ip.rs:198-200` — `cf_connecting_ip` just reads the header verbatim; no assertion the edge is actually Cloudflare.
- `backend/servers/api-server/src/client_ip.rs:150-158` — module docs warn about the *public-edge* case (Cloudflare / cloud LB), but the private-range default case (nginx / sidecar) still trusts `CF-Connecting-IP` unconditionally.
- Prior context: `client_ip.rs:1-14` frames the whole module as fixing `#2789`; the CF-Connecting-IP path is the residual hole.

## Files
- `backend/servers/api-server/src/client_ip.rs`
- `backend/servers/api-server/src/state.rs:485`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug/security)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Deploy the api-server behind a generic private-range reverse proxy (e.g. nginx sidecar at `10.0.0.5`) with `TRUSTED_PROXY_CIDRS` unset — the shipped default.
2. Send `curl -H 'CF-Connecting-IP: 6.6.6.6' https://api.internal/health` from a client that reaches the proxy.
3. Observe the audit log / rate-limiter key attribution: **actual** = `6.6.6.6` (forged); **expected** = the real socket-peer (the client's real IP, since the deployment is not Cloudflare and `CF-Connecting-IP` should be ignored).
4. Equivalent unit repro (no live deploy): call `resolve_client_ip(headers_with_cf_connecting_ip("6.6.6.6"), addr=10.0.0.5:443, trusted=private_defaults())` — asserts return `!= "6.6.6.6"` and today returns `"6.6.6.6"`.

## Suggested approach
1. Introduce a `TrustCfConnectingIp` boolean on `TrustedProxies` (or a sibling struct), defaulting **off**. Populate from env: `TRUST_CF_CONNECTING_IP=1` explicitly opts in, OR the presence of a dedicated `CF_TRUSTED_CIDRS` env whose CIDRs match Cloudflare's published ranges (whichever the operator prefers — start with the boolean flag as the minimal change).
2. In `resolve_client_ip` (`client_ip.rs:245-247`), gate the `cf_connecting_ip(headers).or_else(...)` on that flag — when the flag is off, skip straight to `rightmost_untrusted_xff(...)`. This keeps the current behavior for CF-fronted deployments that explicitly opt in and closes the spoof for the default private-range case.
3. Update the module docs (`client_ip.rs:1-14`, `:150-158`, `:225-234`) so the contract now reads: *"When the trusted edge is Cloudflare, set `TRUST_CF_CONNECTING_IP=1` in addition to `TRUSTED_PROXY_CIDRS`; otherwise `CF-Connecting-IP` is ignored."*
4. Wire the env into `state.rs` alongside `TRUSTED_PROXY_CIDRS` (near `:485` and `:804`); pass the `TrustCfConnectingIp` state through the same plumbing as `TrustedProxies`.
5. Add a unit regression test in `client_ip.rs::tests` that asserts a forged `CF-Connecting-IP` from a private-range peer with the flag OFF is NOT returned as the client IP (and the existing `prefers_cf_connecting_ip_from_trusted_peer` test at `:318` is updated to construct trust with the flag ON to keep asserting the CF-fronted path).
6. Grep for any test or fixture that constructs a bare `TrustedProxies` and expects CF-Connecting-IP to be honored — they must switch to the new flag-on constructor.
7. If any live env already relies on the current behavior (nginx + Cloudflare via a private-range sidecar), operators must set the new flag; call out in the PR description.

## Alternatives considered
- **Only consult CF-Connecting-IP when the peer is in a dedicated CF-only CIDR set** — rejected as the sole mechanism because operators who already gate CF via `TRUSTED_PROXY_CIDRS` would have to duplicate that list; the explicit boolean flag is a lower-friction opt-in and can be composed with the CIDR set as a later refinement.
- **Always drop CF-Connecting-IP and rely solely on XFF** — rejected because Cloudflare's `X-Forwarded-For` can carry multiple hops and semantically Cloudflare treats `CF-Connecting-IP` as authoritative; dropping it breaks CF-fronted deployments' attribution.

## Root-cause trace
1. Symptom: forged `CF-Connecting-IP: 6.6.6.6` from an untrusted client through a trusted private-range proxy is returned as the client IP → audit-log poisoning + per-request throttle-bucket rotation (same class as `#2789`).
2. ← `resolve_client_ip` at `client_ip.rs:245` calls `cf_connecting_ip(headers)` before `rightmost_untrusted_xff` with no gate on which edge the trusted peer represents.
3. ← `cf_connecting_ip` at `client_ip.rs:198-200` reads the header verbatim; the trust check upstream is peer-CIDR-only, not edge-identity-aware.
4. ← `TrustedProxies::private_defaults` at `client_ip.rs:169-173` widens the default trusted set to all RFC1918 + IPv6-ULA + link-local, so the shipped default treats any nginx/container-network sidecar as a CF-Connecting-IP oracle.
5. Origin: the `#2789` fix (module comment `client_ip.rs:1-14`) hardened `X-Forwarded-For` via rightmost-untrusted-XFF but preserved the CF-Connecting-IP preference on all trusted peers rather than restricting it to a Cloudflare-identified edge.

## Test plan
- [ ] `backend/servers/api-server/src/client_ip.rs::tests::forged_cf_connecting_ip_ignored_when_flag_off` — new; asserts private-range peer + forged `CF-Connecting-IP` + flag off returns the socket peer (or falls through to XFF), NOT the forged header.
- [ ] `backend/servers/api-server/src/client_ip.rs::tests::prefers_cf_connecting_ip_from_trusted_peer` (existing, `:318`) — updated to construct `TrustedProxies` with the CF-trust flag ON; still passes to prove the CF-fronted path.
- [ ] Regression command: `cargo test -p api-server --lib client_ip`

## Out of scope
- Reworking the whole `TrustedProxies` abstraction (e.g. splitting into `PrivateProxies` + `CloudflareProxies`) — out of scope for this PR; the flag is the minimal change that closes the spoof.
- Adding a CF-CIDR autoloader (fetching `https://www.cloudflare.com/ips-v4`) — a follow-up if the operator asks for it.
- Auditing every downstream consumer of the resolved IP (rate limiter, audit log, throttle keying) — the fix at `resolve_client_ip` is the single seam; consumers automatically benefit.

## After-merge
- Move this file to `plans/_archive/code-review-api-core-client-ip-cf-spoof.md`
- Mark `backlog.json` row `code-review-api-core-client-ip-cf-spoof` as `status: "done"`
