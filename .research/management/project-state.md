# PPT Project State

_Generated: 2026-08-08 12:21 UTC — routine Phase 1.6 rotating slot (pm-qa). Coverage `scan_kind=upkeep`; pm_cursor idx 3 → 4 (pm-qa → pm-devops next), coverage_cursor idx 5 → 6 (epic-80 re-checked next; this run refreshed epic re-check schedule only). Payload: `buffer-low: claimable=1/72 — refill planner` — routine added 9 backlog vectors + promoted 1 security plan._

## Executive summary

- **Delivery still 47/49 stories done, 2 partial** (Epic 84-1 direct-to-S3 upload wiring, 84-2 sign page). Between 2026-08-07 and 2026-08-08 the merge pipeline shipped **9 more PRs** — heavy dispatcher pass on backend hardening + churn-hotspot refactors.
- **Shipped this window:** SSRF DNS-rebinding TOCTOU close (#2710, closes #2703), scheduler `notified_at` watermark + retrying dispatch (#2714, closes #2612), platform-admin PATCH/GET settings endpoint (#2716), admin mobile-config PATCH endpoint (#2717), layout webhook HMAC parity regression (#2718), inquiry-notifier routing for anonymous POSTs (#2719), reports helpers.rs extraction (#2720), acquire_public_conn dedupe in layout-tenant (#2721), auth error-boilerplate dedupe (#2715).
- **Migration collision recovery:** three PRs (#2714, #2716, #2717) all initially picked migration `00228`; manual renumbering to `00229`/`00230` unblocked them. Same-day parallel-implementer race with no cross-branch coordination — pm-qa surfaced it as a repeatable risk this run.
- **New security finding (promoted immediately via fast-track):** rotating Rust expert review of `api-core` found MFA fail-open in `routes/auth/mod.rs:388` — `if let Ok(Some(mfa_record))` collapses the Err arm into "skip MFA", so a transient DB error during 2FA lookup bypasses the second factor entirely when no org policy demands MFA. Score 3 / confidence high / vector security. Plan: `plans/code-review-api-core-auth-mfa-fail-open.md`.
- **Buffer refill:** dispatcher tier1d generator added 5 code-review-findings today (2 api-core, 1 api-handlers, 2 mobile-native-kmp) but did not fold them into `backlog.json`; this run fixed that — all 5 now open backlog rows for the dispatcher to claim next cycle.
- **Open PRs unchanged:** the 3 accounting-trio PRs (#2555, #2558, #2559) still 10+ days stalled with no reviewer engagement — pm-qa flagged for explicit disposition decision.

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A"** · **3/5 epics done in sprint window**; extended-scope epics all done in `coverage.json` except 84 (3/5, 2 partial).

| Epic | Sprint status | Coverage status (13 epics) |
|---|---|---|
| 6 — Announcements & Communication | in-progress | 6/6 stories done |
| 7A — Basic Document Management | in-progress | 5/5 stories done |
| 8A — Basic Notification Preferences | done | 3/3 stories done |
| 10A — OAuth Provider Foundation | done | 3/3 stories done |
| 10B — Platform Administration | in-progress | 7/7 stories done |
| 80 — Dispute Resolution | partial | 3/3 stories done in coverage; sprint-status still says partial |
| 84 — Documents / e-signature | extended | 3/5 done, 2 partial (84-1 direct-S3, 84-2 sign page) |
| 79 / 81 / 82 / 83 / 85 / 8a / 9 | extended | all done |

## Shipped since last run (9 PRs)

- **#2710** — gh-issue-2703 close SSRF DNS-rebinding TOCTOU in workflow `api_call.rs` (connect-time SSRF-guard DNS resolver + validate_external_url redirect policy + IG3 rebind regression test) [SECURITY HIGH]
- **#2714** — decouple scheduled notification dispatch from publish/activate/close (announcements + votes `notified_at` watermarks, migration 00228, closes #2612) [DURABILITY]
- **#2715** — dedupe auth handler error boilerplate (private AuthError alias + err_response builder, -83 lines)
- **#2716** — add PATCH/GET `/api/v1/platform-admin/settings` (migration 00229 after collision-renumber; unblocks admin-web platform Save)
- **#2717** — add PATCH/GET `/api/v1/admin/mobile-config` (migration 00230; unblocks admin-web mobile-config Save)
- **#2718** — layout webhook HMAC body-binding regression test (asserts sig over different body is rejected, mirrors esignature tamper-rejection suite)
- **#2719** — route anonymous inquiry POSTs through InquiriesHandler notifier seam (fixes never-notified realtors on `send_contact_message` / `request_viewing`)
- **#2720** — extract reports helpers.rs from routes/reports/mod.rs (2460→2185 lines; churn-hotspot refactor)
- **#2721** — extract acquire_public_conn helper in routes/layout/tenant.rs (mirrors #2713 dedupe pattern)

## What's next

Ranked from `action-list.json` (top 5, deduped by owner where possible):

1. **[HIGH pm-security]** Fix MFA fail-open on transient DB error in login handler (`code-review-api-core-auth-mfa-fail-open` — plan promoted this run).
2. **[HIGH pm-qa]** Add regression test for vote scheduler `notified_at` watermark: simulate dispatch failure, assert watermark stays NULL so next tick retries.
3. **[HIGH pm-qa]** Add notifier/fanout assertion test for anonymous inquiries now routed through InquiriesHandler (#2719).
4. **[MED pm-qa]** Get reviewer engagement or explicit defer/close decision on accounting-trio PRs #2555/#2558/#2559 (10+ days stalled).
5. **[MED pm-qa]** Confirm/name the DNS-rebinding TOCTOU regression test tied to #2710/#2703 (not just general SSRF allow/deny unit tests).

## Blockers

- **Buffer starvation** — dispatcher `claimable=1/72` triggered this run's payload; refilled with 9 new backlog rows + 1 promoted plan. Next dispatcher cycle should pull the fresh rows.
- **Accounting-trio review stall** — no reviewer engagement in 10+ days; PM decision needed.

## Role focus today

- **pm-qa** (rotating slot this run) — surfaced 4 test-coverage risks + 5 next actions.
- **Rotating expert review (Rust)** on `api-core` segment — 1 finding (MFA fail-open, HIGH), promoted immediately.
- **Dispatcher tier1d generator** — 5 pre-existing code-review findings from today's runs (api-core delay overflow, api-core quiet-hours fail-open, api-handlers reports silent-zero, mobile-native-kmp create-listing stub, mobile-native-kmp ApiClient dead code) folded into backlog.

## Per-role summary (roles that ran this run)

- **pm-qa** — coverage is generally solid but two of today's shipped fixes (vote-scheduler `notified_at` retry, anonymous-inquiry notifier fanout) have no dedicated regression test; SSRF TOCTOU fix and migration renumbering check clean; accounting-trio PRs remain an unresolved process signal. See `roles/pm-qa.md`.
