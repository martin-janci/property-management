# PPT Project State

_Generated: 2026-06-18 — daily PM rotation (Scrum Master + pm-security; routine refresh). Coverage `scan_kind=upkeep`; pm_cursor idx 5 → 6 (pm-data next), coverage_cursor idx 12 → 0 (epic-85 → epic-6 wrap)._

## Executive summary

- **Governance gap confirmed (issue #1538):** `dev` branch protection requires only a single generic `"check"` context — `cargo test` is **NOT** a required check. Same structural cause as the #1426→#1437 dev-red incident two runs ago. pm-security is filing this as the headline blocker. **Recommendation: freeze backend feature merges until the gate is enforced.**
- **51 PRs merged since 2026-06-16** (range #1447–#1581). Heavy hardening wave: RLS coverage (#1460/#1473/#1492/#1495/#1499/#1551/#1563), OAuth/MFA (#1467/#1539/#1552/#1580), enum-cast bugfixes (#1486/#1489/#1492/#1499/#1537/#1551/#1578), IDOR regression seeds (#1467/#1470/#1517), and dev backend compile restored (#1495 unblocked dev test job).
- **Story 7a-1 promoted to `done`** via PR #1550 (round-trip upload-metadata test). Sprint-status updated 2026-06-17 with confirmation: documents/upload route + tests T1-T15 + S3 happy/failure paths green.
- **6 new untriaged issues:** #1520, #1521, #1522, #1532, #1533, #1538. **#1538 is the security-governance finding** — promoted to next-actions.
- **Churn hotspots:** `backend/crates/db/src/repositories/rental.rs` (8 PRs), `document.rs` (8 PRs), `booking_oauth_routes_tests.rs` (5 PRs) — security-sensitive surface under active multi-author convergence; #1538 means a regression could land silently.
- **Mobile momentum:** #1514/#1515/#1516/#1541/#1542 progress epic-82 stories (82.1 SwiftUI map, 82.2 deep-link confirm, 82.4 favorites session-gating); #1420 lands epic-83 story 83.2 OTA parse.

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · epics_done=1/5 (8A only).

| Epic | Tracked status | Real status (from coverage + activity) |
|---|---|---|
| 6 — Announcements & Communication | in-progress | 1/6 stories complete (6-6); web UI for 6-2/6-3/6-4 still in flight |
| 7A — Basic Document Management | in-progress | **1/5 stories complete (7a-1 done via #1550)**; 7a-2 stale in review (PR #1316) |
| 8A — Basic Notification Preferences | near-done | 8a-1/8a-2 done; 8a-3 publish-leg tests landed (#1395), only mobile-push (FCM/APNs) leg open |
| 10A — OAuth Provider Foundation | in-progress | 0/3 stories complete; #1197 OAuth integration test draft (now merged this wave) — re-verify status |
| 10B — Platform Administration | in-progress | 5/7 stories complete (10b-5 & 10b-7 remaining) |
| 82 — Mobile (Reality KMP) | in-progress | 82.1/82.2/82.4 advanced via #1514-#1542; iOS SwiftUI map docs landed |
| 83 — OTA Integrations | in-progress | story 83.2 lands via #1420 (inbound avail/rate-amount parse) |
| 85 — Mobile Build Pipeline | in-progress | EAS workflow pins confirmed healthy (#1526); both files present |

## Shipped since last run (cursor #1447–#1581 — 51 PRs)

### Backend hardening
- **#1467** — mfa verify_recovery_code RLS-enforce + cross-user IDOR test (BIT-78)
- **#1460** — FORCE RLS on developer_oauth_apps/grants/api_key_usage_logs
- **#1473** — BIT-85 cross-tenant OAuth upsert hazard + manager gate
- **#1495** — Unblock dev test job — form RLS-role grant + rental_platform_connections enum decode (closes the #1437 dev-red incident chain)
- **#1502** — Booking.com credential encryption at-rest contract (BIT-98, IG3)
- **#1537/#1551/#1578** — enum-cast fixes (rental_guests, forms, count_accessible_rls, booking enum binds)
- **#1539** — Reject public OAuth client supplying client_secret with invalid_client
- **#1552** — Gate OTA token-exchange on manager role of path org (closes #1525)
- **#1561** — Admit public agencies on imports via PortalPrincipal (closes #1300)
- **#1563** — Behavioral cross-user RLS isolation for developer_oauth_apps/grants/api_key_usage_logs (closes #1527)
- **#1580** — Rate-limit MFA recovery-code verify endpoint (closes #1523)

### Story / feature
- **#1550** — 7A.1 round-trip upload-metadata test; **7a-1 → done**
- **#1555** — UC-03 FaultsPage useDeleteFault/useFaultStatistics wiring (gap-79-1)
- **#1420** — Epic-83 story 83.2 inbound OTA avail/rate-amount parsing
- **#1541** — Epic-82 story 82.4 FavoritesService session-gating + screen-map sync
- **#1516** — Epic-82 story 82-2 deep-linking + URL-scheme registration confirmed
- **#1542** — UC-82 iOS SearchView stale-response guard
- **#1548** — UC-81 presigned download + retry on report execution history
- **#1514/#1515** — Reality iOS SwiftUI screen-map (82.1) + epic-82 SwiftUI story map

### Test / coverage
- **#1456** — Forms RLS write/create + cross-tenant negatives (#1369)
- **#1470** — DSA report download self-ref regression guard (GH #1201)
- **#1471** — RLS-routing coverage (ESG FORCE policy, webhook repo, public plans, BIT-74)
- **#1477** — App.tsx auth-gate wiring (GH #1411)
- **#1478** — INSERT-branch fail-on-main for registry NOT-NULL COALESCE
- **#1501** — reality-server handler-level IDOR regression for imports/feeds (GH #1300)
- **#1517** — IDOR seed_membership migration to canonical helper (#1373)
- **#1562** — XXE/DTD-safe + namespace-prefix parity for inbound OTA parsers (closes #1519)
- **#1564** — `mark_invitation_viewed` first-view preservation (closes #1524)
- **#1579** — Un-ignore airbnb reconcile tests
- **#1197** — Integration coverage for OAuth authorization server (was stale draft — now merged)

### DevOps / docs
- **#1526** — EAS mobile workflow action pins confirmed healthy
- **#1530** — Document `security-test-gate.yml` required-check verification + remediation
- **#1540** — Confirm `app-tsx-merge-queue.yml` actively serializing App.tsx PRs

## What's next (top 5 actions)

1. **[high] Make `backend / test` a required status check on `dev`** — close issue #1538. Owner: pm-devops (driven by pm-security). Until done, freeze backend feature merges.
2. **[high] Triage 6 untriaged issues #1520/#1521/#1522/#1532/#1533/#1538** — assign owner, severity, sprint slot. Owner: pm-scrum-master.
3. **[high] Resolve 7a-2 folder-organization (PR #1316)** — promote or close. document.rs is hottest churn file (8 PRs), rebase debt growing. Owner: pm-backend.
4. **[high] Audit rental.rs/document.rs 8-PR churn cluster for missing integration-test coverage** — file follow-ups for any gap. Owner: pm-qa.
5. **[high] Close #481 OAuth refresh-token revocation bypass** — add integration test asserting revoked tokens rejected; verify session.rs:55 `revoked_at IS NULL` guard. Owner: pm-security.

## Blockers

- **#1538 — backend `test` job not required on `dev` (CRITICAL).** Owner: pm-devops. Same class as #1426→#1437. Auth/RLS regressions can merge silently.
- **#480 — WebSocket JWT in URL query param** (structural exposure persists across all WS sessions). Owner: pm-security + pm-backend.
- **#481 — OAuth refresh-token revocation bypass** (partial backfill at session.rs:55; no integration test). Owner: pm-security + pm-backend.
- **7a-2 / PR #1316** — stale in review; FK/isolation fix unconfirmed CI green. Owner: pm-backend.
- **Test-hardening batch #480-#487 follow-up cluster** — 7 of 8 still open; gate stories 8a-3, 10a-1, 10a-2, 10a-3 from promotion. Owner: pm-qa + pm-security.

## Role focus today: **pm-security** (+ pm-scrum-master always-on)

- **pm-security** (rotation idx 5, last 2026-05-27, 22d stale): 6 next_actions appended to `action-list.json`; 5 new risks appended to `risks.json`. Full role JSON in `.research/management/roles/pm-security.md`. Headline: **dev branch test job not required → #1538 confirmed**; OAuth refresh revocation (#481) partial backfill but no integration test; WS JWT-in-query (#480) structural exposure remains; document.rs deprecated non-RLS methods callable during 7A churn.
- **pm-scrum-master** (always-on): produced the delivery synthesis above; headline = `dev` test gate gap is still the dominant structural risk; PR wave #1447–#1581 was heavily corrective (RLS + enum-cast + IDOR seeds), but governance gap negates the net.

## Coverage upkeep

- **epic-85 (rotation idx 12) refreshed** in `coverage.json`: both stories (85-1, 85-2) `last_checked=2026-06-18`; added evidence `PR #1526 confirms EAS workflow action pins healthy — both eas-build-android.yml + eas-build-ios.yml present + pinned`. Status remains `partial` until workflow_dispatch green runs are recorded.
- **Cross-story PR-evidence updates:**
  - `7a-1-document-upload-metadata`: PR #1550 round-trip metadata test → status `done` confirmed.
  - `82-*` (3 stories): #1516/#1541 evidence appended.
  - `83-2`: #1420 inbound OTA avail/rate-amount evidence appended.
- Next epic to refresh: **epic-6** (coverage_cursor wraps to idx 0).
