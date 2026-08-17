# security-share-ip-spoof-follow-up-2789

**Vector:** security
**Score:** 3
**Source:** Issue #2789
**Confidence:** high

## Hypothesis
`resolve_client_ip()` in `backend/servers/api-server/src/routes/documents/shares.rs:92` trusts the leftmost `X-Forwarded-For` hop and any `CF-Connecting-IP` header without verifying that the request actually arrived through a trusted proxy. Both headers are attacker-controllable, so the value that feeds two security-relevant consumers — the `share_access_key(token, ip)` brute-force throttle from PR #2773 and the `log_share_access` audit-log IP — can be forged. An attacker rotating a spoofed IP per request opens a fresh throttle bucket each time (defeating #2773's rate-limit) and can attribute any share access to an arbitrary source. Gate header trust on the socket peer being inside a configured trusted-proxy set; prefer `CF-Connecting-IP` when trusted; for `X-Forwarded-For` walk right-to-left skipping known proxies; fall back to `addr.ip()` when the peer is untrusted.

## Evidence
- Issue #2789 — post-merge review of PR #2784 (filed 2026-08-17T14:46:19Z), labels `follow-up`, `from-merged-review`
- `backend/servers/api-server/src/routes/documents/shares.rs:92` — `resolve_client_ip()` unconditionally reads `cf-connecting-ip` then leftmost `x-forwarded-for` before falling back to the socket peer
- `backend/servers/api-server/src/routes/documents/shares.rs:45` — `share_access_key(token, ip)` (throttle bucket key, PR #2773)
- `backend/servers/api-server/src/routes/documents/shares.rs:541,681` — both share-access handlers pass the resolved IP into `log_share_access(..., ip_address)`
- `backend/servers/api-server/src/routes/forms/submissions.rs:47` and `backend/servers/api-server/src/routes/admin/{memberships,impersonation,capabilities,agencies}.rs` — same naive leftmost-XFF extraction pattern, worth centralising in the same PR

## Files
- `backend/servers/api-server/src/routes/documents/shares.rs:92`
- `backend/servers/api-server/src/routes/forms/submissions.rs:47`
- `backend/servers/api-server/src/routes/admin/mod.rs:61`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):**
Mode: cloud-ok

## Repro steps
1. Boot api-server locally with default config (no trusted-proxy CIDR set); or add a unit test with `SocketAddr` set to a *non-trusted* IP (e.g. `203.0.113.99:12345`) as the connecting peer.
2. Issue two `POST /api/v1/shares/protected/{token}/access` requests to the same share token, each with headers `X-Forwarded-For: 198.51.100.<n>` where `<n>` differs between requests (no valid `CF-Connecting-IP`).
3. Assert: both requests hit distinct throttle buckets — `share_access_key(token, "198.51.100.1")` vs `share_access_key(token, "198.51.100.2")` — so the 10-attempt/15-min limiter from PR #2773 does not trip after either.
4. Expected (post-fix): both requests share the socket-peer bucket because the peer isn't a trusted proxy, so headers are ignored → second bucket miss, single throttle key, second request counts against the same limit.
5. Also assert: the row written by `log_share_access` records the socket peer, not the spoofed `X-Forwarded-For` value.

## Suggested approach
1. Introduce a `TrustedProxies` type in a shared module (candidate: `backend/servers/api-server/src/routes/admin/mod.rs` alongside the existing `client_ip` helper, or a new `backend/servers/api-server/src/common/client_ip.rs` — check where `common::` currently lives). Populate it from config/env (Cloudflare CIDRs + local reverse-proxy addresses).
2. Rewrite `resolve_client_ip(headers, addr, trusted)`: if `!trusted.contains(addr.ip())` return `addr.ip().to_string()`; else prefer `CF-Connecting-IP`, else walk `X-Forwarded-For` right-to-left skipping addresses inside `trusted`, else `addr.ip().to_string()`.
3. Update the two call-sites in `shares.rs:541,681` to pass the shared `TrustedProxies` (read from app state / config once at server start).
4. Migrate the leftmost-XFF pattern in `forms/submissions.rs:47` and each `admin/*.rs` `client_ip` call to the same helper (audit-log parity + throttle parity). Keep the diff scoped to call-sites already using naive XFF — don't touch handlers that use `ConnectInfo` directly.
5. Add rustdoc noting the trusted-proxy assumption; document that untrusted-peer requests are pinned to socket IP intentionally.
6. Add regression tests for the throttle-bypass and audit-poison scenarios (see *Test plan*).
7. Update the existing `resolve_client_ip_tests` module in `shares.rs:996` to construct a trusted-proxy set for the tests that expected header trust; add new tests for the untrusted-peer path.

## Alternatives considered
- **Take the rightmost `X-Forwarded-For` hop unconditionally** — rejected because deployments without any reverse proxy would resolve every client to the header's rightmost value (still spoofable); the trusted-proxy gate is what actually restores integrity.
- **Add a WAF / edge rule to strip client-supplied `X-Forwarded-For` / `CF-Connecting-IP`** — rejected as a substitute because it moves the trust decision out of the code path that consumes the value, and the fix is required for local/dev/non-Cloudflare deployments too. A WAF rule is complementary, not a replacement.

## Root-cause trace
1. Symptom: rotating `X-Forwarded-For: 198.51.100.<n>` per request keeps opening fresh throttle buckets — the #2773 rate-limit never fires; the same header value ends up in `log_share_access` rows regardless of the true source.
2. ← `share_access_key(token, ip)` at `shares.rs:45` uses the caller-supplied `ip` string verbatim as part of the bucket key
3. ← both share-access handlers at `shares.rs:541,681` compute that `ip` via `resolve_client_ip(&headers, addr)` at `shares.rs:92`, which trusts headers unconditionally
4. Origin: PR #2784 (merged 2026-08-17T12:33:16Z, `code-review-api-handlers-share-log-proxy-ip`) added `resolve_client_ip()` to fix audit-log proxy attribution but did not gate header trust on a trusted-proxy allowlist. PR #2773 (merged 2026-08-17T12:33Z) then wired the resolved IP into the brute-force throttle, propagating the trust gap into rate-limiting.

## Test plan
- [ ] Add `share_access_throttle_ignores_spoofed_xff_from_untrusted_peer` in `backend/servers/api-server/tests/suites/` — issue 11 requests to the same protected-share token with `X-Forwarded-For` rotating per request, connecting peer NOT in the trusted-proxy set; assert request 11 returns 429 (or the current lock-out response), matching the fixed-source-IP behaviour.
- [ ] Add `share_access_log_records_socket_peer_when_peer_untrusted` in the same suite — issue one access with a spoofed `X-Forwarded-For`, connecting peer NOT trusted; assert the `log_share_access` row's `ip_address` equals `addr.ip().to_string()`, not the header value.
- [ ] Extend existing `resolve_client_ip_tests` in `backend/servers/api-server/src/routes/documents/shares.rs:996` — one test each for: untrusted peer with headers set (headers ignored), trusted peer with only `CF-Connecting-IP` (used), trusted peer with multi-hop `X-Forwarded-For` (rightmost untrusted picked).
- [ ] `cd backend && cargo test -p api-server --test suites shares_ip_spoof` (or the equivalent path once test file lands).
- [ ] `cd backend && cargo test -p api-server routes::documents::shares::resolve_client_ip_tests` for the unit slice.

## Out of scope
- Rewriting all admin/audit call-sites to use `ConnectInfo` directly — only the ones already using the naive XFF helper should be migrated in this plan; the rest are unaffected.
- Introducing configuration UI or ops runbook for the trusted-proxy CIDR set beyond a documented env var / config-file key.
- Retroactive scrubbing of the existing `share_access_log` rows written with spoofable IPs — audit-history rewrites are a separate operational decision.
- Any change to the throttle *policy* itself (attempt count, window) — this plan only restores the intended integrity of the existing #2773 policy.

## After-merge
- Move this file to `plans/_archive/security-share-ip-spoof-follow-up-2789.md`
- Mark the matching `backlog.json` row as `status: "done"`
