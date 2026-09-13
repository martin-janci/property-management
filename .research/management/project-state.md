# PPT Project State

_Generated: 2026-09-13 — routine Phase 1.6 lightweight upkeep (pm-security rotation slot; last 2026-07-21, 54d stale slot refreshed) + pm-scrum-master always-on. Coverage `scan_kind=upkeep`; pm_cursor idx 5 → 6 (pm-security → pm-data next), coverage_cursor idx 7 → 8 (epic-82 re-checked, no material change)._

## Executive summary

- **Delivery unchanged at 47/49 stories done, 2 partial** (84-1 direct-to-S3 upload wiring; 84-2 signer page). No status flips. Since 2026-09-01 the loop shipped **10 code-review fixes + 4 auth/mobile security PRs** (17 merges) across api-server, reality-server, ppt-web, and RN mobile:
  - Security — PR #2941 (JWT exp validation at cold boot), #2942 (single-flight refresh + 401 replay), #2943 (logout purge allowlist coverage), #2950 (mobile NFC/QR credential purge, closes #2947), #2955 (SecureStore NFC credential purge, closes #2953).
  - Correctness — #2922 (saved-search error enum), #2925 (compliance raw-db-leak fix), #2928 (report_summary snapshot consistency), #2931 (test-fails-on-pre-fix regression), #2938 (invoice PDF renderer + tests, UC-ACC-05.9), #2940 (ConfirmationDialog i18n).
  - Test hygiene — #2932 (e2e 403 test agent self-review), #2952 (auto-discover feature-local queryKeys), #2956 (harden queryKeys.test auto-discovery).
  - Housekeeping — #2926 (remove decommissioned FCM legacy), #2927 (EmergencyContact i18n test), #2935 (npm-minor bump).
- **One open cross-tenant security gap:** issue **#2946 — portfolio_analytics IDOR** still open. Sibling fixes #2944 (violation comments/evidence/payments) and #2945 (portfolio_properties) landed clean; analytics is the last open surface in the sweep. Owner: null (unclaimed, top action).
- **Two open infra blockers hurting cloud runners:**
  - **#2949** — `utoipa-swagger-ui` build script blocked in cloud sandbox → blocks every backend security PR from the cloud implementer path.
  - **#2951** — RN `jest-expo` ↔ `react-native` version rot → blocks mobile jest in cloud; RN test PRs currently must land via local implementer.
- **Draft stalled 9d:** PR **#2902** (screen-map drift reconcile) has a dirty base; needs rebase against dev. Owner: martin-janci.
- **Churn cluster** this window: `frontend/apps/mobile/src/services/resetLocalData.ts`, `NFCCredentialManager.ts`, `frontend/apps/ppt-web/src/lib/queryKeys.test.ts` — all part of the cross-tenant-purge hardening train. No structural refactor needed yet; will re-evaluate next hotspot window.

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 3/5** unchanged.

| Epic | Sprint status | Coverage status (13 epics) |
|---|---|---|
| 6 — Announcements & Communication | in-progress | 6/6 stories done |
| 7A — Basic Document Management | in-progress | 5/5 stories done |
| 8A — Basic Notification Preferences | done | 3/3 stories done |
| 10A — OAuth Provider Foundation | done | 3/3 stories done |
| 10B — Platform Administration | in-progress | 7/7 stories done |
| 80 — Dispute Resolution | partial | 3/3 stories done |
| 82 — (extended) | (extended) | done in coverage; **re-checked this run (idx 7), no material change, last_checked=2026-09-13** |
| 84 — Documents / e-signature | (extended) | 3/5 done, 2 partial (84-1, 84-2) — unchanged |
| 81 / 83 / 85 / 79 / 7a / 8a / 9 | (extended) | all done in coverage |

## Shipped since last run (17 merges 2026-09-01 → 2026-09-13)

- **#2922** reality-server saved-search error enum · **#2925** api-handlers compliance raw-db-leak fix · **#2926** remove decommissioned FCM legacy · **#2927** EmergencyContact i18n test · **#2928** report_summary snapshot consistency · **#2931** test-fails-on-pre-fix regression · **#2932** reality-server e2e 403 test (agent self-review) · **#2935** npm-minor bump · **#2938** ppt-web accounting invoice PDF renderer (UC-ACC-05.9) · **#2940** ConfirmationDialog i18n · **#2941** JWT exp validation at cold boot · **#2942** single-flight refresh + replay on 401 · **#2943** logout purge allowlist coverage · **#2950** mobile NFC access-log purge (closes #2947) · **#2952** auto-discover feature-local queryKeys · **#2955** SecureStore NFC credential purge (closes #2953) · **#2956** harden queryKeys.test auto-discovery.

## What's next (top 5 actions)

1. **[high] Close cross-tenant IDOR in portfolio_analytics (#2946)** — sibling fixes (#2944, #2945) shipped; analytics is the last open surface — **owner: null** (needs claim).
2. **[high] Rebase & land PR #2902 (screen-map drift reconcile)** — draft stalled 9d, dirty base — **owner: martin-janci**.
3. **[high] Unblock utoipa-swagger-ui build in cloud runner (#2949)** — every backend security PR blocked from cloud path — **owner: devops / pm-devops**.
4. **[high] Unblock RN jest-expo↔react-native version rot (#2951)** — blocks mobile jest in cloud — **owner: devops / pm-devops**.
5. **[high] Wire ppt-web direct-to-S3 upload (84-1) + signer page (84-2)** — MVP 49/49 gated; standing 4+ upkeep windows — **owner: pm-frontend**.

## Blockers

- **#2949** — `utoipa-swagger-ui` build blocked in cloud runner (backend security PRs must go local).
- **#2951** — RN `jest-expo` ↔ RN version rot blocks mobile jest in cloud.
- **Standing:** #2652 (mobile-native/KMP cloud-runner egress) still open; drives buffer starvation.

## Role focus today: **pm-security** (rotation idx 5; last 2026-07-21, 54d stale) + pm-scrum-master always-on

- **pm-scrum-master** (always-on): the code-review + security-fix loop is delivering — 17 merges in 12 days with no in-window regressions detected. Delivery-side risk is entirely infra: cloud-runner egress (#2949, #2951, #2652) is now the dominant loss mode, not implementer capacity.
- **pm-security** (rotation, 54d stale): cross-tenant IDOR sweep landed 2/3 sibling fixes (#2944, #2945); #2946 (portfolio_analytics) remains the sole open cross-tenant gap and is promoted to the top of the action list. Mobile-side credential-purge track also landed clean (#2950, #2955). New risk logged: `cloud-runner-egress-blocks-security-fixes` — until #2949 is resolved, every backend security PR needs the local implementer path.

## Coverage (upkeep this run — 2026-09-13)

- **`coverage.json` untouched** — `scan_kind=upkeep`, no re-scan (routine's own Phase 1.6 advances the cursor in `state.json`).
- **Epic re-check: epic-82** — cursor idx 7. All stories still `done`. No PR in the 2026-09-01..09-13 window materially touched epic-82 surfaces beyond routine correctness fixes. `last_checked = 2026-09-13`.
- **`coverage_cursor` advances 7 → 8**.
- **`pm_cursor` advances 5 → 6** (pm-security → pm-data next run). `role_last_run["pm-security"] = 2026-09-13`.
- **Composition unchanged: 47 done · 2 partial · 0 not-started** across 13 epics. Zero orphan screens, zero validation errors.
