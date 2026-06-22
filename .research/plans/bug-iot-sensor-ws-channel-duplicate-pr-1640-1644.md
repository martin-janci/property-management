# bug-iot-sensor-ws-channel-duplicate-pr-1640-1644

**Vector:** bug
**Score:** 3
**Source:** Issue #1668 | PR #1640 | PR #1644
**Confidence:** high

## Hypothesis
PR #1640 ("IoT real-time sensor readings WS channel") and PR #1644 ("realtime sensor reading WebSocket channel") both shipped Story 14.3 within hours of each other on 2026-06-21 and now coexist on `dev`. Both subscribe to the same Redis channel `sensors:{org_id}`, but PR #1644's ingest path renamed the published event names from `sensor.reading` / `sensor.batch_readings` → `sensor.reading.created` / `sensor.readings.batch`. PR #1640's endpoint forwards the raw `event_type` verbatim, so any client connected to `/api/v1/iot/ws` now silently receives PR #1644's renamed events — the documented wire format is broken with no failing test. Additionally, PR #1640's endpoint trusts the JWT `tenant_id` claim with no DB membership check, while PR #1644's endpoint enforces `OrganizationMemberRepository::is_member` — the weaker authZ path is still mounted. Converge on PR #1644's endpoint and event names, delete the PR #1640 path entirely, and add a publish→subscribe wire-contract test so a future rename fails CI.

## Evidence
- Issue #1668 (filed 2026-06-22 by `ppt-review-merged` skill) — full post-merge review with file:line
- PR #1640 (merged 2026-06-21T12:24Z) — adds `backend/servers/api-server/src/routes/ws_sensor.rs` (245 LoC), mounted at `lib.rs:257` via `.nest("/api/v1/iot", routes::ws_sensor::router())`; emits `sensor.reading` / `sensor.batch_readings`; trusts JWT `tenant_id`.
- PR #1644 (merged 2026-06-21T15:26Z) — adds `GET /api/v1/iot/sensors/ws` at `routes/iot.rs:145`; emits renamed `sensor.reading.created` / `sensor.readings.batch` at `routes/iot.rs:301,336`; verifies `is_member`.
- Both endpoints subscribe to `sensors:{org_id}` Redis channel — wire-format break is silent.
- `validate_access_token_full` in `backend/crates/api-core/src/extractors/auth.rs` is used only by `ws_sensor.rs`; becomes orphaned after the delete.

## Files
- `backend/servers/api-server/src/lib.rs`
- `backend/servers/api-server/src/routes/mod.rs`
- `backend/servers/api-server/src/routes/ws_sensor.rs`
- `backend/servers/api-server/src/routes/iot.rs`
- `backend/crates/api-core/src/extractors/auth.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [x] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Start a Redis client; subscribe to `sensors:<some-org-id>`.
2. Connect a WebSocket client to `ws://localhost:8080/api/v1/iot/ws?token=<JWT for that org>` (PR #1640 endpoint).
3. POST a sensor reading via the ingest path (`POST /api/v1/iot/sensors/{sensor_id}/readings`) — this goes through PR #1644's publisher.
4. Expected (per #1640 wire-format docs): WS client receives `{"event":"sensor.reading", ...}`. Actual: WS client receives `{"event":"sensor.reading.created", ...}` — the publisher renamed the event.

## Suggested approach
1. Delete `backend/servers/api-server/src/routes/ws_sensor.rs` entirely.
2. In `backend/servers/api-server/src/routes/mod.rs`, remove `pub mod ws_sensor;`.
3. In `backend/servers/api-server/src/lib.rs`, remove the `.nest("/api/v1/iot", routes::ws_sensor::router())` line (~line 257). Keep PR #1644's `sensor_router()` mount as the single source of truth at `GET /api/v1/iot/sensors/ws`.
4. In `backend/crates/api-core/src/extractors/auth.rs`, remove the now-orphaned `validate_access_token_full` (verify with `cargo check -p api-core` that no other crate references it).
5. Add a wire-contract integration test in `backend/servers/api-server/tests/iot_ws_contract_tests.rs` that: (a) starts an in-process test server with the iot router mounted; (b) connects a WS client to `/api/v1/iot/sensors/ws?organization_id=...`; (c) publishes via the ingest path's `add_reading`; (d) asserts the WS frame's `event` field equals `sensor.reading.created`. Use the publisher's exported event-name constants on both sides so a future rename forces both call sites to change together.
6. Optional: if `/api/v1/iot/ws` was already advertised to any client, leave a one-line alias delegating to `sensor_ws_handler` rather than a 404. Confirm with the BIT-135 frontend story owner whether any caller exists; otherwise skip the alias.
7. Run `cargo test -p api-server --test iot_ws_contract_tests` and `cargo clippy -p api-server --all-targets -- -D warnings`.

## Alternatives considered
- **Keep both endpoints, fix the publisher to re-emit both event-name variants** — rejected because it locks in the wire-format duplication forever and leaves PR #1640's weaker `tenant_id`-trust authZ path live. The correctness-and-security win is on the deletion side.
- **Keep PR #1640's endpoint and revert PR #1644's event-name rename** — rejected because the renamed names (`sensor.reading.created` / `sensor.readings.batch`) are more semantically correct and PR #1644's authZ model is the safer one. Reverting trades correctness for compat with an endpoint the frontend (BIT-135) is unlikely to have shipped against yet.

## Root-cause trace
1. Symptom: any client on `/api/v1/iot/ws` (PR #1640) silently receives events with the wrong `event` field (`sensor.reading.created` instead of `sensor.reading`); WS payload contract broken on `dev`.
2. ← `backend/servers/api-server/src/routes/iot.rs:301,336` — `add_reading`/`add_batch_readings` publish with renamed event names.
3. ← `backend/servers/api-server/src/routes/ws_sensor.rs` (PR #1640) — forwards `event_type` verbatim, no name negotiation.
4. ← Two PRs landed within hours of each other (PR #1640 at 2026-06-21T12:24Z, PR #1644 at 2026-06-21T15:26Z) for the same Story 14.3 with no coordination; neither test covers the publisher↔subscriber wire contract.
5. Origin: parallel claim of Story 14.3 by two implementers without a "same-story dedup" check in the dispatcher's promotion path. (Process-level fix is out of scope here.)

## Test plan
- [ ] Integration test `backend/servers/api-server/tests/iot_ws_contract_tests.rs` that asserts `sensor.reading.created` is the wire-format event name received on `/api/v1/iot/sensors/ws` when `add_reading` publishes — should fail today (no such test exists), pass after.
- [ ] `cargo test -p api-server --test iot_ws_contract_tests`
- [ ] `cargo build -p api-server` after the delete — confirms `validate_access_token_full` removal didn't break a hidden consumer
- [ ] `cargo clippy -p api-server --all-targets -- -D warnings`

## Out of scope
- Any change to the frontend WS subscriber (BIT-135 / FR72) — it should already be pointed at `/api/v1/iot/sensors/ws` per PR #1685; if not, that's a separate frontend follow-up.
- Adding sensor-WS authentication beyond what PR #1644 already does — `is_member` is the right gate.
- Backporting PR #1640's `ws_sensor.rs` style improvements into `iot.rs` (PR #1644's handler is the survivor as-is).

## After-merge
- Move this file to `plans/_archive/bug-iot-sensor-ws-channel-duplicate-pr-1640-1644.md`
- Mark `backlog.json` row `bug-iot-sensor-ws-channel-duplicate-pr-1640-1644` as `status: "done"`
