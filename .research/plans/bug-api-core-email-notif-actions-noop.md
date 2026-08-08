# bug-api-core-email-notif-actions-noop

**Vector:** bug
**Score:** 3
**Source:** Phase 1.5 rotating review 2026-08-08 (api-core)
**Confidence:** high

## Hypothesis

`EmailExecutor::execute` and `NotificationExecutor::execute` — the live production handlers registered by `ActionRegistry::new()` for the `send_email` and `send_notification` workflow action types — log the outbound event and then return `ActionResult::success(...)` without calling any delivery code. Any workflow (fault-alert, announcement automation, dispute email fan-out) configured with either action is marked `COMPLETED` while nothing is actually delivered. The smallest fix is to wire each executor to the existing `EmailService` (`services/email.rs`) and `NotificationService` (`services/notification.rs`) — both already ship the concrete `send_template_email` / `send_to_users` APIs the executors need — and mark the action `ActionResult::failure(...)` on delivery error instead of eating it.

## Evidence

- `backend/servers/api-server/src/services/actions/email.rs:86-101` — `EmailExecutor::execute` logs the outgoing email then returns `ActionResult::success()` unconditionally; comment at line 96 says "Simulate email sending... For now, we consider the action successful". No `EmailService` handle exists on the struct.
- `backend/servers/api-server/src/services/actions/notification.rs:182-198` — identical shape: `tracing::info!(...)` then `Ok(ActionResult::success(...))`. Comment at line 193 says "In production, this would actually send notifications via the notification service".
- `backend/servers/api-server/src/services/actions/mod.rs:204-207` — `ActionRegistry::new()` registers `EmailExecutor::new()` and `NotificationExecutor::new()` as the live production executors for the built-in `send_email` / `send_notification` action types.
- `backend/servers/api-server/src/services/workflow_executor.rs:912` — `WorkflowExecutorTask::run` calls the registry executor's `execute()` directly on the live path (not gated behind any test/dev flag).
- `backend/servers/api-server/src/services/email.rs:240` / `services/notification.rs:399` — `EmailService::send_template_email` and `NotificationService::send_to_users` already exist and are used elsewhere in the codebase; wiring is a matter of passing a handle into the executor constructors.

## Files

- `backend/servers/api-server/src/services/actions/email.rs`
- `backend/servers/api-server/src/services/actions/notification.rs`
- `backend/servers/api-server/src/services/actions/mod.rs`
- `backend/servers/api-server/src/services/workflow_executor.rs`

## Dependencies

(none)

## Required capabilities

- [x] C1 — Systematic debugging (bug vector, workflow-engine surface)
- [ ] C2 — Seed data
- [x] C3 — Dev instance running (integration test exercises the workflow engine end-to-end; a running api-server + Postgres is the smallest surface)
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok
_(No browser or mobile device; the fix is server-side and testable via `cargo test`.)_

## Repro steps

1. Author a workflow with an action of type `send_email` (template + recipient) OR `send_notification` (target + title/message).
2. Trigger the workflow (e.g. `POST /api/v1/workflows/{id}/execute` with a matching event context).
3. Observe the workflow-execution record: `status = "completed"`, the action's `output_data.sent_at` is populated, and the action's log line reads `"Workflow sending email"` / `"Workflow sending notification"`.
4. Expected: the recipient's mailbox / notification stream received the message.
5. Actual: nothing was delivered — no `EmailService` / `NotificationService` call happened; the tracing line was the only artifact. The workflow reports success and hides the silent no-op.

## Suggested approach

1. Extend `EmailExecutor` (`services/actions/email.rs`) to hold an `Arc<EmailService>` (mirroring how other service-holding executors are shaped elsewhere in `services/`). Add a `with_service(Arc<EmailService>)` constructor; keep `new()` for a test-only null path only if a call site still needs it.
2. In `EmailExecutor::execute`, call `email_service.send_template_email(&to, &email_config.template, template_data).await`. On `Err`, return `Ok(ActionResult::failure(...))` (do not `unwrap`) so the workflow marks the action failed rather than completed. Delete the "For now, we consider the action successful" comment.
3. Same for `NotificationExecutor` (`services/actions/notification.rs`): hold `Arc<NotificationService>`, replace the `tracing::info!(...)` + success return with a `send_to_users(&targets, &title, &message, channel, priority).await` call, and route errors through `ActionResult::failure`.
4. `services/actions/mod.rs` `ActionRegistry::new()` currently takes no args. Add a `ActionRegistry::with_services(email: Arc<EmailService>, notification: Arc<NotificationService>) -> Self` that constructs the two executors with real handles; keep `new()` as the test/no-delivery path. Update `WorkflowExecutorTask` / its factory in `services/workflow_executor.rs` (grep `ActionRegistry::new` for the call sites) to use `with_services`, sourced from the existing `AppState` handles.
5. Add regression tests (IG3 — fails on `dev`, passes after fix): a mock `EmailService` / `NotificationService` trait or a `#[cfg(test)]` shim capturing `send_template_email` / `send_to_users` invocations; assert the executor calls it exactly once per action and propagates a mock `Err(...)` to `ActionResult::failure`.
6. Verify: `cargo test -p api-server services::actions::email::tests services::actions::notification::tests services::workflow_executor::tests`.

## Alternatives considered

- **Delete the two executors entirely and remove `send_email` / `send_notification` from the workflow schema** — rejected because Epic 94 (fault-alert / announcement automation) depends on these action types being available; removing them is a product regression, not a bug fix.
- **Leave the executors as stubs but return `ActionResult::failure("not implemented")` instead of `success`** — rejected because the services already exist and the wiring is a one-hour job; degrading to explicit failure would break every workflow that uses these actions in prod, while the fix restores the intended behavior.

## Root-cause trace

1. Symptom: workflow with `send_email` / `send_notification` action reports `COMPLETED`, nothing is delivered.
2. ← `services/actions/email.rs:99-105` — `execute()` returns `Ok(ActionResult::success(...))` without calling any service.
3. ← `services/actions/mod.rs:204-207` — `ActionRegistry::new()` registers these executors as the production handlers (i.e. the no-op is not a dev-only shim; it's the live path).
4. Origin: the executors were checked in as stubs with explicit "For now, we consider the action successful" / "In production, this would actually send" comments; the wiring commit never followed. `git log --diff-filter=A -- backend/servers/api-server/src/services/actions/email.rs backend/servers/api-server/src/services/actions/notification.rs` locates the original commit for the exact author/date reference.

## Test plan

- [ ] `backend/servers/api-server/src/services/actions/email.rs` — unit test: `EmailExecutor::execute` with a mock service; assert `send_template_email` was invoked with the interpolated recipient + template + data; assert `Ok(ActionResult::failure)` when the mock returns `Err`.
- [ ] `backend/servers/api-server/src/services/actions/notification.rs` — unit test: same shape for `NotificationExecutor` and `send_to_users`.
- [ ] `backend/servers/api-server/src/services/workflow_executor.rs` — integration test: run a workflow whose action is `send_email`; assert the mock captured exactly one invocation AND the workflow execution's action status is `completed` on success, `failed` on service error.
- [ ] `cargo test -p api-server services::actions::email services::actions::notification services::workflow_executor`

## Out of scope

- Migrating in-flight workflow executions to retry-on-delivery-failure (this plan only fixes the silent-no-op; retry policy is a separate epic).
- Adding a workflow-action-level dead-letter queue for repeated delivery failures.
- Any UI change to the workflow-editor to surface the delivery status.
- Refactoring the other two executors (`ApiCallExecutor`, `DelayExecutor`) — both are live paths (`ApiCallExecutor` was just hardened by PR #2707); no change needed here.

## After-merge

- Move this file to `plans/_archive/bug-api-core-email-notif-actions-noop.md`
- Mark the matching `backlog.json` row as `status: "done"`
