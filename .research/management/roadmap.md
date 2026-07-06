# PPT Delivery Roadmap

_Generated 2026-07-06 (upkeep) — coverage last deep-scan 2026-07-02; today rotates epic-9 upkeep and advances coverage_cursor to epic-10a (idx 12→0)._

## State of the project

**49 stories classified: 40 done · 9 partial · 0 not-started.** MVP is effectively feature-complete; the remaining work is a small tail of reconciles, one mobile-native slice, two feature gaps, and cross-cutting screen-map drift.

| Platform | done | partial |
|----------|-----:|--------:|
| backend  | 31 | 6 |
| frontend | 27 | 6 |
| mobile   | 8  | 4 |

_(stories span multiple platforms, so columns overlap)_

**Screen coverage:** 8 stories without a matching screen-map · 2 orphan epics (epic-85, epic-8a) · 29 orphan screens · 4 missing UC links (UC-10, UC-29, UC-33, UC-40). The systemic driver is that 0/120 screen-maps populate `frontmatter.epics:` — backfilling that is the single highest-leverage fix.

**Top 3 gaps by impact:**
1. **JWT-role-vs-DB-role manager-gate class** — PR #2120 fixed outages.rs only; ~9 handler files across documents/announcements/templates/granular_notifications still gate manager-only mutations on the (Guest-defaulted) JWT role. Epic 6 / 7A stories marked done likely fail closed for real managers in production.
2. **Mobile OS push (FCM/APNs)** deferred (`8a-3`) — pipeline dispatch is serial and the FCM stub swallows failures (#484).
3. **7a-2 CI red on `document_folder_tests`** (PR #1316) blocks epic-7a completion.

Also notable: two security-relevant draft PRs stalled — **#1797** (OCR auth + rental-guest PII, 13d) and **#1812** (reality_portal split, 12d).

## Ranked plan

### MVP

- **[high]** Migrate all `tenant.role.is_manager()` mutation/override gates to DB-validated role (mirror PR #2120) — owner: pm-security — why: same JWT-role bug class silently affects Epic 6/7A shipped stories; security + already-shipped-fail-closed.
- **[high]** Escalate stalled draft PR **#1797** (OCR endpoints auth + manager-gate rental guest PII) — owner: pm-security — why: 13d stale live PII exposure.
- **[high]** Reconcile + verify **permission-based document access** (`7a-3`) — owner: pm-backend — why: authz-sensitive; code + `document_access_test.rs` exist but sprint-status stale and frontend permission UI unconfirmed.
- **[high]** Close out **document sharing** (`7a-5`) — owner: pm-backend — why: confirm web share-panel UUID validation + confirm-dialog parity with mobile fix #2003, resolve #485.
- **[high]** Reconcile **environment-variable setup** (`85-1`) — owner: pm-devops — why: RN/iOS env config implemented (Expo `app.config.ts`) but story doc still in-progress; env-setup .md missing.
- **[high]** Reconcile `sprint-status.yaml` against `coverage.json` — flip epic-10a (10a-1/2/3), epic-10b, 7a-3, 7a-4 to done — owner: pm-scrum-master — why: spine misreport blocks stakeholder view.
- **[high]** Add OAuth 10a security test suite (introspection / refresh-rotation / PKCE) — owner: pm-security — why: revoked-token bypass gate #481 keeps 10a-1/10a-3 flagged.
- **[high]** Implement Redis push-fanout BLPOP drain (push_fanout.rs) — owner: pm-backend — why: jobs enqueued to `push_fanout_queue` silently dropped while 8A stories marked done; infra-foundational for notifications.
- **[medium]** Fix red CI on **folder organization** (`7a-2`) — owner: pm-backend — why: `document_folder_tests` FK/isolation failing (PR #1316); unblocks epic-7a completion.
- **[medium]** Reconcile **document download & preview** (`7a-4`) — owner: pm-backend — why: download/preview handlers shipped; flip stale sprint-status, finish planned preview second frame.
- **[medium]** Apply the **5-step dispute-filing wizard** redesign (`80-2`) — owner: pm-frontend — why: MVP single-page form shipped; the wizard redesign is the remaining polish gating done.
- **[medium]** Close #484 (serial dispatch + FCM stub silent failures) to clear `8a-3` — owner: pm-scrum-master — why: fully clears 8a-3 from partial.
- **[medium]** Confirm closure of test-hardening batch #481 (RFC 9700 revocation) + #487 (MFA rate-limit) — owner: pm-scrum-master — why: still gate 10a-1/10a-3 in spine.
- **[medium]** CI lint for `TenantExtractor + role.is_manager()` mutation antipattern — owner: pm-security — why: prevent JWT-role bug class from reappearing.
- **[medium]** Verify `dev` branch protection enforces "Require Code Owners review" — owner: pm-security — why: PR #2111 deny.toml gate advisory-only otherwise.

### Phase 2

- **[medium]** Implement **create-schedule** for reports (`81-1`) — owner: pm-frontend — why: only `PUT`/pause/resume exist; add `POST /api/v1/reports/schedules` so new schedules can be created.
- **[medium]** Pin cron-validator regression test (Epic 81 gate) — owner: pm-qa — why: prevents #616 recurrence.

### Phase 3

- **[medium]** Add e-signature webhook idempotency guard — owner: pm-backend — why: provider re-delivery can overwrite terminal state.

### Phase 4

- **[medium]** Finish **inquiries & account** on mobile-native (`82-5`) — owner: pm-frontend — why: build Android/KMP Compose inquiry thread + calendar; replace stub Account Profile/Settings with real screens.

### Screen-map drift (cross-phase)

- **[medium]** Backfill screen-map `frontmatter.epics:` across ~120 screens — owner: pm-frontend — why: single highest-leverage fix; eliminates false-orphan cluster of 29 screens.
- **[medium]** Add screen-map for epic-8a `NotificationSettingsPage` — owner: pm-frontend — why: real ppt-web sub-page shipped without map.
- **[low]** Add screen-map(s) for orphan epic-85 (mobile env/build/EAS) — owner: pm-frontend.
- **[low]** Link UC-10 / UC-29 / UC-33 / UC-40 to screen-maps — owner: pm-frontend.
- **[low]** Reconcile / remove ppt/ · reality/ · reality-mobile/ orphan-screen clusters — owner: pm-frontend.

Buffer: 36/36 open · 0 candidates ranked but unqueued
