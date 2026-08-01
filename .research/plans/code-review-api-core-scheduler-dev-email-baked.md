# code-review-api-core-scheduler-dev-email-baked

**Vector:** bug
**Score:** 3
**Source:** Phase 1.5 rotating expert review 2026-08-01 (api-core segment); files `backend/servers/api-server/src/services/scheduler/mod.rs`, `backend/servers/api-server/src/main.rs`
**Confidence:** high

## Hypothesis
`Scheduler::new` bakes `EmailService::development()` (send-disabled) into the inner `NotificationService` used by every scheduler-triggered notification path (`notify_announcement_published`, `notify_vote_started`, `notify_vote_closed`, `notify_payment_due`). The main-side chained `with_email_service()` on `Scheduler` swaps only `self.email_service` used by the *direct* signature-reminder path, so all other scheduler-triggered email notifications silently no-op in production. The SECURITY #527-9 fix only reached the signature-reminder path; the rest still route through the baked-in `development()` transport. Fix: also swap the inner `NotificationService`'s `EmailService` (or construct the inner service lazily from the outer `email_service`) so `with_email_service` reaches both paths.

## Evidence
- `backend/servers/api-server/src/services/scheduler/mod.rs:149` — `Scheduler::new` constructs the inner `NotificationService` with `EmailService::development()` hard-wired.
- `backend/servers/api-server/src/services/scheduler/mod.rs:209` — `with_email_service` swaps `self.email_service` only; the inner `NotificationService.email_service` remains the development stub.
- `backend/servers/api-server/src/main.rs:686` — production wiring calls `.with_email_service(email_service)` under the assumption it propagates; it does not for scheduler-driven notification flows.
- `backend/servers/api-server/src/services/scheduler/votes.rs` — `notify_vote_started`/`notify_vote_closed` call the inner service's notification methods, which route Email-channel dispatches through the baked-in development transport.

## Files
- `backend/servers/api-server/src/services/scheduler/mod.rs:149`
- `backend/servers/api-server/src/services/scheduler/mod.rs:209`
- `backend/servers/api-server/src/main.rs:686`

## Dependencies


## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):**
Mode: cloud-ok

## Repro steps
1. Construct `Scheduler` the way `main.rs` does: `Scheduler::new(...).with_email_service(prod_email_service)`.
2. Invoke a scheduler-triggered path that fans out email notifications — e.g., call `close_expired_votes` on a vote whose participants are configured for Email-channel delivery.
3. Assert that `notify_vote_closed` reached the production `EmailService` transport (spy / mock).
4. Expected: production transport receives the send call. Actual: the inner `NotificationService`'s `EmailService::development()` swallows the call (`enabled=false`) — no email leaves the box, no error surfaces.

## Suggested approach
1. In `scheduler/mod.rs:149`, defer building `NotificationService` until `with_email_service` has run, OR store `email_service` on `Scheduler` and construct the inner `NotificationService` on demand from that field.
2. In `with_email_service` (`scheduler/mod.rs:209`), rebuild the inner `NotificationService` with the new `EmailService` — not just assign `self.email_service`.
3. Add a `Scheduler::with_notification_service_email` test hook (or a getter) that exposes the effective `EmailService` on the inner service so the regression test can assert it.
4. Add a regression test in `backend/servers/api-server/tests/` that constructs a `Scheduler` via the same builder pattern as `main.rs` and asserts the inner `NotificationService.email_service.enabled == true` after `with_email_service` runs.
5. Grep for other `with_*` builder methods on `Scheduler` that may have the same double-instantiation trap (`with_sms_service`, `with_push_service`, etc.); patch them consistently if present.
6. Trace call sites of `notify_announcement_published`, `notify_payment_due`, `notify_vote_started`, `notify_vote_closed` — confirm each now reaches the wired transport.
7. Update `docs/api/notifications.md` (or the closest observability runbook) with a note that scheduler-driven email is now live in prod so the operator watches for a spike.

## Alternatives considered
- **Move email service into `Scheduler` and pass it into `NotificationService` methods as an argument** — rejected because it touches every call site and inflates the diff; the builder-fix keeps the API surface constant.
- **Log a warning at scheduler startup when `NotificationService.email_service.enabled == false`** — rejected because a warning is not a fix; production would still swallow emails silently until someone reads the log.

## Root-cause trace
1. Symptom: scheduler-triggered email notifications (announcements, votes, payment-due) silently no-op in production despite `with_email_service` being wired in `main.rs`.
2. ← `Scheduler::notify_vote_closed` / `notify_announcement_published` route through `self.inner_notification_service.notify_*`, whose `email_service` field is the hard-wired `development()` stub (`backend/servers/api-server/src/services/scheduler/mod.rs:149`).
3. ← `Scheduler::with_email_service` at `scheduler/mod.rs:209` only assigns `self.email_service`; it never rebuilds or updates the inner `NotificationService.email_service`.
4. Origin: initial `Scheduler` construction in `scheduler/mod.rs` — the inner service was instantiated eagerly with `EmailService::development()` before builder chaining could propagate the production transport. Introduced when the scheduler was first refactored to own a `NotificationService` (pre-#527); the SECURITY #527-9 hardening only touched the outer signature-reminder path.

## Test plan
- [ ] New integration test in `backend/servers/api-server/tests/scheduler_email_wiring_tests.rs` that builds `Scheduler::new(...).with_email_service(prod_stub)` and asserts a scheduler-triggered path (e.g., `close_expired_votes` end-to-end) actually invokes the prod `EmailService` transport (spy captures the send call).
- [ ] Regression scenario: `notify_vote_closed` fired with participants configured for Email channel — spy must observe ≥1 send call to the prod transport.
- [ ] Command: `cd backend && cargo test -p api-server --test scheduler_email_wiring_tests`

## Out of scope
- SMS / push wiring parity checks (only if grep in step 5 turns up analogous traps — flag as follow-up, do not bundle).
- Refactor of `NotificationService` internals or `EmailService::development()` semantics.
- Backfilling any emails that were dropped in prior windows.

## After-merge
- Move this file to `plans/_archive/code-review-api-core-scheduler-dev-email-baked.md`
- Mark the matching `backlog.json` row as `status: "done"`
