# PPT Project Roadmap (Deep Scan)

_Generated: 2026-05-23 (deep scan) · upkeep-refreshed 2026-05-27 — supersedes `_bmad-output/implementation-artifacts/gap-analysis-remediation.md` (Epic 86, stale)._

_Upkeep 2026-05-27: 1 PR merged since last run — #555 (gap-80-3 mediation workspace UI, +1612/-137 with tests). Story **80-3 advanced partial → done**: the mediation timeline/resolution-form/chat-thread shipped, the 4 critical App.tsx dispute-route wiring gaps closed, and the missing `docs/screens/ppt/dispute-detail.md` mediation screen-map landed. Rotating epic re-checked: **epic-9** (cursor idx 3) — no MFA regression churn this window; gap-10b-3-health-ui-mfa-fix (#567, approved) is an admin-health MFA-modal wiring fix, not a 9-1 regression — 9-1 remains done. DevOps run: mobile EAS release workflows live only in draft PRs (broken @v6 pins) — no merged mobile build path; security-test-gate enforcement unconfirmed after PR #497._

_Prior upkeep 2026-05-25: 30 PRs merged (#441–#473). Epic 2B notification pipeline (#463) + WebSocket realtime sync (#472) cleared the DEC-001 blockers. Stories advanced to **done**: 9-1 (MFA frontend #441 + e2e #473). Already-done stories enriched with UI evidence: 7a-3/7a-5, 6-5, 6-6 (#464), 10a-2/10a-3 (#468/#469/#471). 8a-3 WS half cleared by #472 (mobile-push leg remains)._

_Code-review finding 2026-05-25: cross-tenant IDOR cluster in `ai.rs` equipment endpoints (update/delete equipment + update_maintenance discard the principal → unscoped mutations). Tracked as `pm-backend-fix-ai-equipment-idor` (high)._

_Sequencing note (pm-scrum-master): DEC-001 unblock triggers have **fired** — 6-2/6-3/6-4/6-5, 8A.2 dispatch, and 8A.3 WS sync are no longer blocked. The Epic 6 announcement web UI is the active frontend cluster (drafts #474/#475/#479)._

## State of the project

| Status | Count | Share |
|---|---:|---:|
| done | 23 | 47% |
| partial | 25 | 51% |
| not-started | 1 | 2% |
| **total** | **49** | 100% |

**Phase breakdown:** MVP 36 stories (20 done, 16 partial), Phase 2 3 stories (0 done, 2 partial, 1 not-started), Phase 3 1 story (done), Phase 4 9 stories (2 done, 7 partial). (80-3 mediation moved MVP done +1 via PR #555.)

**Platform breakdown (a story may span multiple):** backend ~35 stories touched, frontend (web) ~30, mobile (RN + iOS) ~12. Mobile is the most-behind platform — Epic 7A mobile slices, Epic 82 (iOS), and Epic 85 (build/env config) are all `partial` with low/medium confidence.

**Top 3 gap clusters**
1. **Cross-tenant scoping bugs in route monoliths.** `ai.rs` (3142 LOC) has a confirmed IDOR cluster — `update_equipment`/`delete_equipment`/`update_maintenance` discard the principal and issue unscoped mutations. The 3k-line files (ai.rs, platform_admin.rs 2762, announcements.rs 2722) are exactly where a missing tenant scope slips past review. Highest blast radius this run.
2. **Backend handler stubs masquerading as routes.** Epic 10B stories 4–7 (system announcements, support data access, onboarding tours, contextual help) have routes mounted in `main.rs` but handler bodies are stubs. Same pattern in 81-1/81-2 where the frontend calls pause/resume + execution-history endpoints that don't exist in `reports.rs` (verify PRs #488/#489 in flight).
3. **Test-coverage debt on the heavy frontend delivery.** 30 PRs landed messaging realtime, document share-flow, OAuth UI, and MFA; follow-up issues #480–#487 track the gaps. Risk: the done-count outruns regression coverage on auth/sharing surfaces. (The prior "frontend API integration debt" cluster has largely closed — Epic 7A web+mobile UI and Epic 6 messaging/neighbor stories shipped.)

## Ranked plan

### MVP (Phase 1)

- [high] Wire TwoFactorAuthPage to `/api/v1/auth/mfa/*` (add `useMfa` hooks to `@ppt/api-client`) — owner: pm-security — why: security-sensitive feature is silently disabled end-to-end (UI scaffold only).
- [high] Build WebSocket realtime sync infra for notification preferences (Epic 2B foundational) — owner: pm-backend — why: blocks 8a-3 closure and direct-messaging/announcement realtime flows.
- [high] Complete frontend API integration for documents permission-based access (RLS UI, story 7a-3) — owner: pm-backend — why: scored highest (mvp + mobile + RLS) — apiStatus stub means access enforcement isn't user-visible.
- [high] Wire login form/logout/session cleanup to AuthContext (story 79-2) — owner: pm-frontend — why: auth-flow plumbing missing despite AuthContext fully built.
- [high] Implement mobile (RN) document upload UI with metadata (story 7a-1) — owner: pm-frontend — why: mobile slice missing on shipped backend.
- [high] Implement mobile document sharing UI (story 7a-5) — owner: pm-frontend — why: mobile slice missing; backend already supports user/role/public link + password shares.
- [high] Implement PDF.js client-side preview for documents (story 7a-4) — owner: pm-frontend — why: presigned URLs work, but preview rendering not implemented.
- [high] Promote direct messaging screens (story 6-5) from `apiStatus: stub` to integrated — owner: pm-frontend — why: routes + types all exist, missing only wiring.
- [high] Build dedicated folder-tree UI for document organization (story 7a-2) — owner: pm-frontend — why: no UI for already-built 5-level folder hierarchy.
- [high] Implement handler bodies for 10b-4/5/6/7 (system announcements, support data, onboarding tour, contextual help) — owner: pm-backend — why: routes mounted but handlers are stubs (dependency: also need admin-web UI per story).
- [medium] Ship web frontend for announcement viewing/acknowledgment, comments, pinning (stories 6-2/6-3/6-4) — owner: pm-frontend — why: backend done, web screens still `planned` not `shipped`.
- [medium] Verify + document mobile NeighborsScreen privacy integration (story 6-6) — owner: pm-frontend.
- [medium] Wire AnnouncementsPage/FaultsPage to API hooks (story 79-1) — owner: pm-frontend — why: 51 useQuery instances exist but key feature pages still on mocks.
- [medium] Add backend integration tests for OAuth provider (story 10a-x test gap) — owner: pm-backend — why: shipped but untested end-to-end.
- [medium] Ship dispute filing flow + mediation/resolution acceptance-criteria sweep (stories 80-2/80-3) — owner: pm-frontend — why: UI shipped but story task lists unchecked; verify AC coverage.
- [medium] Mobile env-var pipeline cleanup + iOS Info.plist injection (story 85-1) — owner: pm-frontend — why: Expo Constants used in lieu of `react-native-config`; iOS path missing keys.
- [medium] iOS xcconfig + Xcode schemes + app icon variants (story 85-2) — owner: pm-frontend — why: Android flavors done, iOS path incomplete.

### Phase 2

- [high] Implement backend pause/resume + execution-history endpoints for report schedules (stories 81-1/81-2) — owner: pm-backend — why: frontend already calls these endpoints and is shipped — currently 404s in prod path.
- [low] E-signature email integration (story 84-2) — owner: pm-backend — why: only story in `not-started`; Phase 2 scope, no dependents yet.

### Phase 3

- (No outstanding work — Epic 13 stories not yet decomposed; pgvector RAG 84-5 done.)

### Phase 4

- [medium] Re-base Epic 82 iOS stories against the actual `mobile-native/iosApp/` codebase — owner: pm-frontend — why: stories say pending but code exists; AC verification + screen-map entries needed.
- [medium] Airbnb OAuth token-exchange endpoint + webhook handler (story 83-1) — owner: pm-backend — why: models defined but no `/integrations/airbnb/*` routes.
- [medium] Booking.com OTA XML parsing + push endpoint (story 83-2) — owner: pm-backend — why: models defined, transport layer missing.

## Notes

- Stories 81-1/81-2 are technically Phase 2 but contain a production bug (frontend → missing backend endpoint). Treat as high priority despite phase weight.
- Epic 82 iOS stories scored "partial" with low confidence — the implementation files exist but story-to-code mapping is uncertain (epic numbering conflict with `epics-007.md`). Worth a 30-minute alignment pass before further planning.
- 7a-* sprint-status shows `ready-for-dev` despite merged PRs. Sprint-status is stale here — update once apiStatus is verified.

Buffer: 100/36 open · candidates ranked but unqueued: 0 (buffer full — no refill needed this run; upkeep advanced 80-3 → done from PR #555, re-checked epic-9, added 3 pm-devops risks)
