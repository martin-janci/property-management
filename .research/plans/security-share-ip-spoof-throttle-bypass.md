# security-share-ip-spoof-throttle-bypass

**Vector:** security
**Score:** 2
**Source:** Issue #2789 | PR #2784 (introduced) | PR #2773 (throttle it defeats)
**Confidence:** high

## Hypothesis
`resolve_client_ip()` in `backend/servers/api-server/src/routes/documents/shares.rs`
trusts the leftmost `X-Forwarded-For` hop and a present `CF-Connecting-IP`
unconditionally, with no check that the request actually arrived through a
trusted proxy. Both feed (a) the shared-document audit trail and (b) the
`share_access_key(token, ip)` brute-force throttle bucket added by PR #2773.
Because both headers are attacker-controlled at any origin reachable outside
Cloudflare, rotating the spoofed IP per request yields a fresh bucket every
attempt, silently defeating the 10-attempts/15-min limiter and poisoning the
access log with forged source IPs. The smallest fix is to gate header trust on
the socket peer being inside a configured trusted-proxy allowlist, prefer
`CF-Connecting-IP`, and for `X-Forwarded-For` walk right-to-left to the
rightmost untrusted hop.

## Evidence
- `backend/servers/api-server/src/routes/documents/shares.rs:92-114` — `resolve_client_ip()` calls `header_ip(headers, "x-forwarded-for", leftmost_of_list=true)` with no peer/trust check.
- `backend/servers/api-server/src/routes/documents/shares.rs:45-66` — `share_access_key(token, ip)` and the throttle map that PR #2773 keys by IP.
- Issue #2789 — post-merge review of PR #2784 documenting the spoof→bypass chain (medium severity, `security` label).
- PR #2784 body — introduces `resolve_client_ip` for the access-log path, no trust boundary.
- PR #2773 body — brute-force throttle it silently nullifies.

## Files
- `backend/servers/api-server/src/routes/documents/shares.rs:92`
- `backend/servers/api-server/src/routes/documents/shares.rs:45`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [x] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):** cloud-ok — no browser or device required; unit tests over `resolve_client_ip` cover the whole surface.

Mode: cloud-ok

## Repro steps
1. Send `POST /api/v1/shared/<token>/access` with an obviously-wrong password and header `X-Forwarded-For: 1.2.3.4`. Repeat 20× varying the forged IP (`1.2.3.5`, `1.2.3.6`, …).
2. Expected: after the 10th attempt the throttle from PR #2773 returns `429`. Actual: every request is accepted (each IP forms its own bucket), the audit log records the forged IP verbatim, and the password oracle keeps answering.

## Suggested approach
1. Introduce a `TrustedProxies` config type in `backend/servers/api-server/src/state.rs` seeded from a new `TRUSTED_PROXY_CIDRS` env (comma-separated CIDRs; default empty = trust nothing, force socket-peer only). Cloudflare CIDR ranges are the intended production value.
2. Change `resolve_client_ip(headers, addr)` in `routes/documents/shares.rs:92` to `resolve_client_ip(headers, addr, trusted: &TrustedProxies)`; short-circuit to `addr.ip().to_string()` when `!trusted.contains(addr.ip())`.
3. Keep the `cf-connecting-ip` branch (only reached when the peer is trusted). For `x-forwarded-for`, replace the leftmost-hop selector with a right-to-left walk that skips known-trusted addresses and returns the first untrusted hop.
4. Thread `state.trusted_proxies` (or an `Extension<TrustedProxies>`) into both `access_shared_document` and `access_protected_share` call sites; update `share_access_key` callers to use the new value unchanged.
5. Audit `backend/servers/api-server/src/routes/forms/submissions.rs` for the same leftmost-XFF anti-pattern (issue #2789 flags it as a co-victim) and refactor to share the extractor; if refactor is bigger than 30 LOC, file a follow-up and touch only the shares.rs path in this PR.
6. Add unit tests in `backend/servers/api-server/tests/suites/documents_share_ip_trust_tests.rs` covering: (a) untrusted peer → header ignored, (b) trusted peer + `CF-Connecting-IP` → CF wins, (c) trusted peer + XFF chain → rightmost-untrusted wins, (d) trusted peer + forged leftmost + real rightmost → real IP wins.
7. Regression: extend `documents_share_password_throttle_tests.rs` (or add a companion) that hammers the endpoint from a fixed socket peer with rotating XFF and asserts the throttle still trips at attempt 11.

## Alternatives considered
- **Trust `CF-Connecting-IP` unconditionally when present** — rejected because at any origin reachable outside Cloudflare (dev deploys, misconfigured firewalls, or direct-IP scans) the header is client-supplied and no more trustworthy than the peer address; this would fix the audit path but leave the throttle bypass wide open in exactly the environments most likely to be attacked.
- **Cap throttle bucket count per token instead of tightening IP resolution** — rejected because it treats the symptom (bucket explosion) rather than the cause (bogus keys), forces a token-wide global limiter that will legitimately trip for busy shared docs on office NATs, and leaves the audit-log poisoning untouched.

## Root-cause trace
1. Symptom: brute-force throttle from PR #2773 silently fails; audit log rows contain forged IPs.
2. ← `share_access_key(token, ip)` at `routes/documents/shares.rs:45` derives its bucket key from whatever `resolve_client_ip` returned.
3. ← `resolve_client_ip` at `routes/documents/shares.rs:92-114` picks `header_ip("x-forwarded-for", leftmost=true)` before falling back to the peer, without any trust check on the peer.
4. Origin: PR #2784 (merged 2026-08-17T12:33:16Z) — introduced `resolve_client_ip` for audit unwind; PR #2773 (merged same window) predates it and never anticipated attacker-controlled key input.

## Test plan
- [ ] `backend/servers/api-server/tests/suites/documents_share_ip_trust_tests.rs` — new suite: untrusted peer + spoofed headers yields the peer IP; trusted peer + spoofed XFF still resolves to the true rightmost hop.
- [ ] Regression against the throttle: 11 forged-XFF requests from one socket peer trip the 429 that PR #2773 promised.
- [ ] Command: `cargo test -p api-server documents_share_ip_trust_tests` and `cargo test -p api-server documents_share_password_throttle`.

## Out of scope
- Rewriting `routes/forms/submissions.rs` — call out that it has the same pattern, but land only the shares.rs fix + shared extractor stub in this PR; a follow-up plan covers the submissions path.
- Rolling out a per-request rate-limiter middleware in front of the whole app — that is a separate architectural change.
- Publishing Cloudflare CIDR discovery / auto-refresh — env-configured static list is enough for the fix.

## After-merge
- Move this file to `plans/_archive/security-share-ip-spoof-throttle-bypass.md`
- Mark the matching `backlog.json` row as `status: "done"`
