# code-review-api-core-email-notification-action-executors-are-stubs

**Vector:** bug
**Score:** 3
**Source:** Phase 1.5 rotating expert review 2026-08-09 (api-core segment)
**Confidence:** medium

## Hypothesis
Workflow actions of type `send_email` and `send_notification` execute against `EmailExecutor` / `NotificationExecutor` stubs that only emit `tracing::info!` and return `ActionResult::success` — no SMTP transport, no push transport is ever invoked. Real `EmailService` (lettre-based) and `NotificationService` exist in `backend/servers/api-server/src/services/{email,notification}.rs` but are never wired into these executors. Net effect: every user-configured "email tenant on fault", "notify manager on vote" workflow logs success while delivering nothing.

## Evidence
- `backend/servers/api-server/src/services/actions/email.rs:87` — `EmailExecutor::execute` `tracing::info!`s the email and returns `ActionResult::success` without invoking any transport
- `backend/servers/api-server/src/services/actions/notification.rs:183` — `NotificationExecutor::execute` `tracing::info!`s and returns `ActionResult::success`; real `NotificationService` never called
- `backend/servers/api-server/src/services/actions/mod.rs:204` — `ActionRegistry::new()` registers both stubs under `send_email` / `send_notification` action types
- `backend/crates/db/src/models/workflow_templates.rs` — ships shipped workflow templates that use `send_email` and `send_notification` action types

## Files
- `backend/servers/api-server/src/services/actions/email.rs`
- `backend/servers/api-server/src/services/actions/notification.rs`
- `backend/servers/api-server/src/services/actions/mod.rs`
- `backend/servers/api-server/src/services/email.rs`
- `backend/servers/api-server/src/services/notification.rs`

## Dependencies

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
1. Configure a workflow whose action list includes a `send_email` action (any target address in the request body).
2. Trigger the workflow (via API or scheduler) and inspect the resulting `ActionResult` — status is `success`, `sent_at` is populated, `duration_ms` is set.
3. Check the mail transport (SMTP log, mailhog inbox in dev, or lettre message queue): no message was ever sent.
4. Expected: an email is delivered via `EmailService::send_template_email` (or the correct variant). Actual: no delivery attempt was made.

## Suggested approach
1. Inject `Arc<EmailService>` and `Arc<NotificationService>` into `ActionRegistry` at construction (extend `ActionRegistry::new()` in `mod.rs:200-210` to accept them and pass through to executors).
2. In `EmailExecutor::execute` (`email.rs:63-113`), after template substitution and validation, call the correct `EmailService` method based on `email_config.template` (`send_verification_email`, `send_password_reset_email`, `send_invitation_email`, `send_notification_email`, `send_template_email`) — map the enum → method dispatch table.
3. In `NotificationExecutor::execute` (`notification.rs:161-210`), after target resolution, call `NotificationService::send_to_users(&targets, &title, &message, channel, priority, action_url)` — the method already exists at `notification.rs:399`.
4. Preserve the `duration_ms` measurement; on transport error, return `ActionError::TransportError(err)` (add variant to `actions::error::ActionError` if absent) rather than swallowing.
5. Update `ActionRegistry::new()` callers (`WorkflowExecutorTask::new` in `workflow_executor.rs:868-888` and any test constructors) to plumb the services.
6. Add a compile-time no-fallback assertion: neither executor should have any code path returning `ActionResult::success` without a transport call.

## Alternatives considered
- **Leave the stubs and add a feature flag `WORKFLOW_ACTIONS_LIVE`** — rejected because the current stubs report `success` unconditionally, which is a silent lie; a flag would just hide the bug behind config. The fix is to make the executors real.
- **Route through a fresh `WorkflowNotifier` seam mirroring PR #2696's `inquiry_notifier`** — rejected because `EmailService`/`NotificationService` already exist and are correctly abstracted. A new seam duplicates the abstraction for no gain.

## Root-cause trace
1. Symptom: workflow-configured emails/push notifications never arrive; audit log shows `ActionResult::success` for every dispatch.
2. ← `EmailExecutor::execute` at `email.rs:87` returns `Ok(ActionResult::success(...))` after only `tracing::info!` — no transport call.
3. ← `NotificationExecutor::execute` at `notification.rs:183` mirrors the same shape.
4. ← `ActionRegistry::new()` at `mod.rs:204` registers both stubs; no code path swaps them for a real implementation.
5. Origin: workflow-actions module scaffold — the executors were shipped as `// In production, this would use EmailService` placeholders and never completed.

## Test plan
- [ ] `backend/servers/api-server/src/services/actions/email.rs` add `#[tokio::test]` `execute_email_action_invokes_email_service` — inject a mock `EmailService` (via a trait), execute the executor with a valid config, assert the mock's `send_template_email` was called with the expected `to`/`subject`/`template_data`. Test MUST fail today (executor never touches the service) and pass after the fix.
- [ ] `backend/servers/api-server/src/services/actions/notification.rs` add `execute_notification_action_invokes_notification_service` — same shape against `NotificationService::send_to_users`.
- [ ] Add integration test `backend/servers/api-server/tests/workflow_actions_deliver.rs` — spin an in-memory transport, run a workflow with a `send_email` step, assert the transport received the message.
- [ ] Run: `cargo test -p api-server services::actions --lib` locally.

## Out of scope
- Migrating existing tracing calls to structured logs.
- Adding a workflow-action retry policy (separate concern; belongs on a workflow-runtime plan).
- Extending `NotificationService` to new channels — the existing method set is enough for the two executors to be real.

## After-merge
- Move this file to `plans/_archive/code-review-api-core-email-notification-action-executors-are-stubs.md`
- Mark `backlog.json` row `code-review-api-core-email-notification-action-executors-are-stubs` as `status: "done"`
