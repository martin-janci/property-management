# code-review-api-core-notif-action-stub-noop

**Vector:** bug
**Score:** 3
**Source:** api-core segment review 2026-08-21 (Phase 1.5 code-review slice)
**Confidence:** medium

## Hypothesis
`NotificationExecutor::execute()` is the sibling stub to `EmailExecutor`: it logs one `tracing::info!` and returns `ActionResult::success(...)` while never touching `NotificationService`. On top of that, `resolve_target()` for `Role` and `Building` returns literal placeholder strings (`format!("role:{}", role)` / `format!("building:{}", resolved)`) instead of expanding to real user ids, so even once the executor is wired the recipient set would be nonsense. Fix is to inject `Arc<NotificationService>` (and the DB pool the target-expansion query needs) into `NotificationExecutor`, resolve `Role` / `Building` targets to real user ids, then call the existing batch-send API.

## Evidence
- `backend/servers/api-server/src/services/actions/notification.rs:159-209` — `execute()` only calls `tracing::info!` then returns `ActionResult::success(...)`; the real send is a comment (`// notification_service.send_batch(...)`).
- `backend/servers/api-server/src/services/actions/notification.rs:116-131` — `resolve_target()` returns literal strings for `Role`/`Building` instead of expanding to user ids (comments explicitly note "In production, this would query users with the role" / "…building residents").
- `backend/servers/api-server/src/services/actions/mod.rs:205` — `registry.register(Box::new(NotificationExecutor::new()));` inside `ActionRegistry::new()` — production wiring.
- `backend/servers/api-server/src/services/workflow_executor.rs:236,286,894,951-972` — same false-COMPLETED flow as the email stub: `ActionRegistry::new()` is instantiated in the prod `WorkflowExecutorTask` constructors and a successful stub `execute()` stamps step + execution as COMPLETED.
- `backend/servers/api-server/src/services/notification.rs:135-146,168-169` — `NotificationService` exists with a `new(...)` and `development()` constructor and is already used by scheduler; wiring exists, it's just not plugged in here.

## Files
- `backend/servers/api-server/src/services/actions/notification.rs`
- `backend/servers/api-server/src/services/actions/mod.rs`
- `backend/servers/api-server/src/services/workflow_executor.rs`
- `backend/servers/api-server/src/services/notification.rs`

## Dependencies
- code-review-api-core-email-action-stub-drops-mail

## Required capabilities
- [x] C1 — Systematic debugging (bug — silent-success data loss)
- [x] C2 — Seed data (workflow + users bound to a role and a building)
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):** no C4, no C5 → `Mode: cloud-ok`

Mode: cloud-ok

## Repro steps
1. Seed a workflow whose action is `{"type":"notify","target":{"kind":"role","role":"manager"},"title":"...","message":"..."}` and attach it to a trigger.
2. Fire the trigger so `WorkflowExecutorTask` runs the action end to end.
3. Expected: the workflow step reports success **and** `NotificationService::send_batch` is called exactly once with the resolved user-id list (assert via a spy). Actual today: step reports success, `tracing::info!` fires with `targets = ["role:manager"]`, no `NotificationService` call ever happens.

## Suggested approach
1. Add `notification_service: Arc<NotificationService>` and `db: PgPool` fields to `NotificationExecutor`. Update `NotificationExecutor::new(notification_service, db)`; keep a `development()` for tests.
2. Piggy-back on the `ActionRegistry` plumbing added by `code-review-api-core-email-action-stub-drops-mail`: extend the same builder call to `ActionRegistry::new(email_service, notification_service, db)`.
3. Rewrite `resolve_target()` (`services/actions/notification.rs:116-131`) as `async fn resolve_target(...) -> Result<Vec<UserId>, ActionError>`:
   - `Role(role)` → `SELECT user_id FROM organization_members WHERE org_id = $1 AND role = $2` (bound to `context.org_id`).
   - `Building(building_id)` → the existing "building residents" query the scheduler already runs; extract into a shared helper if it isn't already.
   - Keep `Users(ids)` as passthrough.
4. In `execute()`, replace the `tracing::info!`-only body with:
   ```rust
   let user_ids = self.resolve_target(&notif_config.target, context).await?;
   self.notification_service
       .send_batch(&user_ids, &title, &message, notif_config.channel, notif_config.priority)
       .await
       .map_err(|e| ActionError::ExecutionFailed(e.to_string()))?;
   ```
5. Preserve the current substitution logic (`services/actions/notification.rs:172-177`) — `title`, `message`, `action_url` still flow through `context.substitute_template(...)` before dispatch.
6. Map `NotificationService` errors to `ActionError::ExecutionFailed(...)` so the step transitions to `step_status::FAILED` at `services/workflow_executor.rs:894` instead of silently succeeding.

## Alternatives considered
- **Leave target expansion to `NotificationService` and just pass the untyped `Role` / `Building` down** — rejected because `NotificationService::send_batch` today takes a user-id slice; widening its shape doubles the change and forces every caller (scheduler, etc.) to opt in.
- **Fire-and-forget spawn via `tokio::spawn`** — rejected because it prevents the error from surfacing to the workflow-step status; a workflow that reports COMPLETED while notifications silently failed is the exact bug this plan removes.

## Root-cause trace
1. Symptom: workflow "notify" action reports COMPLETED; no push / notification arrives; the resolved-target log line shows `["role:manager"]` instead of user ids.
2. ← `NotificationExecutor::execute()` at `backend/servers/api-server/src/services/actions/notification.rs:159-209` returns `Ok(ActionResult::success(...))` without invoking `NotificationService`.
3. ← `resolve_target()` at `backend/servers/api-server/src/services/actions/notification.rs:116-131` returns placeholder strings instead of querying users.
4. ← `WorkflowExecutorTask` at `backend/servers/api-server/src/services/workflow_executor.rs:894,951-972` stamps the step + execution as COMPLETED on the returned `Ok(...)`.
5. Origin: the executor was written as a scaffold ("In production, this would actually send notifications via the notification service") and never wired to `NotificationService` even after `NotificationService` landed for the scheduler.

## Test plan
- [ ] `backend/servers/api-server/tests/` — integration test that drives `WorkflowExecutorTask` end-to-end with a spy `NotificationService` and a seeded org whose `manager` role contains two users; assert `send_batch` was called exactly once with a user-id list of length 2.
- [ ] Regression: `Users(ids)` target still passes through untouched (behaviour preserved).
- [ ] Command: `cargo test -p api-server workflow_notify_action`

## Out of scope
- Wiring `EmailExecutor` to `EmailService` — sibling plan `code-review-api-core-email-action-stub-drops-mail`.
- Adding a queued / retry-with-backoff notification path.
- Refactoring the `ActionTarget` enum or the underlying `notifications` schema.
- Anything under `voice_commands.rs` — different subsystem, tracked separately.

## After-merge
- Move this file to `plans/_archive/code-review-api-core-notif-action-stub-noop.md`
- Mark the matching `backlog.json` row as `status: "done"`
