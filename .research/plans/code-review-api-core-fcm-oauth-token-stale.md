# code-review-api-core-fcm-oauth-token-stale

**Vector:** bug
**Score:** 3
**Source:** Tier-1d dispatcher code review 2026-08-28 (`api-core` services segment); `.research/signals/2026-08-28-api-core-tier1d-services.json`
**Confidence:** medium

## Hypothesis
`FcmConfig.oauth_token` in `backend/servers/api-server/src/services/push_fanout.rs:217/233` is populated once from `std::env::var("FCM_OAUTH_TOKEN")` at startup and reused as the FCM HTTP v1 `Bearer` credential for every send (`push_fanout.rs:330-341`). Google FCM OAuth2 tokens minted from a service account expire in 3600 seconds by design, so after one hour every push starts returning HTTP 401 and the whole fanout goes silent — for the entire process lifetime, until the api-server is restarted. The module doc at `push_fanout.rs:36` already flags the shape (`"(read once at startup)"`) but no send-path refresh exists. Fix: refresh the bearer per send from a token source that returns the current value (either an env re-read on every send, keyed off a startup flag; or a small in-memory cache with expiry), and let a 401 from FCM force a refresh + one retry.

## Evidence
- `backend/servers/api-server/src/services/push_fanout.rs:36` — module doc table row: `` FCM_OAUTH_TOKEN … OAuth2 bearer token for FCM HTTP v1 (read once at startup) ``.
- `backend/servers/api-server/src/services/push_fanout.rs:217` — struct field `pub oauth_token: Option<String>` — a plain `String`, no expiry, no source-of-truth.
- `backend/servers/api-server/src/services/push_fanout.rs:230-236` — `FcmConfig::from_env` populates the field once via `std::env::var("FCM_OAUTH_TOKEN").ok()`; there is no other write path in the file.
- `backend/servers/api-server/src/services/push_fanout.rs:330-341` — send path clones the cached token and passes it verbatim to `reqwest::RequestBuilder::bearer_auth`; the caller cannot indicate freshness and no 401 handling forces a refresh.
- Adjacent legacy fallback path (`send_fcm_legacy`) works around this today by preferring `FCM_SERVER_KEY` (a long-lived key) — the v1 path is the one that silently degrades.

## Files
- `backend/servers/api-server/src/services/push_fanout.rs:36`
- `backend/servers/api-server/src/services/push_fanout.rs:217`
- `backend/servers/api-server/src/services/push_fanout.rs:230`
- `backend/servers/api-server/src/services/push_fanout.rs:330`

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
1. Configure the api-server with `FCM_PROJECT_ID` and `FCM_OAUTH_TOKEN` set (v1 path selected — bypasses the legacy `FCM_SERVER_KEY` branch).
2. Trigger a push (any notification-emitting flow that fans out via FCM). Point the `fcm_base_url` at a `wiremock` server whose `/v1/projects/{id}/messages:send` responds `200 { "name": "…" }` while the cached bearer is `"stale"`. Assert: send succeeds.
3. Simulate the ambient token rotation: replace the process env `FCM_OAUTH_TOKEN` with a *new* value; leave `wiremock` configured to respond `401 { "error": { "status": "UNAUTHENTICATED" } }` when the request presents the OLD bearer, and `200` when it presents the NEW bearer.
4. Trigger another push. Expected: the send-path either re-reads the env (or its source-of-truth) and presents the NEW bearer, or receives the 401 and refreshes-then-retries with the NEW bearer, ending in `200`. Actual on `dev`: send presents the OLD (cached) bearer, receives `401`, records a send failure, and every future send behaves identically — the fanout is dead until restart.

## Suggested approach
1. Replace the `oauth_token: Option<String>` field (`push_fanout.rs:217`) with a `token_source: TokenSource` enum: `Static(Arc<String>)` for tests / legacy callers that pass a raw token, and `EnvVar("FCM_OAUTH_TOKEN")` for the from-env path (`push_fanout.rs:230-236`).
2. Update `FcmConfig::from_env` (`push_fanout.rs:230-236`): if `FCM_OAUTH_TOKEN` is present at startup, keep `token_source = EnvVar("FCM_OAUTH_TOKEN")` regardless of the current value (so subsequent env mutations are honoured); if absent, leave the source unset and let the send path fall through to `send_fcm_legacy` as today.
3. Add a private `current_bearer(&self) -> Option<String>` helper on `FcmConfig` (or on the adapter) that resolves `token_source` on each call — for `EnvVar`, do `std::env::var(<name>).ok()`; for `Static`, clone the `Arc`. Update the module doc at `push_fanout.rs:36` to remove `"(read once at startup)"` and describe the new lookup shape.
4. In the send path (`push_fanout.rs:328-345`) call `current_bearer()` inline and thread it into `.bearer_auth(&bearer)`; if `current_bearer()` returns `None` while an `EnvVar` source is configured, fall through to `send_fcm_legacy` (existing behaviour when no token).
5. Add a single 401 retry: if the FCM response status is `UNAUTHORIZED`, call `current_bearer()` again and reissue *once* with the fresh value before recording send failure. Cap at one retry — no exponential backoff, no loop.
6. Do NOT introduce a full google-cloud-auth SDK dependency; the module has an explicit "no GCP SDK" line already (`push_fanout.rs:317-323`). The env-var source is enough to interoperate with an out-of-process refresher (cron / sidecar) that rewrites `FCM_OAUTH_TOKEN` before it expires.
7. `cargo test -p api-server push_fanout` — the module has an existing test harness with `wiremock`; wire the two new tests (`fcm_v1_uses_current_env_bearer_per_send` and `fcm_v1_retries_once_after_401`) into it.

## Alternatives considered
- **Bake a full google-cloud-auth SDK client + service-account key file into `push_fanout`** — rejected because `push_fanout.rs:317-323` explicitly rules it out ("re-use `FCM_SERVER_KEY` … to keep the dependency footprint minimal (no GCP SDK)"). The env-var source keeps the current shape and defers credential minting to an out-of-process refresher.
- **Cache the token with a 55-minute TTL and refresh via an internal HTTP call to a token-minter service** — rejected because it invents a mint endpoint the codebase doesn't have and the operator hasn't asked for. The env-var-per-send path solves the observed hang with zero new infra.

## Root-cause trace
1. Symptom: every FCM v1 send returns `401 UNAUTHENTICATED` more than an hour after startup; the fanout appears "silent" to callers.
2. ← `push_fanout.rs:335` — `.bearer_auth(&bearer)` on every send call, where `bearer` was cloned from `self.fcm_config.oauth_token` (`push_fanout.rs:330`).
3. ← `push_fanout.rs:217/233` — `FcmConfig.oauth_token: Option<String>` is a plain string, populated once at startup by `FcmConfig::from_env` and never refreshed.
4. ← Google FCM v1's `messages:send` uses OAuth2 Bearer tokens that expire in 3600 s; any long-running process caching one value silently degrades after one hour.
5. Origin: the FCM v1 path was added after the legacy `FCM_SERVER_KEY` path (which uses a long-lived key), and the doc row `"read once at startup"` at `push_fanout.rs:36` cements the shape rather than flagging the mismatch. Latent since the v1 path landed.

## Test plan
- [ ] `backend/servers/api-server/src/services/push_fanout.rs` — add unit test `fcm_v1_uses_current_env_bearer_per_send` using `wiremock`: pre-set `FCM_OAUTH_TOKEN=old`, first send matches only when the auth header carries the old token; then update `FCM_OAUTH_TOKEN=new`, second send matches only when it carries the new token. Both sends must succeed. (Fails on `dev`.)
- [ ] `backend/servers/api-server/src/services/push_fanout.rs` — add unit test `fcm_v1_retries_once_after_401`: `wiremock` returns `401` when the request presents the old bearer and `200` when it presents the new one; before the send, mutate the env from old to new. Assert: exactly two HTTP requests were made and the second returned `200`. (Fails on `dev`.)
- [ ] `cargo test -p api-server push_fanout`
- [ ] `cargo test -p api-server` (full crate — the push adapter is called from notification pipeline / scheduler / route handlers; verify no other consumer regressed).

## Out of scope
- The legacy `FCM_SERVER_KEY` path (`send_fcm_legacy`) — its long-lived key does not have this expiry class and its behavior is not changing.
- Adopting `google-cloud-auth` or any GCP SDK — explicitly rejected in Alternatives.
- Redesigning `NotificationPipeline` back-pressure or dead-letter handling around persistent 401s — this plan restores the happy path only.

## After-merge
- Move this file to `plans/_archive/code-review-api-core-fcm-oauth-token-stale.md`
- Mark the matching `backlog.json` row as `status: "done"`
