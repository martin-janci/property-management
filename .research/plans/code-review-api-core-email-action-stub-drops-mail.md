# code-review-api-core-email-action-stub-drops-mail

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review [api-core] 2026-08-21 — `.research/signals/2026-08-21-api-core-tier1d.json`
**Confidence:** medium

## Hypothesis
`EmailExecutor::execute()` at `backend/servers/api-server/src/services/actions/email.rs:63-113` parses config, substitutes template variables, validates the recipient — then only calls `tracing::info!` and returns `ActionResult::success(...)`. The real send is commented out (`// email_service.send_template_email(...)`). The executor is registered into the production `ActionRegistry` (`backend/servers/api-server/src/services/actions/mod.rs:204`), which `WorkflowExecutorTask` builds unconditionally at `backend/servers/api-server/src/services/workflow_executor.rs:236` and `:286`. Consequence: every workflow "send email" action silently succeeds while the recipient receives nothing. `EmailService` (`backend/servers/api-server/src/services/email.rs:91`) already exists on `AppState` (`state.rs:426`) with concrete methods (`send_template_email`, `send_html_email`, `send_notification_email`). The smallest correct change is to plumb `Arc<EmailService>` from `AppState` through `WorkflowExecutorTask::action_registry` and have `EmailExecutor::execute` call the real service, keeping the current success-shape and observability.

## Evidence
- `backend/servers/api-server/src/services/actions/email.rs:86-104` — comment `// Simulate email sending (in production, integrate with EmailService)` + `Ok(ActionResult::success(...))` immediately after the `tracing::info!`.
- `backend/servers/api-server/src/services/actions/mod.rs:203-205` — `registry.register(Box::new(EmailExecutor::new()));` inside `ActionRegistry::new()` — no dependency injection.
- `backend/servers/api-server/src/services/workflow_executor.rs:236` and `:286` — `action_registry: ActionRegistry::new()` on both the sync-configured and spawned execution paths — production code, not a test double.
- `backend/servers/api-server/src/services/email.rs:91,102,240` — `pub struct EmailService`, `impl EmailService`, `pub async fn send_template_email(...)`.
- `backend/servers/api-server/src/state.rs:426` — `pub email_service: EmailService` already lives on `AppState`, and is cloned at `:665`/`:770` — trivial to hand into the executor.

## Files
- `backend/servers/api-server/src/services/actions/email.rs`
- `backend/servers/api-server/src/services/actions/mod.rs`
- `backend/servers/api-server/src/services/workflow_executor.rs`
- `backend/servers/api-server/src/services/email.rs`
- `backend/servers/api-server/src/state.rs`

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
1. In `backend/servers/api-server/src/services/actions/email.rs::tests::test_email_executor_success` (existing test, line 145) assert that a spy `EmailService` (or a counter tracked via a `MockEmailBackend` test double) recorded exactly one send call for the resolved `to`/`subject`.
2. Expected on `dev`: the assertion fails because the executor never invokes any service — it only logs and returns success.
3. Expected after fix: assertion passes; `send_template_email` (or `send_html_email` if `email_config.template.is_none() && email_config.html`) was called once with the substituted recipient and subject.

## Suggested approach
1. Add an `Arc<EmailService>` field to `EmailExecutor` (`services/actions/email.rs:37-40`). Introduce `EmailExecutor::with_service(service: Arc<EmailService>) -> Self` and keep `EmailExecutor::new()` returning an executor with a service constructed from a `NullEmailBackend` (log-only, no SMTP) so existing tests and any missing-config path still compile — but the *production* wiring must call `with_service`.
2. In `services/actions/email.rs::execute`, after the recipient validation, dispatch to the real service:
   - If `email_config.template.is_some()` → `self.email_service.send_template_email(&to, template, template_data).await`.
   - Else if `email_config.html` → `self.email_service.send_html_email(&to, &subject, &body).await`.
   - Else → `self.email_service.send_notification_email(&to, &subject, &body).await` (the existing plain-text path).
   Map any `EmailError` into `ActionError::ExecutionError` — do NOT swallow.
3. Extend `ActionRegistry::new()` in `services/actions/mod.rs:198` with a sibling `ActionRegistry::with_services(email: Arc<EmailService>, notif: Arc<NotificationService>) -> Self` that constructs the executors with real services. Leave `new()` (which currently uses `EmailExecutor::new()`) for the workflow-executor unit tests at `services/actions/mod.rs:277`; production code moves to `with_services`.
4. Update the two production call sites in `services/workflow_executor.rs:236` and `:286` to call `ActionRegistry::with_services(state.email_service.clone(), state.notification_service.clone())`. Thread `state.email_service` into `WorkflowExecutorTask` where needed. The `Workflow`/`AppState` wiring in `state.rs:665,770` already clones `email_service`; no new field on `AppState` is required.
5. Add a `#[cfg(test)] impl EmailService { pub fn spy() -> (Self, Arc<AtomicUsize>) }` helper (or a `MockEmailBackend` that increments a shared counter). Update `test_email_executor_success` (`email.rs:145`) and `test_email_executor_with_template_substitution` (`:169`) to construct the executor via `EmailExecutor::with_service(Arc::new(EmailService::spy(...)))`, then assert the counter incremented — capturing the "drops mail" regression.
6. Run `cargo fmt --all && cargo clippy -p api-server --all-targets -- -D warnings && cargo test -p api-server actions::email` and confirm the new assertions fail on `dev` (baseline) and pass on the branch.
7. Add a short line to `services/actions/email.rs` module doc noting the executor now dispatches through `EmailService` (delete the "log the email for now" comment).

## Alternatives considered
- **Change the executor to fail-closed (`ActionError::ExecutionError("not implemented")`)** — rejected because it would immediately break every workflow already relying on the "send email" step succeeding (workflow authors have been shipping against the silent-success shape); the fix must actually send, not remove the capability.
- **Introduce a global `EmailBackend` trait and inject via a `once_cell::sync::OnceCell`** — rejected because `EmailService` already exists on `AppState` and is cloned into other services in the same file; a parallel injection singleton would fork the dependency graph and complicate tests without adding testability that `Arc<EmailService>` (with a spy) doesn't already give us.

## Root-cause trace
1. Symptom: users who trigger a workflow with a `send-email` action see the run go green in the workflow-execution UI but never receive the email.
2. ← `EmailExecutor::execute()` at `email.rs:86-104` returns `Ok(ActionResult::success(...))` after only calling `tracing::info!`; no `EmailService` invocation.
3. ← `EmailExecutor::new()` at `:44` stores no dependency because `ActionRegistry::new()` at `mod.rs:198-210` doesn't take one either.
4. ← Both `WorkflowExecutorTask` build sites at `workflow_executor.rs:236` and `:286` call `ActionRegistry::new()` unconditionally — production and tests share the log-only registry.
5. Origin: initial workflow-actions scaffolding (Epic 94, Story 94.1 — see `email.rs:1-3` module doc) shipped the executor with a `// Simulate email sending (in production, integrate with EmailService)` comment and no follow-up plumbing.

## Test plan
- [ ] `backend/servers/api-server/src/services/actions/email.rs::tests::test_email_executor_success` — extend to assert send-count increments (currently only asserts `result.success`).
- [ ] `backend/servers/api-server/src/services/actions/email.rs::tests::test_email_executor_with_template_substitution` — same extension, assert the recipient the spy saw matches the template-substituted address.
- [ ] New test `test_email_executor_maps_send_failure` — configure the spy service to return an error; assert `execute` returns `ActionError::ExecutionError`, not silent success.
- [ ] `cargo test -p api-server actions::email` — all green on the branch; baseline capture on `dev` shows the two extended tests failing.
- [ ] `cargo clippy -p api-server --all-targets -- -D warnings` clean.

## Out of scope
- Sibling `code-review-api-core-notif-action-stub-noop` (score 3, open) — same pattern for `NotificationExecutor`; leave for a follow-up so this PR stays reviewable. This plan's `ActionRegistry::with_services` signature deliberately reserves the `notif` slot so the follow-up PR can flip `NotificationExecutor` without another registry-signature change.
- `code-review-api-core-voice-contact-mgr-fake-email` (score 2, open) — different site (`voice_commands.rs`), different backend surface; separate plan when it clears the readiness bar.
- Any change to the `EmailService` API surface or new email templates.

## After-merge
- Move this file to `plans/_archive/code-review-api-core-email-action-stub-drops-mail.md`
- Mark the matching `backlog.json` row as `status: "done"`
