# security-voice-device-idor

**Vector:** security
**Score:** 3
**Source:** commit 83eae73 (origin/dev HEAD) — handler `unlink_voice_device` in `backend/servers/api-server/src/routes/ai.rs:3002`
**Confidence:** high

## Hypothesis
The `DELETE /api/v1/ai/llm/voice/devices/{id}` handler (`unlink_voice_device`) binds the authenticated principal as `_principal` and then discards it, calling `deactivate_voice_device(id)` with nothing but the path UUID. The underlying query updates `voice_assistant_devices` filtered only by `WHERE id = $1`, so any authenticated user can deactivate any other tenant's voice device by supplying its UUID — a cross-tenant write (IDOR). The smallest correct fix is to scope the mutation to the caller's ownership: pass the principal's identity into the repository call and add an ownership/tenant predicate to the `UPDATE`, returning `404` when the row is not owned by the caller.

## Evidence
- `backend/servers/api-server/src/routes/ai.rs:3002` — `unlink_voice_device(State, _principal: RequestPrincipal, Path(id))` ignores `_principal` and calls `state.llm_document_repo.deactivate_voice_device(id)`
- `backend/servers/api-server/src/routes/ai.rs:2059` — route is wired: `.route("/voice/devices/{id}", delete(unlink_voice_device))`
- `backend/crates/db/src/repositories/llm_document.rs:1141` — `UPDATE voice_assistant_devices SET is_active = FALSE, updated_at = NOW() WHERE id = $1` has no owner/org predicate
- `link_voice_device` (the POST sibling) records the device under the caller; the DELETE path never re-checks that association
- Found by the rotating expert (Rust) review of segment `api-core`; call path traced end-to-end from the Axum route to the unscoped `UPDATE`

## Files
- `backend/servers/api-server/src/routes/ai.rs:3002`
- `backend/crates/db/src/repositories/llm_document.rs:1139`

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [x] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):**
Mode: cloud-ok

## Repro steps
1. Authenticate as user A and `POST /api/v1/ai/llm/voice/devices` to create a voice device; note its returned `id` (call it `DEV_A`).
2. Authenticate as a different user B in a different org (B has no relationship to `DEV_A`).
3. As B, call `DELETE /api/v1/ai/llm/voice/devices/{DEV_A}`.
4. Expected: `404 Not Found` (B cannot see or affect A's device). Actual today: `204 No Content` and `voice_assistant_devices.is_active` for `DEV_A` flips to `FALSE` — B silently disabled A's device.

## Suggested approach
1. Determine the ownership column on `voice_assistant_devices` (inspect the migration that creates the table and how `link_voice_device` / `create_voice_device` populate it — likely a `user_id` and/or org/building scope).
2. Change `deactivate_voice_device` in `backend/crates/db/src/repositories/llm_document.rs:1139` to accept the caller's scoping value(s) and add them to the `WHERE` clause, e.g. `WHERE id = $1 AND user_id = $2` (mirror whatever predicate `find`/`list` for voice devices already use).
3. In `unlink_voice_device` (`ai.rs:3002`), rename `_principal` to `principal`, extract the identity/tenant the repository expects, and pass it into `deactivate_voice_device`.
4. Keep the existing `Ok(false) => 404` branch — with the scoped query, a non-owned or non-existent id naturally returns `false`, so an attacker gets an indistinguishable `404`.
5. Audit the other voice handlers wired near `ai.rs:2057-2060` (`list_voice_devices`, `link_voice_device`, `list_voice_commands`) for the same missing-scope pattern and fix any that share it within this PR's blast radius.

## Alternatives considered
- **Fetch-then-check in the handler (load device, compare owner, then call unscoped deactivate)** — rejected because it introduces a check-then-act race and an extra round-trip; folding the predicate into the single `UPDATE` is atomic and simpler.
- **Enforce via Postgres RLS only** — rejected as the sole fix because this repository method takes a plain `id` and the call site does not pass an RLS-scoped connection here; adding the explicit predicate is the reliable, testable change. (RLS hardening can still land separately under the existing RLS backlog.)

## Root-cause trace
1. Symptom: user B's `DELETE …/voice/devices/{DEV_A}` returns `204` and disables user A's device.
2. ← immediate cause at `backend/servers/api-server/src/routes/ai.rs:3002` — handler ignores `_principal`, passing only `id` to the repository.
3. ← upstream cause at `backend/crates/db/src/repositories/llm_document.rs:1141` — the `UPDATE` filters on `id` alone, with no owner/tenant predicate, so the authorization decision was never expressed in the query.
4. Origin: the voice-assistant feature (Story 64.5) shipped the deactivate path without an ownership check; the `_principal` underscore binding shows the scope was scaffolded but never wired.

## Test plan
- [ ] Integration test in the api-server suite: user B deleting user A's voice device gets `404` and `DEV_A.is_active` stays `TRUE` (this fails on `dev` today — currently returns `204` and flips the flag).
- [ ] Positive regression: user A deleting their own device still returns `204` and flips `is_active` to `FALSE`.
- [ ] Run: `cargo test -p api-server --lib` (plus the voice-device integration test once added); confirm the new cross-tenant test fails before the fix and passes after.

## Out of scope
- Broader RLS-connection migration for the `llm_document` repository (tracked separately).
- Soft-delete vs hard-delete semantics for voice devices.
- Rate limiting or audit logging of device-unlink actions.

## After-merge
- Move this file to `plans/_archive/security-voice-device-idor.md`
- Mark the matching `backlog.json` row as `status: "done"`
