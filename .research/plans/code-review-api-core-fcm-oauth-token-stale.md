# code-review-api-core-fcm-oauth-token-stale

**Vector:** bug
**Score:** 3
**Source:** dispatcher Tier-1d review 2026-08-28 (`.research/signals/2026-08-28-api-core-tier1d-services.json`) — routine 2026-08-29 corrective revival (dispatcher-generated finding not previously actualized into backlog)
**Confidence:** medium

## Hypothesis
`FcmHttpAdapter` captures `FCM_OAUTH_TOKEN` once at startup (`FcmConfig::from_env` at `push_fanout.rs:229`) and re-uses that static string as the bearer for every FCM HTTP v1 send (`push_fanout.rs:330-341`). FCM v1 OAuth2 access tokens minted from a Google service account are short-lived (~1 h TTL), so after roughly one hour of uptime every Android push send returns 401 UNAUTHENTICATED. The 401 branch (`push_fanout.rs:362-376`) only classifies FCM statuses `NOT_REGISTERED` / `UNREGISTERED` / `INVALID_REGISTRATION` as `token_expired`, so a credential-level failure is (correctly) not treated as a device-token expiry — but nothing else detects it either. Net effect: silent, indefinite Android push outage after ~1 h, indistinguishable from ordinary per-message rejections. Fix: detect HTTP 401/403 or `err_status == "UNAUTHENTICATED"` as a *credential* failure that either triggers a refresh or emits an operational alert, so the outage is either self-healing or loud.

## Evidence
- `backend/servers/api-server/src/services/push_fanout.rs:233` — `oauth_token: std::env::var("FCM_OAUTH_TOKEN").ok()` captured once in `FcmConfig::from_env`; module doc at `push_fanout.rs:36` labels this "(read once at startup)".
- `backend/servers/api-server/src/services/push_fanout.rs:330-341` — `bearer` is cloned from the static `oauth_token` on every `send_fcm_v1` call; no refresh path, no re-read of env, no `google-cloud-auth` token source (the code-comment at `:324-328` acknowledges "In production this would come from a service-account key file via google-cloud-auth" but the code just reuses a static env var).
- `backend/servers/api-server/src/services/push_fanout.rs:362-376` — the FCM error branch matches only device-registration statuses as `expired`; auth statuses (`UNAUTHENTICATED`, HTTP 401/403) fall through to `(false, false)` with a per-message `tracing::warn!("[8A-3] FCM rejected message")`.
- Dispatcher Tier-1d signal `code-review-api-core-fcm-oauth-token-stale` (`.research/signals/2026-08-28-api-core-tier1d-services.json`) — same defect, same evidence, `score_delta: 3`, expert `rust`.

## Files
- `backend/servers/api-server/src/services/push_fanout.rs:233`
- `backend/servers/api-server/src/services/push_fanout.rs:330`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug vector)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):**

Mode: cloud-ok

## Repro steps
1. Boot `api-server` with `FCM_PROJECT_ID` and `FCM_OAUTH_TOKEN` set. In a test, construct `FcmHttpAdapter` with a mock `fcm_base_url` pointing at a `wiremock`/`mockito` HTTP double.
2. First stubbed request: return `200` with an empty `FcmSendResponse` — assert `send_fcm_v1` returns `(true, false)` (baseline: authenticated OK).
3. Rewind the stub: subsequent requests return `HTTP 401` with body `{"error": {"status": "UNAUTHENTICATED", "message": "Request had invalid authentication credentials."}}`.
4. Call `send_fcm_v1` again — **expected**: the adapter surfaces a distinct credential-level failure (either via a new `TransportResult` discriminant, a bumped `tracing::error!` with `fcm_status = "UNAUTHENTICATED"`, or a re-acquisition attempt). **Actual today**: returns `(false, false)` and logs only the generic `[8A-3] FCM rejected message` warn — indistinguishable from a device-side rejection.

## Suggested approach
1. Extend the FCM error classifier in `push_fanout.rs:362-376`: treat `err_status == "UNAUTHENTICATED"` **and** any HTTP `401`/`403` as a new `credential_expired` classification (separate from `token_expired`).
2. On `credential_expired`: emit a `tracing::error!` (not `warn!`) with a distinct message (e.g. `[8A-3] FCM credential expired`), and record a metric/counter that operators can alert on. Do NOT delete the device token — the DB row is fine, the server credential is the problem.
3. Introduce a minimal refresh seam: define a `FcmBearerSource` trait with `async fn current(&self) -> Result<String, _>` and a default `EnvBearerSource` that just reads `FCM_OAUTH_TOKEN` every call (still no GCP SDK, but no captured-at-startup cache). `send_fcm_v1` swaps `self.fcm_config.oauth_token.clone()` for `self.bearer_source.current().await`. This is the minimum contract that lets a follow-up PR plug in a real service-account refresher without touching the send path.
4. If `bearer_source.current()` returns an empty/error value, fall back to `send_fcm_legacy` exactly as today (preserving the current behavior when only `FCM_SERVER_KEY` is present).
5. Add unit tests for the two new branches (see *Test plan*).
6. Out of scope: shipping the real Google-service-account refresher. That is a bigger change (adds a GCP dependency); the point of this plan is the *detection + seam*, so the operational blindness is fixed today and the refresher can land later against a stable contract.

## Alternatives considered
- **Ship a full google-cloud-auth-backed refresher inline** — rejected because it drags in a heavy GCP SDK dependency and expands the diff far beyond the observed defect; the operational blindness (silent 401s) is fixable in isolation, and adding the refresher against a stable seam is a clean follow-up.
- **Just log louder on 401 without a seam** — rejected because the underlying bug (static-at-startup credential) recurs on every restart; the refresh seam is what lets operations either automate the fix or plug in a real token source without re-editing the send path.

## Root-cause trace
1. Symptom: Android devices stop receiving push notifications ~1 h after an `api-server` restart; no operational alert fires.
2. ← `send_fcm_v1` at `push_fanout.rs:376` returns `(false, false)` on the 401 branch; the caller (`FcmHttpAdapter::send`) has no path to distinguish that from a device-side rejection.
3. ← The classifier at `push_fanout.rs:367-370` treats only `NOT_REGISTERED` / `UNREGISTERED` / `INVALID_REGISTRATION` as `expired`; auth statuses (`UNAUTHENTICATED`, HTTP 401/403) never set `expired = true` and never emit a distinct log/metric.
4. ← `bearer` at `push_fanout.rs:330-336` is a clone of `self.fcm_config.oauth_token`, populated once by `FcmConfig::from_env` at `push_fanout.rs:229-236`; there is no refresh timer, no re-read of env, and no service-account token source.
5. Origin: Epic 8A-3 (`push_fanout.rs:1-46` module doc) — the FCM HTTP v1 adapter shipped with an explicit acknowledgement in the code-comment that "in production this would come from a service-account key file via google-cloud-auth" and the env-var fallback was left in place as a stub; that stub was never followed up.

## Test plan
- [ ] Unit test in `backend/servers/api-server/src/services/push_fanout.rs` `#[cfg(test)] mod tests` (or `tests/push_fanout_credentials.rs` if adapter is testable stand-alone): construct an `FcmHttpAdapter` pointed at a `wiremock`/`mockito` `fcm_base_url`. Stub `HTTP 401` with FCM `err_status = "UNAUTHENTICATED"`. Assert the adapter emits a `credential_expired`-flavored `tracing::error!` (or the new discriminant, whichever the fix picks) — **fails on `main` because today's classifier silently falls through**.
- [ ] Regression test: stub `HTTP 200` with `err_status = "NOT_REGISTERED"` — assert existing `token_expired = true` path still fires (locks in current device-eviction behavior).
- [ ] Regression test: stub `HTTP 200` success — assert `(true, false)` (locks in happy path).
- [ ] Command: `cargo test -p api-server --test push_fanout_credentials` (or `cargo test -p api-server push_fanout` for the inline `mod tests` variant).

## Out of scope
- Shipping the real `google-cloud-auth`-backed service-account token refresher — that is a follow-up plan against the seam introduced here.
- APNs (`ApnsHttpAdapter`) auth handling — its P8 JWT refresh (`refreshed every 50 minutes` per module doc) is a separate mechanism and is not affected.
- Legacy `send_fcm_legacy` path — uses a long-lived server key and is not affected.

## After-merge
- Move this file to `plans/_archive/code-review-api-core-fcm-oauth-token-stale.md`.
- Mark the matching `backlog.json` row `code-review-api-core-fcm-oauth-token-stale` as `status: "done"`.
