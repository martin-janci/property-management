# code-review-reality-server-sso-reqwest-missing-timeout

**Vector:** bug
**Score:** 3
**Source:** segment=reality-server rotating review (2026-08-28)
**Confidence:** high

## Hypothesis
The three OAuth-related HTTP clients in `reality-server/src/routes/sso.rs` (`exchange_code_for_tokens`, `get_user_info`, `introspect_pm_token`) each construct a bare `reqwest::Client::new()` with no `.timeout(...)`. When the Property-Management OAuth server is slow, unreachable, or drops packets, the axum task awaits `send()` indefinitely — socket pool exhaustion + a public DoS surface on the anonymous `GET /api/v1/sso/callback` handler. The fix is to build a shared timeout-configured `reqwest::Client` at `AppState` construction (mirroring how `PmApiClient` in `state.rs:757` already does it) and use it in all three call sites.

## Evidence
- `backend/servers/reality-server/src/routes/sso.rs:687` — `let client = reqwest::Client::new();` inside `exchange_code_for_tokens`
- `backend/servers/reality-server/src/routes/sso.rs:711` — same pattern inside `get_user_info`
- `backend/servers/reality-server/src/routes/sso.rs:748` — same pattern inside `introspect_pm_token`
- `backend/servers/reality-server/src/state.rs:757` — `PmApiClient` already builds `reqwest::Client::builder().timeout(...)` — the shared-client-with-timeout template to copy
- Reached from the public `GET /api/v1/sso/callback` handler (`sso.rs` route registration)

## Files
- `backend/servers/reality-server/src/routes/sso.rs`
- `backend/servers/reality-server/src/state.rs`

## Dependencies
- (none — self-contained single-crate change)

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
1. Stand up `reality-server` locally against a `PmApiClient` base URL that drops connections (e.g. point `PM_API_BASE_URL` at a black-hole port `127.0.0.1:1`).
2. Send `GET /api/v1/sso/callback?code=x&state=y` with any valid pending session id.
3. Observe: the handler task hangs indefinitely — no 5xx, no request timeout ever fires. Expected: request completes with a 502/504 within ~10 s and the task exits.

## Suggested approach
1. Add a `sso_http_client: reqwest::Client` field on `AppState` (`state.rs`), constructed once with `reqwest::Client::builder().timeout(Duration::from_secs(10)).connect_timeout(Duration::from_secs(5)).build()?`. Mirror the pattern already in use for `PmApiClient` at `state.rs:757`.
2. Wire the new field through `AppState::new(...)` construction (same call site that builds `PmApiClient`).
3. In `sso.rs`, replace each `let client = reqwest::Client::new();` at lines 687 / 711 / 748 with `let client = &state.sso_http_client;` (and pass through the `AppState` handle already available in the handler signatures).
4. Convert the timeout constants to `Duration::from_secs` values pulled from `AppState` config if there's an existing config knob, else hardcode 10 s (matching `PmApiClient` timeout).
5. Update any existing `sso.rs` tests that stubbed a `reqwest::Client` to instead accept a client from state.

## Alternatives considered
- **Add `.timeout(...)` inline at each call site** — rejected because it re-creates a new TCP pool per request (defeats connection reuse) and leaves the "one shared timeout policy" requirement scattered across 3 sites (drift risk).
- **Wrap `send()` in `tokio::time::timeout(...)`** — rejected because `reqwest` already exposes a proper timeout that surfaces a typed `Error::Timeout`; a manual `tokio::time::timeout` swallows the reqwest error context and requires per-site error mapping.

## Root-cause trace
1. Symptom: `GET /api/v1/sso/callback` awaits forever when the PM OAuth backend hangs.
2. ← Handler awaits `client.post(...).send()` at `sso.rs:687` with no timeout.
3. ← `client` is a fresh `reqwest::Client::new()` — `Client::new()` builds with no request timeout (per reqwest docs).
4. Origin: SSO handlers were added without adopting the `AppState.pm_api_client` timeout convention already established at `state.rs:757`.

## Test plan
- [ ] Unit test in `sso.rs` mod tests — call `exchange_code_for_tokens` against a `wiremock` mock that hangs `.send()` indefinitely; assert the call returns `Err(reqwest::Error)` with `is_timeout() == true` within ~11 s.
- [ ] Regression: repeat for `get_user_info` and `introspect_pm_token`.
- [ ] Local run: `cargo test -p reality-server --test sso_integration -- --nocapture` (add `sso_integration.rs` if not present).

## Out of scope
- Reworking the SSO flow's error taxonomy (`SsoError` variants).
- Adding retries — this plan lands the timeout only; retry policy is a separate story.
- Changing `PmApiClient` — it already has a timeout.

## After-merge
- Move this file to `plans/_archive/code-review-reality-server-sso-reqwest-missing-timeout.md`
- Mark the matching `backlog.json` row as `status: "done"`
