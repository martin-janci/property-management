# Risks

<sub>Generated: 2026-06-27T14:25:00Z</sub>

| Owner | Probability | Impact | Risk | Mitigation |
|-------|-------------|--------|------|------------|
| pm-backend | high | high | Cross-tenant IDOR cluster in ai.rs equipment endpoints: update_equipment (ai.rs:1113), delete_equipm | Thread the principal's tenant_id into equipment_repo.update/delete/update_mainte |
| pm-devops | high | high | PR #1426 merged despite breaking `dev` backend compile (issue #1437) - ALL backend CI gates now brok | Land #1435 or #1436 immediately to restore green. Add `cargo check --workspace - |
| pm-qa | high | high | Issue #480 (JWT access token in WebSocket query-param access logs) is high-severity and open; story  | Block 8a-3 promotion; strip token from log line and re-validate JWT on session b |
| pm-qa | high | high | Issue #481 (OAuth refresh-token revocation bypass) is high-severity and open, yet stories 10a-1/10a- | Block 10a-1/10a-3 promotion until #481 closed; escalate to tech lead. |
| pm-scrum-master | high | high | Story 7a-1 not promotable: POST /api/v1/documents/upload backend handler absent; mobile upload UI (P | Immediate backend task; critical-path blocker for Epic 7A completion. |
| pm-scrum-master | high | high | Epic 16 saved-search-alerts drainer has two HIGH-severity concurrency bugs (duplicate emails/pushes  | Expedite rust-backend patch with SELECT FOR UPDATE skip-locked + single-transact |
| pm-security | high | high | update_schedule (report_schedule.rs) missing RBAC (#614) and missing tenant/org scope (#624): an aut | Add RequireCapability extractor + thread principal tenant/org into the WHERE cla |
| pm-security | high | high | WebSocket JWT exposed in HTTP access logs: buildWebSocketUrl appends ?token=<jwt> (websocket.ts:94); | Short-lived one-time WS ticket endpoint (REST pre-auth) or proxy log scrubbing;  |
| pm-backend | high | medium | 10B handler stubs (10b-4/5/6/7) are mounted routes returning success with no business logic — caller | Return 501 Not Implemented with a documented timeline until real handlers ship,  |
| pm-devops | high | medium | Mobile release pipeline has no merged path: EAS Android/iOS build workflows exist only in draft PRs  | Land gap-85-2-android-ci-fix + gap-85-2-ios-ci-fix together; downgrade pins to @ |
| pm-frontend | high | medium | Post-merge review opened follow-up issues #480-#487 covering test gaps + minor security/UX follow-up | pm-scrum-master to slot #480-#487 into the sprint as a single test-hardening bat |
| pm-backend | high | medium | Airbnb at-least-once webhook (webhook.rs:1028) enqueues duplicate SYNC_EXTERNAL jobs on event bursts | Add an event_id dedup table (ON CONFLICT DO NOTHING) + already-queued-job check  |
| pm-backend | high | medium | Redis push-fanout queue (push_fanout.rs:621) is never drained (BLPOP deferred) — jobs enqueued to pu | Implement the BLPOP drain before any production traffic relies on the fanout pat |
| pm-devops | high | medium | Dispatcher meta-issue #1380: stale gap-scan buffer feeds no-op claims + Tier-2 escalation endpoint m | pm-devops or dispatcher owner refreshes gap-scan buffer; verify Tier-2 endpoint  |
| pm-scrum-master | high | medium | 18 follow-up issues (#1360–#1377) from the post-merge review of 2026-06-14 merges remain untriaged;  | pm-scrum-master triages the batch this run — assign owner or close as won't-fix; |
| pm-scrum-master | high | medium | App.tsx is the top churn hotspot; concurrent edits across #503, #500, and 80-3 mediation wiring risk | Sequence App.tsx PRs: merge 80-3 wiring first, then #503; pm-frontend coordinate |
| pm-frontend | high | medium | 10 of 22 remaining partial/not-started stories are mobile (7a-2/7a-4, 8a-3 push, epic-82 SwiftUI); u | Force gap-82 drafts (#639/#641/#705) to non-draft review this window; assign pm- |
| pm-scrum-master | high | medium | sprint-status.yaml is materially stale (10b-3/10b-4/10b-6 still ready-for-dev despite done evidence) | Write the sprint-status update as a tracked action (pm-scrum-master-sync-sprint- |
| pm-scrum-master | high | medium | sprint-status.yaml is 5+ weeks stale (last updated 2026-05-25) — team may make planning decisions of | Orchestrator should write the reconciled sprint-status.yaml immediately as part  |
| pm-devops | medium | high | security-test-gate.yml may be advisory only (not a required status check): PR #497 shipped a securit | Make the gate a required status check on dev branch protection; re-run against a |
| pm-devops | medium | high | `security-test-gate.yml` workflow file is present but enforcement-vs-advisory on `dev` branch protec | Verify required-status-check listing via `gh api repos/.../branches/dev/protecti |
| pm-backend | medium | high | sqlx 0.8→0.9 major bump (Dependabot PR #666) affects every workspace query; merging without a coordi | Freeze PR #666 from auto-merge; backend owner audits the 0.9 changelog and runs  |
| pm-security | medium | high | Booking.com OAuth/credential connect flow lacks secure replacement on re-connect (#1362, #1374) — ol | Implement atomic credential swap + add OAuth handler/CSRF test coverage. |
| pm-qa | medium | high | PR #497 inquiry IDOR fix shipped with zero regression tests; mark_inquiry_read_for_realtor does an o | Add reality-server/tests/inquiry_idor_tests.rs immediately; require a test file  |
| pm-backend | medium | high | record_payment handler does a check-then-insert without serializable isolation or unique-constraint  | Wrap in serializable tx OR add (idempotency_key, payment_id) unique constraint;  |
| pm-scrum-master | medium | high | 11-day cursor lag in the daily research routine means 75+ post-merge reviewer issues accumulated wit | Restore daily routine cadence; pm-security to triage the 75 'follow-up + from-me |
| pm-security | medium | high | PR #725 (ai-maintenance IDOR fix) sits at verdict=changes (B1-session-IDOR, B2-sentiment-IDOR, B3-mi | pm-security addresses the three change requests within a day; reviewer re-approv |
| pm-security | medium | high | PR #435 merged with deferred security findings (#438/#439): P1-05 SSRF, P0-12 cookie scope, P1-04 De | Assign pm-security sole owner of the residual #438/#439 items; treat remaining P |
| pm-scrum-master | medium | high | Six open test-hardening gates (#480-#485, #487) with no assigned owner or due date — OAuth stories ( | Assign specific owner roles and target sprint for each gate; consider deferring  |
| pm-security | medium | high | Cookie Path breaking change (#617) from PR #565 session-cookie scope hardening: if mis-scoped it eit | Verify cookie set/clear across / and the API prefix and the SSO callback flow be |
| pm-security | medium | high | Guest ID-document OCR (story 18.2, PR #1750 merged) processes government-issued identity documents;  | Merge PII hardening PR #1823 before Epic 18 prod rollout; ensure OCR route does  |
| pm-security | medium | high | LLM document IDOR (draft PR #1857 unmerged): if LLM-document endpoints are not tenant-RLS-gated, a m | Merge PR #1857 before any LLM-document feature ships to prod; treat as release b |
| pm-security | medium | high | OAuth provider stories 10a-1/10a-2/10a-3 shipped with no introspection/refresh-rotation/PKCE securit | Add the OAuth security test suite (revoked-token, family-reuse, PKCE S256) befor |
| pm-security | medium | high | Refresh-token revocation gap (issue #481): if find_by_token_hash_any_status filters revoked_at IS NU | Read session.rs to confirm the query fetches any-status rows; fix if not; issue  |
| pm-tech-lead | medium | high | Three route monoliths on hot paths this run: ai.rs (3142 lines), platform_admin.rs (2762), announcem | Add ai.rs/platform_admin.rs/announcements.rs to the module-split backlog; add a  |
| pm-backend | medium | medium | Epic 81 frontend calls backend report-schedule endpoints (/schedules/{id}/pause\|resume, /executions) | Implement the missing endpoints or feature-flag the frontend calls until they ex |
| pm-data | medium | medium | FaultStatusCount / FaultByStatusTable surfaced by the new Support Data admin page (#635) defines a f | Factor the fault-status bucket + window definition into one shared query/helper  |
| pm-data | medium | medium | The Support Data page (#635) lets support/admin staff read cross-tenant user diagnostics (membership | Emit a support_data_viewed / support_sessions_revoked audit event (admin_user_id |
| pm-devops | medium | medium | App.tsx router-file churn + 6 concurrent dispatcher drafts (#563-#568) risk repeated triple-conflict | Serialize App.tsx-touching PRs via a merge queue / auto-rebase ordering so concu |
| pm-devops | medium | medium | `eas-build-android.yml` and `eas-build-ios.yml` now exist in `.github/workflows/` (cleared since 202 | Trigger a no-op push or workflow_dispatch run; confirm both Android and iOS jobs |
| pm-devops | medium | medium | Pre-push fmt/clippy gate (#1431) merged but is local hook only - does not enforce on CI side and doe | Mirror the gate as a fast CI status check (`cargo fmt --check && cargo clippy -q |
| pm-frontend | medium | medium | Epic 6 announcement web UI (viewing/ack 6-2, comments 6-3, pin 6-4) is split across three draft PRs  | Sequence the three drafts (#474 viewing → #475 comments → #479 pin) so they merg |
| pm-backend | medium | medium | e-signature webhook status writes have no idempotency guard; a provider re-delivering a completed/vo | Skip the workflow update when current_status is terminal; coordinate with PR #71 |
| pm-qa | medium | medium | Cron validator drift (#1368) could silently reintroduce regression #616 (Epic 81 promotion blocker)  | Pin a fixture-based test for the cron validator; gate Epic 81 promotion on it. |
| pm-scrum-master | medium | medium | saved_search_alerts.rs and reality_portal.rs are churn hotspots (3 touches each in this run) — conti | Serialize drainer work through a single branch; add integration test coverage be |
| pm-security | medium | medium | P1-04 residual (PR #435): internal type internals may reach audit-trail log lines via Debug ({:?}) f | Replace {:?} with Display/structured fields on internal types in audit records. |
| pm-backend | low | high | Two outbound-request sinks (signatures.rs:628 signed_url fetch, integrations.rs:2743 webhook-test PO | validate_external_url extracted to shared module and applied at the outbound sin |
| pm-scrum-master | low | high | Epic 6/8A blocked on un-built Epic 2B notification infra; deferred dispatch + WS sync (8A.2/8A.3) | Epic 2B notification pipeline (PR #463) + WebSocket realtime sync (PR #472) merg |
| pm-security | low | high | Stripe webhook replay: verify_signature in stripe.rs enforces a 300-second timestamp tolerance windo | Add startup assertion that STRIPE_WEBHOOK_SECRET is set (similar to JWT_SECRET p |
| pm-security | low | high | Voice device endpoints permitted cross-tenant device access (IDOR) — a principal could address anoth | Tenant scoping added to voice device lookups (PR #461). |
| pm-frontend | low | medium | ProtectedRoute.tsx:117 role gate was fail-open: when user.role was falsy the role check was skipped  | Now denies on missing role; user.role populated in AuthContext (PR #459). |
| pm-tech-lead | low | medium | OAuth provider (10A) backend complete; admin client-management UI + user-grants UI now shipped (PRs  | Add OAuth integration test suite (gap-10a-1-oauth-tests); UI delivery risk now c |
