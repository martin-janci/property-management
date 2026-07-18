# code-review-api-core-voice-cmd-fabricated-success

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review api-core 2026-07-17 (rust expert)
**Confidence:** high

## Hypothesis
Two voice-assistant action handlers ship as stubs on a live webhook path yet return `VoiceActionResult { success: true }` with fabricated data. `action_check_balance()` always answers "your balance is zero, no outstanding fees" (hardcoded `balance: 0.0`) without consulting the financial repository; `action_report_fault()` mints a throwaway `Uuid::new_v4()` ticket, speaks it back to the resident, and never persists a fault. Both handlers are reachable in production via `routes/voice_webhooks.rs` → `VoiceCommandService::process_command`. The smallest safe fix is to make each handler call the corresponding repository (financial for balance, fault for report) and return `success: false` with a graceful error path when the repository call fails, so the caller cannot be told "balance is zero" or "ticket recorded" unless it is true.

## Evidence
- `backend/servers/api-server/src/services/voice_commands.rs:235-266` — `action_check_balance` returns hardcoded `data.balance=0.0`, `success: true`, response text "Your current balance is zero. You have no outstanding fees." Comment on line 240: "In a real implementation, this would query the financial repository".
- `backend/servers/api-server/src/services/voice_commands.rs:267-310` — `action_report_fault` calls `Uuid::new_v4()` for the ticket id, speaks "The manager will be notified shortly", never touches the fault repo. Comment on line 279: "In a real implementation, this would create a fault via the fault repository".
- `backend/servers/api-server/src/services/voice_commands.rs:143-144` — dispatch table wires the two intents to these stubs; no feature-flag or dev-only gate wraps them.
- `backend/servers/api-server/src/routes/voice_webhooks.rs:146,162,265` — three public webhook entry points that call `VoiceCommandService::process_command`, so the stubs are reachable from Google/Amazon/Twilio voice-assistant integrations.
- `backend/crates/db/src/repositories/fault.rs`, `backend/crates/db/src/repositories/financial.rs` — repositories the stubs claim in comments already exist.

## Files
- `backend/servers/api-server/src/services/voice_commands.rs`
- `backend/servers/api-server/src/routes/voice_webhooks.rs`
- `backend/crates/db/src/repositories/fault.rs`
- `backend/crates/db/src/repositories/financial.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

Mode: cloud-ok

## Repro steps
1. Start api-server locally with any voice-assistant device row already seeded (or POST `/voice/register` to create one).
2. `POST /voice/webhooks/twilio` (or the equivalent Google/Amazon path) with `command_text="What is my balance?"` and a `device_id` pointing at a resident with non-zero outstanding fees in `financial_transactions`.
3. Expected: response text quotes the resident's actual balance, `data.balance` matches DB, `success: true` only if the repo query succeeded.
4. Actual: response text is "Your current balance is zero. You have no outstanding fees.", `data.balance == 0.0`, regardless of DB state.
5. Repeat step 2 with `command_text="Report a fault: gas smell in unit"`. Expected: a row in `faults` (or the equivalent table) whose `id` matches the ticket number spoken back to the resident. Actual: no row is written, the ticket id is a fresh `Uuid::new_v4()` that maps to nothing.

## Suggested approach
1. In `voice_commands.rs::action_check_balance`, inject the financial repository (or a repository trait) into `VoiceCommandService` so the handler can call e.g. `financial_repo.get_outstanding_balance(device.unit_id).await`. Return `success: false` with a localised "sorry, we couldn't reach your ledger" response on error.
2. Replace the hardcoded response and `data.balance = 0.0` with the actual figure and currency. Localise using the existing per-language match arm.
3. In `voice_commands.rs::action_report_fault`, call `fault_repo.create_fault(...)` with the parsed description, `device.unit_id`, and a source enum (add `FaultSource::Voice` if it doesn't already exist). Use the returned row's id (not `Uuid::new_v4()`) in the spoken response.
4. When the repository call fails, return `success: false` with a localised "we couldn't file the ticket; please try again" message so the resident is not misled.
5. Add an integration test at `backend/servers/api-server/tests/voice_command_actions_tests.rs` that stubs the two repositories, asserts the wiring, and asserts the exact `data.balance` / persisted fault-row id shape.
6. Grep the code base for `Uuid::new_v4()` inside `services/voice_commands.rs` and `services/actions/`; fail the review if any live handler still fabricates ids.
7. If either repository call requires an org/tenant context, thread `set_tenant_context(device.org_id)` through the same RLS helper the rest of the voice webhook uses (see `voice_webhooks.rs`).

## Alternatives considered
- **Feature-flag the two handlers off until the wiring lands** — rejected because the flag would silently disable a shipped, marketed capability instead of fixing it, and the flag itself is a new lever that requires ops discipline. The fix is small enough (two handlers, two repos) that gating adds more risk than it saves.
- **Return `success: false` with a "not implemented" message and file a follow-up** — rejected because it leaves the marketed feature broken in production and defers the actual repository wiring, which is what makes the finding actionable rather than a triage row.

## Root-cause trace
1. Symptom: voice caller is told "balance is zero" regardless of actual balance; spoken fault reports never appear in the operator queue.
2. ← `services/voice_commands.rs:249-266` returns fabricated `data.balance=0.0` unconditionally.
3. ← Same file :285-299 returns a fresh `Uuid::new_v4()` as the ticket id without any persistence call.
4. ← Both handlers were shipped with "In a real implementation" comments — they are placeholder implementations that were wired into the dispatcher (line 143-144) without gating.
5. Origin: initial commit that introduced `VoiceCommandService` (predates the tier1d review window; verify with `git blame backend/servers/api-server/src/services/voice_commands.rs`).

## Test plan
- [ ] `backend/servers/api-server/tests/voice_command_actions_tests.rs::action_check_balance_uses_repo` — asserts the handler queries the financial repo and echoes the returned balance.
- [ ] `backend/servers/api-server/tests/voice_command_actions_tests.rs::action_report_fault_persists` — asserts the fault row exists after a successful call and its id matches the spoken response.
- [ ] Failing-on-main regression: assert that with the stub in place, `data.balance` is not equal to the seeded outstanding balance (this fails today, passes after the wiring).
- [ ] `cargo test -p api-server voice_command_actions`

## Out of scope
- New voice intents beyond the two named handlers.
- Voice-assistant privacy/consent flows.
- Localisation of new error messages beyond sk/cs/en parity.
- Reworking `VoiceCommandService::process_command` dispatch table.

## After-merge
- Move this file to `plans/_archive/<slug>.md`
- Mark the matching `backlog.json` row as `status: "done"`
