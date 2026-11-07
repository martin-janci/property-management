# code-review-api-core-notification-service-push-stub

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review 2026-11-07 api-core
**Confidence:** high

## Hypothesis
`NotificationService::send_to_user` has a stub `NotificationChannel::Push` branch that only emits a `tracing::info!` "integration pending" line and never dispatches to FCM/APNs, yet the Scheduler calls that path for every scheduled notifier (announcements, votes, meter readings, payments). All scheduled push notifications silently drop in production while the request-time paths (`routes/announcements/lifecycle.rs`, `routes/voting.rs`) correctly dispatch through `state.notification_pipeline` → `PushFanoutWorker`. Replacing the Scheduler's `NotificationService` calls with the already-wired `NotificationPipeline::broadcast`/`dispatch_to_users` restores push delivery without new infrastructure.

## Evidence
- `backend/servers/api-server/src/services/notification.rs:348-356` — the `NotificationChannel::Push` match arm has a `// Push notifications would integrate with FCM/APNs` comment and only calls `tracing::info!(..., "Push notification (integration pending)")`; no FCM/APNs dispatch.
- `backend/servers/api-server/src/services/scheduler/mod.rs:491` — Scheduler calls `self.notification_service.notify_announcement_published(announcement, &target_user_ids)` which goes through the stub branch for push channels.
- `backend/servers/api-server/src/services/scheduler/mod.rs:986`, `:1066`, `:1152` and `backend/servers/api-server/src/services/scheduler/votes.rs:96`, `:256`, `:486` — every other scheduled notifier (votes, meter, payment) routes the same way.
- Request-time correct path: `backend/servers/api-server/src/routes/announcements/lifecycle.rs:138` and `backend/servers/api-server/src/routes/voting.rs:739` call `state.notification_pipeline.*` which fans out to `PushFanoutWorker` — the plumbing exists.

## Files
- `backend/servers/api-server/src/services/notification.rs`
- `backend/servers/api-server/src/services/scheduler/mod.rs`
- `backend/servers/api-server/src/services/scheduler/votes.rs`

## Dependencies

(none)

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
1. Spin up a test DB + api-server, create an announcement with `scheduled_publish_at` a few seconds in the future, and a target audience of users whose `notification_preferences` include `NotificationChannel::Push`.
2. Wait for the Scheduler's `publish_scheduled_announcements` tick (default interval — see `services/scheduler/mod.rs`).
3. Expected: a push notification is enqueued through `NotificationPipeline` → `CombinedPushAdapter` → `PushFanoutWorker` (verify via `tracing` spans and the `push_fanout_jobs` queue). Actual: only the `tracing::info!(... "Push notification (integration pending)")` log line appears from `notification.rs:348`, no job is enqueued. Same repro works for `vote_started`, `vote_closed`, `meter_reading_due`, `payment_due`.

## Suggested approach
1. Audit every `self.notification_service.notify_*` call site inside `services/scheduler/mod.rs` and `services/scheduler/votes.rs` (lines 491, 986, 1066, 1152, votes.rs:96, 256, 486 — grep for `notification_service\.notify_`) to enumerate the full migration list.
2. Give `Scheduler` access to the same `NotificationPipeline` the request-time paths use. If `Scheduler` already holds `AppState`, use `state.notification_pipeline` directly; otherwise thread it in via the existing `Scheduler::new` constructor.
3. For each scheduled notifier, replace the `NotificationService::notify_*` call with a `NotificationPipeline` call (`broadcast` or `dispatch_to_users`) that routes `NotificationChannel::Push` through `CombinedPushAdapter` → `PushFanoutWorker`, matching `routes/announcements/lifecycle.rs:138` and `routes/voting.rs:739`.
4. Keep the `in-app` and `email` channels intact — the goal is specifically to stop silently dropping push.
5. Add an integration test under `backend/servers/api-server/tests/` that triggers a scheduled announcement and asserts the `push_fanout_jobs` table (or the pipeline's enqueue counter) records the push — this would have caught the stub.
6. Once Scheduler is fully migrated, mark the `NotificationService::send_to_user` stub branch as deprecated (comment + `#[cfg(test)]`-only compile if possible) so it cannot be re-introduced from new code paths. Full removal is a follow-up (see *Out of scope*).
7. Update `services/notification.rs` doc comment to point future authors at `NotificationPipeline` as the canonical entry point.

## Alternatives considered
- **Implement real FCM/APNs dispatch inside `NotificationService::send_to_user`** — rejected because the pipeline + fanout worker already exists; building a parallel path duplicates the delivery logic and the retry/queue semantics, and would leave two push code paths with divergent behaviour.
- **Add a delivery-success metric alarm and leave the stub** — rejected because the stub silently succeeds (returns Ok with info log); the metric would need to be inverted (count expected dispatches vs. actual queue inserts). Fixing the dispatch is cheaper than building a drop-detection alarm and still leaves users without push.

## Root-cause trace
1. Symptom: production scheduled push notifications never arrive on device (announcements, votes, meters, payments).
2. ← `services/scheduler/mod.rs:491` et al. call `NotificationService::notify_announcement_published` / `notify_vote_started` / etc., which route through `NotificationService::send_to_user`.
3. ← `services/notification.rs:348-356` — the `NotificationChannel::Push` branch is a `tracing::info!` stub with a `// integration pending` comment.
4. Origin: the stub predates the `NotificationPipeline` + `PushFanoutWorker` wiring (which already works for request-time routes). When the fanout worker landed, the Scheduler's call sites were not migrated, leaving the silent-drop branch live only on the scheduled path.

## Test plan
- [ ] New integration test in `backend/servers/api-server/tests/` (e.g. `scheduler_push_dispatch.rs`) that stubs the fanout worker's enqueue counter, drives `Scheduler::publish_scheduled_announcements` with one announcement and one push-subscribed user, and asserts exactly one push job was enqueued. Would fail on `main` today.
- [ ] Mirror the test for one vote and one meter/payment path to prove the migration is complete across all scheduled notifiers.
- [ ] Run `cargo test -p api-server --test scheduler_push_dispatch` (and the existing suite_*.rs scheduler tests) locally; expect green after the change and red before.

## Out of scope
- Full removal of the `NotificationService` type. The `in-app` and `email` branches are still used elsewhere; retire the stub Push branch and plan a follow-up to consolidate everything into `NotificationPipeline` later.
- Changing `NotificationChannel` enum variants or the DB schema for notification preferences.
- SMS channel (not part of the Scheduler's push path today).
- Backfill / resend of previously-dropped push notifications; this plan restores the forward delivery path only.

## After-merge
- Open a follow-up issue to retire `NotificationService` entirely once all remaining callers have been migrated to `NotificationPipeline`.
- Watch the `push_fanout_jobs` queue depth for the first week post-merge — a big step-up confirms scheduled pushes are now firing (previously silently dropped).
- Add a Scrum-Master note about the silent-failure class so that any future "pipeline" split is accompanied by a diff-guard test (IG3-style failing-on-main) at the Scheduler boundary.
