# Decision log — PPT delivery

> Maintained by `pm-scrum-master`. Append decisions; never delete. Format below.

## Decisions made

### DEC-001 — Epic 2B build order: land infra first, formally defer dependent slices

**Date:** 2026-05-24
**Owner:** pm-tech-lead
**Status:** decided

**Decision:**
Land Epic 2B notification infrastructure (event bus, WebSocket server, push/FCM/APNs, email dispatch) **before** picking up:
- Epic 6 stories 6.2+ (announcement viewing/comments/pinning that trigger notifications on publish)
- Epic 8A.2 notification dispatch logic
- Epic 8A.3 WebSocket-based preference sync

The formally deferred slices are:
- `6-2-announcement-viewing-acknowledgment`
- `6-3-announcement-comments-discussion`
- `6-4-pinned-announcements`
- `6-5-direct-messaging` (requires WS realtime)
- `8a-2-critical-notification-override` (dispatch half)
- `8a-3-notification-preference-sync` (WS half)

**Rationale:**
1. **Zero WS infrastructure exists.** No WebSocket server (Axum upgrade handler, connection registry, pub/sub bridge) exists in `backend/`. Only type definitions and notification preference models are present. Building 8A.3 or 6-5 real-time features now means writing throw-away stubs that will be replaced when 2B lands — a confirmed high-probability, high-impact rework risk (risks.json `pm-tech-lead-notification-ws-foundation-rework`).
2. **Sprint notes already acknowledge the deferrals.** `sprint-status.yaml` explicitly records "Story 8A.2 notification dispatch logic deferred — requires Epic 2B" and "Story 8A.3 WebSocket sync deferred — requires WebSocket infrastructure". This decision formalises what the sprint already implies.
3. **Feature-flag alternative is not cost-free.** Freezing dependent slices behind feature flags requires flag infrastructure that doesn't exist either. Landing 2B directly is cleaner and lower net effort than building a flag system first.
4. **No functional regression.** The deferred stories have no shipped frontend yet (all `apiStatus: stub` or `ready-for-dev`). Deferring them does not break anything currently live.
5. **2B unblocks multiple epics at once.** Once 2B lands it unblocks 8A.2, 8A.3, 6-2 through 6-5, and the future direct-messaging realtime slice — making it the highest-leverage infrastructure investment in the current backlog.

**Scope of Epic 2B to land (in order):**
1. Story 2B.1 — Event Bus Foundation (Redis pub/sub adapter in `backend/crates/`)
2. Story 2B.2 — Push Notification Service (FCM/APNs via HTTP v1)
3. Story 2B.3 — Email Notification Service (dispatch, not preference toggle — `services/email.rs` skeleton exists)
4. Story 2B.4 — In-App Notification Center (DB-backed, REST endpoints for list/mark-read)
5. Story 2B-C.1 — WebSocket Real-Time Sync (Axum WS upgrade handler, ADR-008)

Stories 2B.5 (Privacy-Aware Design), 2B.6 (Offline Sync Queue), 2B.7 (Idempotency), and 2B-C.2/2B-C.3 can follow in a subsequent sprint as they do not block the immediate Epic 6/8A unblock.

**Formally deferred (not deleted from backlog):**
The stories listed in "deferred slices" above remain in the backlog at their current priority. They are unblocked the moment Epic 2B stories 1–5 (including WS) are merged to dev. pm-scrum-master should re-sequence them into the sprint immediately after 2B lands.

**Unblock trigger table (pm-scrum-master, 2026-05-24):**

| Epic 2B story merged | Unblocks |
|---|---|
| `2b-1-event-bus-foundation` | 8a-2 dispatch half (can begin wiring event emission) |
| `2b-2-push-notification-service` | push leg of 6-2/6-3/6-4 notification dispatch |
| `2b-3-email-notification-service` | email leg of 6-2/6-3/6-4 notification dispatch |
| `2b-4-in-app-notification-center` | in-app notification display on 6-2/6-3/6-4 views |
| `2b-c1-websocket-realtime-sync` | 8a-3 WS half, 6-5 direct messaging realtime, future DM slice |
| All five (2b-1 → 2b-c1) merged | Full re-sequencing: 6-2/6-3/6-4/6-5/8a-2-dispatch/8a-3 move to ready-for-dev |

**Sprint sequencing note (pm-scrum-master, 2026-05-24):**
DEC-001 is the authoritative sequencing record. `sprint-status.yaml` updated to reflect:
Epic 2B stories (`2b-1 → 2b-2 → 2b-3 → 2b-4 → 2b-c1`) are sequenced first;
stories 6.2–6.5, 8A.2 dispatch, and 8A.3 WS sync are formally marked `blocked`
until those five stories merge to `dev`. pm-scrum-master will re-sequence them
into the sprint immediately after 2B lands, using the unblock trigger table above.

**DEC-001 unblock-trigger status (2026-05-25, pm-scrum-master):**
All five sequenced Epic 2B stories have effectively landed — **PR #463 (notification pipeline: event bus + push/email/in-app transports) and PR #472 (WebSocket realtime sync) merged 2026-05-25**. The unblock triggers in the table above have **fired**: 6-2/6-3/6-4/6-5, 8A.2 dispatch half, and 8A.3 WS half are unblocked. 6-5 (direct messaging) and the WS leg of 8a-3 are now delivered; the Epic 6 announcement web UI (6-2/6-3/6-4) moved to ready-for-dev and is in flight as draft PRs #474/#475/#479. Remaining: 8a-3 mobile-push leg (FCM/APNs).

---

## Decisions needed
- ~~Whether to pull Epic 2B notification infrastructure into the current sprint to unblock Epic 6 publish + 8A.2 dispatch — owner: pm-scrum-master _(raised 2026-05-23)_~~ **RESOLVED by DEC-001 (PR #442) + sprint-status sequencing (2026-05-24); unblock triggers FIRED 2026-05-25 (PRs #463/#472)**
- ~~Merge target/priority for security PR #435 vs. feature review backlog — owner: pm-tech-lead _(raised 2026-05-23)_~~ **RESOLVED — #435 merged; residual findings (#438/#439) reduced (SSRF #450 + ProtectedRoute #459 landed); remaining items owned by pm-security**
- ~~Whether to delete the dead AuthHandler/BuildingHandler modules now vs. wire them as the canonical path — owner: pm-tech-lead _(raised 2026-05-23)_~~ **RESOLVED — PR #437 deleted these modules**
- **NEW (2026-05-25, pm-frontend):** Should the Epic 6 announcement web UI ship as one squashed PR or land incrementally via #474 → #475 → #479? The three drafts share `AnnouncementsPage` and risk merge conflicts if landed out of order — owner: pm-frontend / pm-scrum-master.
- **NEW (2026-05-25, pm-qa):** Adopt a PR-merge policy requiring a test file for every security-labelled fix? PR #493 (equipment IDOR) included a regression test; PR #497 (inquiry IDOR) shipped without one — owner: pm-tech-lead.
- **NEW (2026-05-25, pm-qa):** P0-this-sprint vs formal deferral for high-severity #480 (JWT in WS access logs) and #481 (OAuth refresh-token revocation bypass) before 8a-3 / 10a-1 / 10a-3 are promoted to done — owner: pm-security + pm-tech-lead.
- **NEW (2026-05-25, pm-scrum-master):** Merge sequence for App.tsx route changes (80-3 mediation wiring first, then #503 system announcements) to prevent triple-conflict rebases on the top churn hotspot — owner: pm-frontend.
- **NEW (2026-05-27, pm-security):** Treat #614 (`update_schedule` missing RBAC), #624 (`update_schedule` missing tenant/org scope), and #617 (cookie `Path` breaking change from PR #565) as pre-release P0/P1 blockers gating Epic 81 promotion — owner: pm-security. _(#617 cookie-Path leg RESOLVED 2026-05-28 by PR #642 — auth.rs+sso.rs reconciliation with tests; #614/#624 still open.)_
- **NEW (2026-05-28, pm-data):** Should support-staff reads through the new Support Data admin page (#635) emit their own audit/analytics events (who viewed which tenant's diagnostics, who revoked sessions), separate from the `audit_read` capability gate, and should the FaultStatusCount metric be unified with the owner/portfolio fault KPIs into one shared definition? — owner: pm-data / pm-backend.
- **NEW (2026-05-29, pm-integration):** Accept or defer the sqlx 0.8→0.9 upgrade (Dependabot PR #666) this sprint — a workspace-wide DB-layer major bump that may break `query!`/`migrate`; needs a go/no-go with a named backend owner and a `cargo check --workspace` pass before landing — owner: pm-backend / pm-tech-lead.
- **NEW (2026-05-29, pm-integration):** Canonical fix for Airbnb at-least-once duplicate `SYNC_EXTERNAL` jobs (`bug-webhook-airbnb-dup-sync-jobs`): DB event_id dedup table (ON CONFLICT DO NOTHING) vs worker-level idempotent upsert — owner: pm-backend.
- **NEW (2026-05-29, pm-integration):** Must the Redis push-fanout BLPOP drain (`dx-push-fanout-blpop-drain`) ship before Epic 8A is considered production-shippable? sprint-status marks 8A done but the fanout path is a logging no-op — owner: pm-product / pm-backend.
- **NEW (2026-06-15, pm-qa):** Pre-push fmt/clippy gate (#1375) — local hook only, CI status check, or both? Owner: pm-tech-lead.
- **NEW (2026-06-15, pm-qa):** Triage protocol for the 18 follow-up issues #1360-#1377 — bulk-assign by theme to per-role queues, or per-issue triage? Owner: pm-scrum-master.
- **NEW (2026-06-15, pm-qa):** Promotion-gate policy — should each high-severity coverage gap (atomicity, IDOR, RLS) block its source epic's done-promotion until a failing-on-main test exists? Owner: pm-tech-lead + pm-qa.
- **NEW (2026-06-16, pm-devops):** Scope of pre-push fmt/clippy gate (#1431): local hook only, CI status check, or both? Local-only did NOT catch the #1426 → #1437 compile break. Owner: pm-tech-lead.
- **NEW (2026-06-16, pm-devops):** `dev`-push smoke gate enforcement model — fail-fast (block the push) vs warn-only (notify but allow)? backend.yml currently runs on PR only; #1437 would have been caught by an on-push `cargo check --workspace --tests`. Owner: pm-tech-lead + pm-devops.
- **NEW (2026-06-16, pm-devops):** CI bisect protocol when `dev` breaks — who owns + escalates? PR #1426 → #1437 was not surfaced for ~1 day. Owner: pm-scrum-master.

---

## Decisions logged 2026-07-23 (Phase 1.6 — pm-scrum-master + pm-data)

- **NEW (2026-07-23, pm-scrum-master):** Prioritize the 2 remaining `partial` MVP stories (84-1 direct-to-S3 upload wiring, 84-2 document-sign page) over post-merge follow-up issues in the next implementer window — both are frontend-only, backend is shipped, and closing them would take the project to 49/49. Owner: pm-frontend.
- **NEW (2026-07-23, pm-scrum-master):** Cross-cutting webhook hardening audit — treat #2485 (layout webhook lacks timestamp/replay guard) as a symptom, not an isolated bug. Booking / Airbnb / esignature / layout webhooks need parity check for signature verification, timestamp window, and replay protection. Owner: pm-integration + pm-security.
- **NEW (2026-07-23, pm-data):** Minimum-analytics DoD for new stories — proposed. Every new story that ships user-facing behavior must define at least (a) a business event, (b) an audit event if it mutates cross-tenant or platform-admin state, and (c) a KPI dashboard link (even if placeholder). Owner: pm-scrum-master + pm-data to decide binding vs advisory.
- **NEW (2026-07-23, pm-data):** Layout & Content Manager pilot shipped with zero instrumentation — publish/webhook analytics event schema needs to be defined before adoption grows and retrofitting becomes costly. Sequenced as `data-layout-publish-event-tracking-2026-07-23` on action-list. Owner: pm-data + pm-backend.
- **NEW (2026-07-23, pm-data):** Retention policy for `support_tooling_events` — carried over from 2026-05-28 open decision, still unresolved. Publishing a policy (TTL vs indefinite, PII classification) is now blocking pm-data's ability to expand the audit event pattern to disputes/OAuth/layout. Owner: pm-security + pm-data.
- **NEW (2026-07-23, pm-data):** FaultStatusCount metric unification — also carried over from 2026-05-28. As dashboards expand, dual definitions will produce contradictory numbers. Owner: pm-data (final call).

---

## Decisions logged 2026-07-30 (Phase 1.6 — pm-scrum-master + pm-backend)

- **NEW (2026-07-30, pm-scrum-master):** Reviewer-slot policy for large-scope feature PRs — the accounting MVP-loop trio (#2555 / #2558 / #2559) has been draft-ready for 2 days with no reviewer engagement, showing the dispatcher stack is now bottlenecked on reviewer capacity, not implementer capacity. Owner: pm-tech-lead to define whether large-scope PRs get an explicit reviewer slot or if reviewer rotation is added to the daily routine. _(2026-08-10 update: still unaddressed; the trio is now at 13 days idle. Same failure mode observed on #2684.)_
- **NEW (2026-07-30, pm-backend):** Standard: a hotfix that ships without a regression test needs an explicit follow-up issue at merge time (not discovered a run later). The PR #2547 (scheduler retention prune) and PR #2568 (Android SSO CSRF, half-wired) both slipped through this hole in the last two windows. Owner: pm-tech-lead.

---

## Decisions logged 2026-08-10 (Phase 1.6 — pm-scrum-master + pm-qa)

- **NEW (2026-08-10, pm-scrum-master):** Formal call: does the announcement fan-out KPI added by PR #2723 satisfy the pm-data `data-announcement-fanout-instrumentation` goal, or do we keep the item open with expanded scope (per-scope delivered/read/ack breakdown)? The action-list item was retired this run pending this decision. Owner: pm-data.
- **NEW (2026-08-10, pm-scrum-master):** Reviewer-slot policy for human-authored PRs (re-raise) — the accounting trio (#2555/#2558/#2559) is now at **13 days** idle and PR #2684 (workflow-cond-parse-failopen) is at ~2 days; the 2026-07-30 decision on reviewer-slot policy remains unaddressed. The dispatcher's bot loop is consuming all reviewer bandwidth. Owner: pm-tech-lead — 2-week deadline before this escalates to a blocker.
- **NEW (2026-08-10, pm-qa):** Enforcement mechanism for the "test-with-every-security-fix" policy (2026-06-15 open decision) — the reality-server security batch this window (#2724/#2725/#2726/#2727) shipped code fixes but the merge digest shows no matching failing-on-main tests. Options: (a) CI gate on `security:*` labels requiring a new test file in the diff; (b) PR-review checklist item; (c) scrum-master weekly audit. Owner: pm-tech-lead + pm-qa.
- **NEW (2026-08-10, pm-qa):** Coverage baseline requirement for churn hotspots (top-3 files by recent churn) before further dedupe passes — reality-server state.rs and routes/agencies.rs need a cargo-llvm-cov baseline before we ship the next split proposal, or we risk the next regression sliding under the covers. Owner: pm-tech-lead.
