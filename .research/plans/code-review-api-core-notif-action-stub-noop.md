# code-review-api-core-notif-action-stub-noop

**Vector:** bug
**Score:** 3
**Source:** commit a328b02 (dispatcher tier-1d 2026-08-21-api-core-tier1d.json)
**Confidence:** medium

## Hypothesis
`NotificationExecutor::execute` in the production `ActionRegistry` never delivers a notification: the real send is commented out, the executor returns `ActionResult::success` from a `tracing::info!` log, and `resolve_target` for `Role`/`Building` returns literal placeholder strings (`"role:{}"`, `"building:{}"`) instead of resolving real recipients. Every workflow-configured "notify" action is silently dropped; the recipient set on the two composite target kinds is also unusable garbage. Wire the executor to `AppState`-owned `NotificationService::send_to_users` and swap `resolve_target` to real repo lookups.

## Evidence
- `backend/servers/api-server/src/services/actions/notification.rs:159-205` — `NotificationExecutor::execute` emits `tracing::info!` and returns `Ok(ActionResult::success(...))`; the send is a comment `// notification_service.send_batch(...).await?`.
- `backend/servers/api-server/src/services/actions/notification.rs:116-131` — `resolve_target` for `Role` returns `format!("role:{}", role)` and for `Building` returns `format!("building:{}", resolved)`; comments explicitly note this is a placeholder ("In production, this would query users with the role" / "…building residents").
- `backend/servers/api-server/src/services/actions/mod.rs:205` — `NotificationExecutor::new()` is registered into the production `ActionRegistry`; consumed by `WorkflowExecutor` / `WorkflowExecutorTask` at `workflow_executor.rs:236,286`.
- `backend/servers/api-server/src/services/notification.rs:399` — `NotificationService::send_to_users(user_ids, …)` already exists; `NotificationService::new` is constructed in `scheduler/mod.rs:166`, so the wiring pattern is precedented.
- Sibling `EmailExecutor` is the same class of stub (see companion plan `code-review-api-core-email-action-stub-drops-mail`).

## Files
- `backend/servers/api-server/src/services/actions/notification.rs:116`
- `backend/servers/api-server/src/services/actions/notification.rs:159`
- `backend/servers/api-server/src/services/actions/mod.rs:190`
- `backend/servers/api-server/src/services/workflow_executor.rs:236`
- `backend/servers/api-server/src/services/notification.rs:399`

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
1. Create a workflow with a single "notify" action, target kind `Role` (e.g. `BUILDING_MANAGER`) or `Building` (any building id), channel `InApp`.
2. Trigger the workflow.
3. Assert: check the in-app notification store (or a mock `NotificationService` capture) for the delivered recipients.
4. Expected vs actual: expected notifications delivered to every user with the role (or every resident of the building); actual — the workflow marks `step_status::COMPLETED` and `set_execution_status(COMPLETED)`, but zero notifications were sent, and even the "targets" field of the recorded action output is the literal string `"role:BUILDING_MANAGER"`.

## Suggested approach
1. Add fields `notification_service: Arc<NotificationService>` and a repo handle (`Arc<OrganizationRepository>` or equivalent, whichever provides "list users with role in org" and "list residents of building") to `NotificationExecutor`; keep `NotificationExecutor::new()` for tests and add `NotificationExecutor::with_services(...)` for prod.
2. Rewrite `resolve_target` (`notification.rs:116-131`) so `Role(name)` queries the org for user_ids holding that role (RLS-scoped by `context.organization_id`), and `Building(id)` queries `UnitResident` (or the equivalent building-residents repo) for user_ids — returning `Vec<Uuid>` rather than placeholder strings. Preserve `User(id)` and `Users(ids)` shapes.
3. Update `NotificationExecutor::execute` (`notification.rs:159-205`) to call `self.notification_service.send_to_users(&user_ids, &title, &message, channel, priority, action_url).await?`, mapping the service's error into `ActionError`. Preserve the existing `tracing::info!` and add a `tracing::warn!` on `Err`.
4. Extend `ActionRegistry::with_services(email, notif, deps)` in `actions/mod.rs`; keep `ActionRegistry::new()` returning stubs for unit tests only (mark as such).
5. Thread `NotificationService` (already owned by `AppState`/scheduler) into both `WorkflowExecutor::with_config` and `WorkflowExecutorTask::spawn` at `workflow_executor.rs:236 + :286`; update `main.rs` where `AppState` is assembled.
6. Add a stub-detection regression unit test in `actions/notification.rs` using a mock/counting `NotificationService` — assert one call per `execute()` with the expected recipient list.
7. Add an integration test `tests/suites/workflow_notification_action_tests.rs` that drives a single-notify-action workflow end-to-end and asserts recipient resolution + service invocation.

## Alternatives considered
- **Resolve targets inline in `execute()` without extracting `resolve_target`** — rejected because the current separation makes each target-kind mockable and future kinds (e.g. `AnnouncementAudience`) can be added without editing the send path.
- **Return early with `ActionError::ConfigurationError` when target resolves to zero users** — rejected as behavior change; the correct default is "successful zero-send, log a `tracing::warn!`". A configuration-error surface should be gated behind an explicit workflow-config option.

## Root-cause trace
1. Symptom: workflows configured with a "notify" action complete `COMPLETED` but no notifications are delivered; the recorded `targets` output is the literal string `"role:{role}"` or `"building:{building_id}"`.
2. ← immediate cause at `backend/servers/api-server/src/services/actions/notification.rs:159-205` — `NotificationExecutor::execute` returns `Ok(ActionResult::success(...))` from a `tracing::info!` log with no `NotificationService::send_to_users` call.
3. ← upstream cause at `backend/servers/api-server/src/services/actions/notification.rs:116-131` — `resolve_target` returns placeholder strings for `Role` and `Building` instead of user_ids, so even the "targets" output is unusable.
4. ← upstream cause at `backend/servers/api-server/src/services/actions/mod.rs:205` — `NotificationExecutor::new()` (no-args stub) is what `ActionRegistry::new()` registers; `WorkflowExecutor::{with_config,spawn}` at `workflow_executor.rs:236 + :286` builds `ActionRegistry::new()` with no service injection point.
5. Origin: Epic 94 Story 94.1 landed workflow-actions scaffolding with intentionally-stubbed executors ("in production, this would use `NotificationService`"). The follow-up wiring landed for `NotificationService` in `scheduler/mod.rs` but never plumbed it through `ActionRegistry` — the executor stayed a stub.

## Test plan
- [ ] `backend/servers/api-server/src/services/actions/notification.rs` — new `#[tokio::test]` `notification_executor_calls_service` with a mock `NotificationService` that counts calls and captures recipient ids; asserts one call per `execute()` with resolved user_ids (fails today because service is not invoked and targets are placeholder strings).
- [ ] `backend/servers/api-server/src/services/actions/notification.rs` — new test `resolve_target_role_returns_user_ids` seeding an org + 2 users with a role and asserting resolution returns exactly those 2 user_ids (fails today because it returns `format!("role:{}", role)`).
- [ ] `backend/servers/api-server/tests/suites/workflow_notification_action_tests.rs` — new integration test `workflow_notify_action_delivers_via_notification_service` driving a single-notify-action workflow end-to-end.
- [ ] `cargo test -p api-server services::actions::notification` and `cargo test -p api-server --test integration workflow_notification_action_tests`.

## Out of scope
- Wiring `EmailExecutor` (tracked as `code-review-api-core-email-action-stub-drops-mail` — separate PR).
- Push/SMS delivery channels — leave `NotificationChannel::Push`/`Sms`/`All` behavior as it is today (only `InApp` needs to be wired for this PR; the others can be follow-ups once the InApp path is proven).
- Rewriting the `ActionExecutor` trait for dyn-safety or richer error taxonomy — keep the wiring minimal.

## After-merge
- Move this file to `plans/_archive/code-review-api-core-notif-action-stub-noop.md`
- Mark the matching `backlog.json` row as `status: "done"`
