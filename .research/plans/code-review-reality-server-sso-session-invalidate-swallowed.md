# code-review-reality-server-sso-session-invalidate-swallowed

**Vector:** security
**Score:** 3
**Source:** rotating-expert-review reality-server 2026-08-01 (backlog id `code-review-reality-server-sso-session-invalidate-swallowed`)
**Confidence:** high

## Hypothesis
`sync_session` in reality-server treats "drop the portal session when the upstream PM token is inactive" as a critical security control, but the implementation discards the `Result` of `invalidate_session` with `let _ = …` at `sso.rs:1043` and then unconditionally logs `"Invalidated portal session due to inactive PM token"` and returns `401 pm_session_expired` (`sso.rs:1047-1056`). If the Redis/DB layer errors — even transiently — the portal session cookie remains valid on the client, but the server tells the client "you've been logged out". The client-side flow then obeys the 401 and re-logs in, so the stale cookie could still be replayed from any other browser/session that has it (or by an attacker who captured it), because the server never actually revoked the session record. The smallest fix is to propagate the invalidate error: log at `error!`, and return `500 sso_invalidate_failed` when the revocation fails so the caller does NOT get a confident "expired" response for a session that is still live.

## Evidence
- `backend/servers/reality-server/src/routes/sso.rs:1041-1057` — `if !token_info.active { … let _ = state.session_service.invalidate_session(portal_session).await; tracing::info!("Invalidated portal session due to inactive PM token"); return Err((StatusCode::UNAUTHORIZED, Json(SsoError::new("pm_session_expired", …))));` — the log is unconditional, the error result is discarded.
- Neighboring code in the same file consistently propagates errors from `session_service` calls (search for `session_service.` in `sso.rs`); the `sync_session` inactive-token branch is the outlier.
- Sibling routes/helpers propagate service errors via `.map_err(|e| …)` chains; there is no explicit "swallow on best-effort" contract for `invalidate_session` — the method returns `Result<(), _>` and callers elsewhere handle the error.
- Impact class: broken security control that self-reports success. Not a data-leak per-request, but the guarantee this branch is supposed to provide ("portal session is dead when PM session dies") does not hold under any Redis/DB partial failure. That's the failure mode where you want loud diagnostics, not a warm reassuring log line.

## Files
- `backend/servers/reality-server/src/routes/sso.rs:1043`
- `backend/servers/reality-server/src/routes/sso.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (trace the invalidate_session return path and confirm no caller depends on the swallow behaviour)
- [x] C2 — Seed data (need a portal_session in Redis + a PM token whose introspection returns `active=false`)
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Seed a portal session (via `sync_session` happy path or direct Redis SET on the session key) with `portal_session = "test-cookie"`.
2. Stub `introspect_pm_token` (or set the PM token to one the introspection endpoint returns `active=false` for).
3. Force `session_service.invalidate_session` to error — e.g. drop the Redis connection between step 1 and the call, or point `REDIS_URL` at a closed port for that one call.
4. Call `POST /api/v1/sso/sync-session` with `{ "pm_token": "<inactive>", "portal_session": "test-cookie" }`.
5. Expected: `500 sso_invalidate_failed` (or equivalent), portal session key still present in Redis, log line at `error!` level naming the invalidate failure.
6. Actual (today): `401 pm_session_expired`, portal session key still present in Redis, log line at `info!` level asserting an invalidation that did not happen.

## Suggested approach
1. Change the `if !token_info.active { … }` branch at `sso.rs:1041` from `let _ = state.session_service.invalidate_session(portal_session).await;` to a matched call:
   ```rust
   if let Err(e) = state.session_service.invalidate_session(portal_session).await {
       tracing::error!(error = %e, "failed to invalidate portal session for inactive PM token");
       return Err((
           StatusCode::INTERNAL_SERVER_ERROR,
           Json(SsoError::new("sso_invalidate_failed", "portal session revocation failed")),
       ));
   }
   tracing::info!("Invalidated portal session due to inactive PM token");
   ```
2. Only reach the `pm_session_expired` return after the invalidate succeeded — the log line then reflects reality.
3. Optionally: also guard the `request.portal_session.is_none()` case explicitly (today it silently skips the revoke — that's fine when there is no session to revoke, but worth a `debug!` for observability).
4. Grep the rest of `sso.rs` for other `let _ = …` calls on `session_service` / repos and audit each one — same class of bug likely lives elsewhere.
5. Author the regression test described in *Test plan* against the failing repro above.
6. Run `cargo test -p reality-server sso::sync_session` locally to confirm the new test fails on `dev` and passes after the fix.
7. `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all` before pushing.

## Alternatives considered
- **Fire-and-forget with retry queue** — rejected because it introduces a new durability system (background retry) to fix a call that today is on the request path and should just fail loudly. The invalidate failure is already ~zero-latency to observe; a retry queue is over-engineered for a call that hits Redis.
- **Downgrade to `warn!` log without changing the response** — rejected because the security guarantee this branch offers is "portal is dead"; returning `401 pm_session_expired` when the session is still live is a lie by the security layer. The caller must know so it can either retry or fail closed. A warn-only fix leaves the false-positive `401` in place.

## Root-cause trace
1. Symptom: portal session key remains in Redis after `POST /sso/sync-session` returned `401 pm_session_expired`; log line "Invalidated portal session due to inactive PM token" appears despite no state change.
2. ← `sso.rs:1043` — `let _ = state.session_service.invalidate_session(portal_session).await;` drops the `Result`.
3. ← `sso.rs:1047-1056` — log + `401 pm_session_expired` are unconditional after the discarded call.
4. Origin: the invalidate helper was likely added in a batch with the token-introspection path; the `let _ =` pattern is idiomatic for "best-effort" ops but this branch is not best-effort — it is the security guarantee of the endpoint. See `git log -p backend/servers/reality-server/src/routes/sso.rs` around the introduction of `invalidate_session` in this file.

## Test plan
- [ ] `backend/servers/reality-server/tests/sso_sync_session_tests.rs` — new test `sync_session_returns_500_when_invalidate_fails`: stub `session_service.invalidate_session` to return `Err(_)`, drive `sync_session` with `active=false`, assert `500` + `sso_invalidate_failed` body + session key still present.
- [ ] Positive-path regression: `sync_session_returns_401_and_deletes_session_when_pm_inactive` — invalidate succeeds → assert `401 pm_session_expired` + session key absent.
- [ ] Command: `cargo test -p reality-server --test sso_sync_session_tests`.

## Out of scope
- Any other `let _ = …` swallow patterns in reality-server outside `sso.rs` — track those as separate signals if found (name them in the PR description but do not fix in this PR).
- The `sso.rs` cookie-refresh path (`sso.rs:1069+`) — different call, different security posture.
- Rate-limiting or lockout logic on `sync_session` failures.

## After-merge
- Move this file to `plans/_archive/code-review-reality-server-sso-session-invalidate-swallowed.md`
- Mark the matching `backlog.json` row as `status: "done"`
