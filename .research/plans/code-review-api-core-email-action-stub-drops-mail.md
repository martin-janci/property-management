# code-review-api-core-email-action-stub-drops-mail

**Vector:** bug
**Score:** 3
**Source:** commit a328b02 (dispatcher tier-1d 2026-08-21-api-core-tier1d.json)
**Confidence:** medium

## Hypothesis
`EmailExecutor::execute` in the production `ActionRegistry` never sends an email — the real send path is commented out and the method returns `ActionResult::success` from a `tracing::info!` log. Every workflow-configured "send email" action is silently dropped while the workflow reports success and marks the step COMPLETED. This is user-facing data loss on Epic 94 (workflow actions): a manager configuring "email the tenant on payment overdue" observes green completion but no mail is delivered. Wiring `EmailExecutor` to the already-constructed `AppState::email_service` and calling `send_template_email` closes the gap.

## Evidence
- `backend/servers/api-server/src/services/actions/email.rs:86-104` — `EmailExecutor::execute()` only emits `tracing::info!`; the send is a comment `// email_service.send_template_email(&to, &template, template_data).await?`, then returns `Ok(ActionResult::success(...))`.
- `backend/servers/api-server/src/services/actions/mod.rs:204` — `EmailExecutor::new()` is registered into the production `ActionRegistry` (`ActionRegistry::new()`), consumed by `WorkflowExecutor`/`WorkflowExecutorTask`.
- `backend/servers/api-server/src/services/workflow_executor.rs:236,286` — both `WorkflowExecutor` construction sites build `ActionRegistry::new()` with no services.
- `backend/servers/api-server/src/services/email.rs:240` — `EmailService::send_template_email(to, template, data)` already exists and is used elsewhere (`state.rs:426` stores `email_service: EmailService` on `AppState`; `main.rs:528` constructs it).
- Sibling `NotificationExecutor` is the same class of stub (see companion plan `code-review-api-core-notif-action-stub-noop`).

## Files
- `backend/servers/api-server/src/services/actions/email.rs:86`
- `backend/servers/api-server/src/services/actions/mod.rs:190`
- `backend/servers/api-server/src/services/workflow_executor.rs:236`
- `backend/servers/api-server/src/services/email.rs:240`

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
1. Create a workflow with a single "send_email" action (recipient: any test address; template: `payment_reminder`; data: dummy fields).
2. Trigger the workflow (any trigger).
3. Assert: the workflow execution reaches `COMPLETED`; check MailHog / captured `EmailService::send_template_email` calls.
4. Expected vs actual: expected the email to appear in the SMTP capture; actual — no send occurs; workflow execution shows `step_status::COMPLETED` and `set_execution_status(COMPLETED)`, yet the `EmailService` mock/capture recorded zero calls.

## Suggested approach
1. Add an `email_service: Arc<EmailService>` field to `EmailExecutor` in `actions/email.rs`; keep a default `EmailExecutor::new()` for tests (mocking the service via a small trait or `Arc`) — the production path uses `EmailExecutor::with_service(email_service)`.
2. Extend `ActionRegistry` with `ActionRegistry::with_services(email: Arc<EmailService>, notif: Arc<NotificationService>) -> Self` in `actions/mod.rs`; keep `ActionRegistry::new()` returning stubs for unit tests, and mark the stub path with `#[cfg(test)]` or a doc note that it is test-only.
3. Update `WorkflowExecutor::with_config` and `WorkflowExecutorTask::spawn` in `workflow_executor.rs:236` + `:286` to accept and pass services through from `AppState` (both construction sites); thread the services from `main.rs` where `AppState` is assembled.
4. In `EmailExecutor::execute`, delete the commented-out block and call `self.email_service.send_template_email(&to, template_name, template_data).await?`; map `EmailError` to `ActionError::ExternalService(...)` so a real failure aborts the workflow step (or applies retry per config), not silently succeeds.
5. Preserve the existing `tracing::info!` (with `workflow_id`, `execution_id`, `to`, `subject`, `template`) but move it to fire after the send succeeds; add a `tracing::warn!` on `Err`.
6. Add a stub-detection regression test in `actions/email.rs` that constructs `EmailExecutor::with_service` against a mock `EmailService` (or an `Arc<dyn EmailSender>` trait if the concrete type is not mockable) and asserts the mock was invoked exactly once per `execute()` call — failing today because `execute()` never calls the service.
7. Add an integration test in `tests/suites/workflow_email_action_tests.rs` (new file) that runs a workflow with a single email action end-to-end through `WorkflowExecutorTask` and asserts the email service received the send.

## Alternatives considered
- **Pass `EmailService` via `ActionContext` per-execution** — rejected because `ActionContext` is per-workflow-run scratch state (variables + trigger event); shared services belong in the executor's registry (constructed once), not in per-call context.
- **Introduce a thread-local / OnceLock `EmailService` singleton** — rejected because the existing `AppState` pattern already owns lifetime + wiring, and a global would defeat test-isolation for the new regression test.

## Root-cause trace
1. Symptom: workflows configured with a "send email" action complete `COMPLETED` but no email is delivered — user-visible data loss.
2. ← immediate cause at `backend/servers/api-server/src/services/actions/email.rs:86-104` — `EmailExecutor::execute` returns `Ok(ActionResult::success(...))` from a `tracing::info!` log; the send call is commented out.
3. ← upstream cause at `backend/servers/api-server/src/services/actions/mod.rs:204` — `EmailExecutor::new()` is registered into production `ActionRegistry` and `WorkflowExecutor::{with_config,spawn}` builds `ActionRegistry::new()` at workflow_executor.rs:236 + :286 with no `EmailService` injection point.
4. Origin: Epic 94 Story 94.1 landed the workflow-actions scaffolding with placeholder executors intended for a follow-up integration pass; the follow-up wired the `EmailService` into `AppState` and `main.rs` but never plumbed it through `ActionRegistry` to the executor. The stub has been production-live since.

## Test plan
- [ ] `backend/servers/api-server/src/services/actions/email.rs` — new `#[tokio::test]` `email_executor_calls_email_service` using a mock/counting `EmailService` (fails today because the service is not invoked).
- [ ] `backend/servers/api-server/tests/suites/workflow_email_action_tests.rs` — new integration test `workflow_send_email_action_delivers_via_email_service` that drives a single-email-action workflow through `WorkflowExecutorTask` and asserts one send.
- [ ] `cargo test -p api-server services::actions::email` and `cargo test -p api-server --test integration workflow_email_action_tests` (or the crate's standard integration target).

## Out of scope
- Wiring `NotificationExecutor` (tracked as `code-review-api-core-notif-action-stub-noop` — separate PR to keep the diff reviewable).
- Overhauling `ActionExecutor` trait ergonomics (dyn-safety, async-trait, error taxonomy) — keep the wiring diff minimal.
- Retry-policy tuning for `EmailError::TransportError` — reuse whatever `WorkflowExecutor` already does with `ActionError`.

## After-merge
- Move this file to `plans/_archive/code-review-api-core-email-action-stub-drops-mail.md`
- Mark the matching `backlog.json` row as `status: "done"`
