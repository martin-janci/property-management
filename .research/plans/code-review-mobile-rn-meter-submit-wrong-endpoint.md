# code-review-mobile-rn-meter-submit-wrong-endpoint

**Vector:** bug
**Score:** 3
**Source:** code-review-finding (Tier-1d mobile-rn 2026-10-14)
**Confidence:** high

## Hypothesis
The mobile Meter Reading screen POSTs every submission to `/api/v1/meter-readings`, a path the api-server never mounts. The real endpoint is `POST /api/v1/meters/readings` with a `SubmitReading` body (`meter_id: Uuid`, `reading: Decimal`, optional `photo_url`/`reading_date`); the mobile body sends `{ meterType, reading, unit, photoUri }` — no `meter_id`, wrong shape — so every submission 404s. The offline queue treats 404 as terminal and drops the action, while handleSubmit has already shown the user an optimistic success Alert. Fix: resolve the user's meter UUID for the selected type, POST to the correct path with a `SubmitReading`-shaped body, and surface the real result rather than an optimistic Alert.

## Evidence
- `frontend/apps/mobile/src/screens/meters/MeterReadingScreen.tsx:128-138` — `addToQueue({ endpoint: '/api/v1/meter-readings', method: 'POST', body: { meterType, reading, unit, photoUri } })`
- `backend/servers/api-server/src/routes/meters.rs:44` — `.route("/readings", post(submit_reading))` nested under `/api/v1/meters`; utoipa path `/api/v1/meters/readings`
- `backend/crates/db/src/models/meter.rs:193-204` — `SubmitReading { meter_id: Uuid, reading: Decimal, reading_date?, reading_time?, source?, photo_url? }`
- `frontend/apps/mobile/src/hooks/useOfflineSupport.ts:437-465` — 4xx is classified `isPermanent` → `droppedPermanent` + `console.error`, action discarded
- `frontend/apps/mobile/src/screens/meters/MeterReadingScreen.tsx:144-152` — `Alert.alert(t('meters.submitSuccess') / t('meters.savedOffline'))` fires optimistically at enqueue time, before any server round-trip

## Files
- `frontend/apps/mobile/src/screens/meters/MeterReadingScreen.tsx:128`
- `frontend/apps/mobile/src/hooks/useOfflineSupport.ts:437`
- `backend/servers/api-server/src/routes/meters.rs:44`
- `backend/crates/db/src/models/meter.rs:193`

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. On the mobile app, open the Meter Reading screen and submit a reading (any `meterType`, any value, with or without a photo).
2. Observe the submit succeeds in-app (success Alert).
3. Observe the queued action in the offline queue — the endpoint is `/api/v1/meter-readings` and the body shape is `{ meterType, reading, unit, photoUri }`.
4. Expected: a persisted reading on the server (visible via `GET /api/v1/meters/{id}/readings`). Actual: 404 from the server, action silently dropped by `useOfflineSupport.ts`'s `isPermanent` branch — no reading on the server, user believes it saved.

## Suggested approach
1. Add a meter-list query to the screen (e.g. `useMeters(unitId)`) so the user picks a server-known meter rather than only a hardcoded `METER_TYPES` enum, and keep the actual `meter_id` in form state. If no meter exists for the selected type on the current unit, surface a disabled state + explanation instead of letting submit proceed.
2. Change `handleSubmit` in `MeterReadingScreen.tsx:122-138` to enqueue `{ endpoint: '/api/v1/meters/readings', method: 'POST', body: { meter_id, reading: <numeric>, reading_date, photo_url } }`. Convert `reading` from the raw string to a numeric string the backend's `rust_decimal` serde accepts, and drop `meterType`/`unit`/`photoUri` keys.
3. Replace the optimistic Alert at `MeterReadingScreen.tsx:144-152` with a pending UI state driven by `addToQueue`'s result (or the queue flush callback); only confirm success once the action transitions out of pending.
4. Treat the photo path as a known gap for now (tracked separately): either hide the photo capture button or clearly label it "Preview only — photo upload not yet available" so the user is not misled.
5. Add a jest-expo test under `frontend/apps/mobile/src/screens/meters/__tests__/MeterReadingScreen.test.tsx` that mounts the screen, drives a submit, and asserts the enqueued action's endpoint equals `/api/v1/meters/readings` AND the body has `meter_id` + numeric `reading` — this test fails on main today.
6. Run `pnpm -F @ppt/mobile test MeterReadingScreen` locally; it must fail before step 2 and pass after.

## Alternatives considered
- **Mount `/api/v1/meter-readings` on the api-server as an alias** — rejected because it keeps the wrong payload shape (no `meter_id`), still requires correlating `meterType` → meter row server-side, and leaves two names for one endpoint going forward.
- **Keep the client body as-is and have the backend infer `meter_id` from `(unit_id, meterType)`** — rejected because a unit can have multiple meters of the same type (e.g. two electricity sub-meters), the inference is ambiguous, and the mobile code already needs a meter-list for the user to disambiguate.

## Root-cause trace
1. Symptom: resident submits a meter reading on the mobile app → in-app success Alert, no server-side reading persisted (silent data loss).
2. ← `useOfflineSupport.ts:437-465` classifies the 404 as `isPermanent` and drops the queued action with `console.error`, invisible to the user.
3. ← `MeterReadingScreen.tsx:128-138` enqueues `/api/v1/meter-readings` with a `{ meterType, reading, unit, photoUri }` body — path does not exist (`meters.rs:44` nests readings at `/api/v1/meters/readings`), shape mismatches `SubmitReading` (`db/src/models/meter.rs:193-204`).
4. Origin: screen author selected a hardcoded `METER_TYPES` enum rather than consuming the real meter list and the real OpenAPI contract (`SubmitReading`); no jest-expo test asserted the enqueued endpoint/body ever matched the server contract.

## Test plan
- [ ] `frontend/apps/mobile/src/screens/meters/__tests__/MeterReadingScreen.test.tsx` — mount, submit, assert queued action `endpoint === '/api/v1/meters/readings'` and `body.meter_id` is a UUID + `body.reading` is a numeric string.
- [ ] Regression: assert that in the no-meter-available case the submit button is disabled (no silent-success path).
- [ ] Local run: `cd frontend && pnpm -F @ppt/mobile test MeterReadingScreen`

## Out of scope
- Full photo upload pipeline (presigned-URL capture → store → `photo_url` string). Tracked separately as `code-review-mobile-rn-meter-photo-not-uploaded`. This plan removes the photo path from the submit body or labels it as unavailable; it does not add real upload.
- Meter CRUD UI on mobile (listing/creating meters). This plan only consumes an existing meter list; if the API doesn't support that, surface the gap instead of adding server work.

## After-merge
- Move this file to `plans/_archive/code-review-mobile-rn-meter-submit-wrong-endpoint.md`
- Mark the matching `backlog.json` row as `status: "done"`
