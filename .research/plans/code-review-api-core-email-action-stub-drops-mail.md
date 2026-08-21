# code-review-api-core-email-action-stub-drops-mail

**Vector:** bug
**Score:** 3
**Source:** api-core segment review 2026-08-21 (Phase 1.5 code-review slice)
**Confidence:** medium

## Hypothesis
The production `EmailExecutor::execute()` used by `WorkflowExecutorTask` logs a `tracing::info!` and returns `ActionResult::success` without ever calling `EmailService`. Every workflow "send email" action a manager configures records step + execution as COMPLETED while no mail is actually sent. `EmailService` is already wired for `scheduler` and `NotificationService`, so the fix is to inject it into `ActionRegistry` (or into the `EmailExecutor` directly) and call `send_template_email` on the resolved target before returning success.

## Evidence
- `backend/servers/api-server/src/services/actions/email.rs:86-113` — `execute()` only logs via `tracing::info!` then returns `ActionResult::success(...)`; the real send is commented out (`// email_service.send_template_email(&to, &template, template_data).await?`).
- `backend/servers/api-server/src/services/actions/mod.rs:204` — `registry.register(Box::new(EmailExecutor::new()));` inside `ActionRegistry::new()` — this is the executor used in prod, not a test double.
- `backend/servers/api-server/src/services/workflow_executor.rs:236` and `:286` — both `WorkflowExecutorTask` constructors instantiate `ActionRegistry::new()`.
- `backend/servers/api-server/src/services/workflow_executor.rs:894` and `:951-972` — a successful stub `execute()` marks the step `step_status::COMPLETED` and then `set_execution_status(COMPLETED)`, so the workflow reports success while the email silently disappears.
- `backend/servers/api-server/src/services/notification.rs:23,137,150` — `EmailService` is already used elsewhere (via `NotificationService`) and has both a `development()` constructor and a real send path — the wiring already exists, it's just not plugged in here.

## Files
- `backend/servers/api-server/src/services/actions/email.rs`
- `backend/servers/api-server/src/services/actions/mod.rs`
- `backend/servers/api-server/src/services/workflow_executor.rs`
- `backend/servers/api-server/src/services/notification.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug — silent-success data loss)
- [x] C2 — Seed data (workflow + email action config)
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):** no C4, no C5 → `Mode: cloud-ok`

Mode: cloud-ok

## Repro steps
1. Seed a workflow whose action is `{"type":"email","to":"user@example.com","subject":"hi","template":"welcome","body":"..."}` and attach it to a trigger.
2. Fire the trigger so `WorkflowExecutorTask` runs the action end to end.
3. Expected: the workflow step reports success **and** a call to `EmailService::send_template_email` is observed (assert via a test double / spy). Actual today: step reports success, `tracing::info!` fires, no `EmailService` call happens — the email is silently dropped.

## Suggested approach
1. Add an `email_service: Arc<EmailService>` field to `EmailExecutor` and switch `EmailExecutor::new()` to `EmailExecutor::new(email_service: Arc<EmailService>)`. Keep a `EmailExecutor::development()` that uses `EmailService::development()` for tests.
2. Thread `email_service: Arc<EmailService>` through `ActionRegistry::new(email_service, notification_service)` at `services/actions/mod.rs:196-208`; update the two `ActionRegistry::new()` call sites in `services/workflow_executor.rs:236,286` (`WorkflowExecutorTask::new(...)` and its variant) to pass the existing `Arc<EmailService>` already constructed for the scheduler / notification service.
3. In `EmailExecutor::execute()` (`services/actions/email.rs:86-113`), replace the "Simulate email sending" comment with `self.email_service.send_template_email(&to, &email_config.template, /* template_data */).await.map_err(|e| ActionError::ExecutionFailed(e.to_string()))?;` and only then build the `ActionResult::success` payload.
4. Map the `EmailService` error type to `ActionError::ExecutionFailed(...)` so the step transitions to `step_status::FAILED` in `services/workflow_executor.rs:894` instead of silently succeeding.
5. Reuse the substitution logic already at `services/actions/email.rs:75-76` for `subject` / `body` so `template_data` matches the shape `EmailService::send_template_email` expects.
6. Do the sibling refactor for `NotificationExecutor` in a separate plan (`code-review-api-core-notif-action-stub-noop`) — same wiring, different service.

## Alternatives considered
- **Enqueue the send into a background job (Redis / worker)** — rejected because it postpones failure surfacing to a queue reader we don't have wired for actions; the fix must at minimum surface send errors up to the workflow-step status, and inline `send_template_email` already does that.
- **Leave the stub and gate the executor behind a `SIMULATE_ACTIONS=1` env var** — rejected because production today runs without any such gate and the current behaviour is a silent data-loss bug, not a deliberate simulation.

## Root-cause trace
1. Symptom: managers configure workflow email actions; the workflow status shows COMPLETED; no email arrives.
2. ← `EmailExecutor::execute()` at `backend/servers/api-server/src/services/actions/email.rs:86-113` returns `Ok(ActionResult::success(...))` without invoking `EmailService`.
3. ← `WorkflowExecutorTask` at `backend/servers/api-server/src/services/workflow_executor.rs:894,951-972` reads that `Ok(...)` and stamps step + execution as COMPLETED.
4. Origin: the executor was written as a scaffold ("Simulate email sending (in production, integrate with EmailService)") and never wired to `EmailService` even after `EmailService` landed for the scheduler/notification paths.

## Test plan
- [ ] `backend/servers/api-server/tests/` — integration test that drives `WorkflowExecutorTask` end-to-end with a spy `EmailService`, asserts `send_template_email` was called exactly once with the resolved `to`/`template`.
- [ ] Regression: a workflow whose action target `to` fails address-validation still returns `ActionError::ConfigurationError` (existing behaviour preserved).
- [ ] Command: `cargo test -p api-server workflow_email_action`

## Out of scope
- Wiring `NotificationExecutor` to `NotificationService` — sibling plan `code-review-api-core-notif-action-stub-noop`.
- Adding a queued / retry-with-backoff email path.
- Refactoring `ActionRegistry` construction to a builder pattern.
- Anything under `voice_commands.rs` — different subsystem, tracked separately.

## After-merge
- Move this file to `plans/_archive/code-review-api-core-email-action-stub-drops-mail.md`
- Mark the matching `backlog.json` row as `status: "done"`
