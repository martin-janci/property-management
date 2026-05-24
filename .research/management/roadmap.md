# PPT Project Roadmap (Deep Scan)

_Generated: 2026-05-23 (deep scan) · upkeep-refreshed 2026-05-24 — supersedes `_bmad-output/implementation-artifacts/gap-analysis-remediation.md` (Epic 86, stale)._

_Upkeep 2026-05-24: gap-7a-4 (PDF.js client-side preview) shipped in PR #446 — web slice done; 7a-4 stays `partial` pending the mobile preview slice. Rotating epic re-checked: epic-10a (status unchanged — backend done, admin UI/tests still gaps)._

## State of the project

| Status | Count | Share |
|---|---:|---:|
| done | 17 | 35% |
| partial | 31 | 63% |
| not-started | 1 | 2% |
| **total** | **49** | 100% |

**Phase breakdown:** MVP 31 stories (8 done, 23 partial), Phase 2 3 stories (0 done, 2 partial, 1 not-started), Phase 3 1 story (done), Phase 4 11 stories (3 done, 8 partial).

**Platform breakdown (a story may span multiple):** backend ~35 stories touched, frontend (web) ~30, mobile (RN + iOS) ~12. Mobile is the most-behind platform — Epic 7A mobile slices, Epic 82 (iOS), and Epic 85 (build/env config) are all `partial` with low/medium confidence.

**Top 3 gap clusters**
1. **Frontend API integration debt.** Many ppt-web screens are `buildStatus: shipped` but `apiStatus: stub` (Epics 6 + 7A — ~11 stories). UIs render but don't fully invoke real backend endpoints. Highest blast radius for a "looks done, isn't" failure mode.
2. **Foundational infra not delivered.** WebSocket realtime sync (needed by 8a-3, 6-5, plus broader notification flows) is deferred, blocking story closure across communications/notifications. 2FA endpoints exist but `TwoFactorAuthPage` is a UI scaffold — feature is silently disabled end-to-end.
3. **Backend handler stubs masquerading as routes.** Epic 10B stories 4–7 (system announcements, support data access, onboarding tours, contextual help) have routes mounted in `main.rs` but handler bodies are stubs. Similar pattern in 81-1/81-2 where the frontend calls pause/resume + execution-history endpoints that don't exist in `reports.rs`.

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
- 7a-* sprint-status shows `ready-for-dev` despite merged PR #6 (`5cf52608`). Sprint-status is stale here — update once apiStatus is verified.
