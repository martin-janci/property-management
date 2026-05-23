# Action list

_Generated 2026-05-23 from `action-list.json` (merged: pm-analysis rotation + gap-scan)._

## From pm-analysis rotation (2026-05-23)

| Priority | Action | Owner | Dependency | Status | Deadline |
|---|---|---|---|---|---|
| high | Get a review decision on PR #435 (security auth/security fixes) and merge to dev | pm-scrum-master | none | open | — |
| high | Complete code review of Epic 8A (8a-1/8a-2/8a-3, all in review) and move to done | pm-scrum-master | none | open | — |
| high | Decide build order: land Epic 2B notification infra before Epic 6 publish + 8A dispatch, or formally defer those slices | pm-tech-lead | pm-scrum-master | open | — |
| high | Remove dead duplicate AuthHandler/BuildingHandler modules so security fixes cannot diverge between handler/route copies | pm-tech-lead | none | open | — |
| medium | Review and merge story 6.1 (announcement creation/targeting) | pm-scrum-master | none | open | — |
| medium | Sequence Epic 2B notification infrastructure ahead of Epic 6 publish + 8A.2 dispatch | pm-scrum-master | pm-tech-lead | open | — |
| medium | Split churn-hot route modules (integrations.rs, organizations.rs, documents.rs) by surface | pm-tech-lead | none | open | — |
| medium | Define OAuth provider (10A) token/storage/rotation design before 10a-1 pickup to avoid rework | pm-tech-lead | none | open | — |
| low | Land the security-voice-device-idor plan (ready in .research/plans/) | pm-scrum-master | none | open | — |
| low | Confirm WebSocket infra ownership for 8A.3 sync (ADR-008) is scheduled, not implicitly assumed | pm-tech-lead | pm-scrum-master | open | — |

## From gap-scan (2026-05-23, top 10)

| Priority | Action | Owner | Dependency | Status |
|---|---|---|---|---|
| high | Wire `TwoFactorAuthPage` to `/api/v1/auth/mfa/*` — add `useMfa` hooks to `@ppt/api-client` (9-1 TOTP) | pm-security | — | open |
| high | Build WebSocket realtime sync infra for notification preferences (8a-3) | pm-backend | Epic 2B WebSocket infra | open |
| high | Complete frontend API integration for documents permission-based access (7a-3) | pm-backend | — | open |
| high | Wire login form/logout/session cleanup to `AuthContext` (79-2) | pm-frontend | — | open |
| high | Implement mobile (RN) document upload UI with metadata (7a-1) | pm-frontend | — | open |
| high | Implement mobile document sharing UI (7a-5) | pm-frontend | — | open |
| high | Implement PDF.js client-side preview for documents (7a-4) | pm-frontend | — | open |
| high | Promote direct messaging screens from `apiStatus: stub` to integrated (6-5) | pm-frontend | — | open |
| high | Build dedicated folder-tree UI page for document organization (7a-2) | pm-frontend | — | open |
| high | Implement handler bodies for 10b-4/5/6/7 (route stubs only) | pm-backend | — | open |
