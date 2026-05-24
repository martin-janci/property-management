# Action list

_Generated 2026-05-24 from `action-list.json` (merged: pm-analysis rotation + gap-scan)._

## From pm-analysis rotation

| Priority | Action | Owner | Dependency | Status |
|---|---|---|---|---|
| high | Get a review decision on PR #435 (security auth/security fixes) and merge to dev | pm-scrum-master | none | done |
| high | Complete code review of Epic 8A (8a-1/8a-2/8a-3) and move to done | pm-scrum-master | none | done |
| high | Remove dead duplicate AuthHandler/BuildingHandler modules | pm-tech-lead | none | done |
| high | Resolve post-merge security findings #438/#439 (SSRF, Debug-hash, cookie scope, ordering, IG3) | pm-security | none | open |
| high | Fix P1-05 SSRF — shared validate_external_url at signatures.rs:628 + integrations.rs:2743 | pm-backend | none | open |
| high | Implement Epic 81 report-schedule endpoints frontend already calls (pause/resume/executions) | pm-backend | none | open |
| high | Decide build order: Epic 2B infra before Epic 6 publish + 8A dispatch, or defer | pm-tech-lead | pm-scrum-master | done |
| medium | Review and merge story 6.1 (announcement creation/targeting) | pm-scrum-master | none | done |
| medium | Sequence Epic 2B notification infrastructure ahead of Epic 6 publish + 8A.2 dispatch | pm-scrum-master | pm-tech-lead | done |
| medium | Split churn-hot route modules (integrations.rs, organizations.rs, documents.rs) by surface | pm-tech-lead | none | open |
| medium | Define OAuth provider (10A) token/storage/rotation design before 10a-1 pickup | pm-tech-lead | none | open |
| medium | Harden ProtectedRoute.tsx:117 fail-open role guard + populate user.role (fold into 79-2) | pm-frontend | none | open |
| low | Land the security-voice-device-idor plan (ready in .research/plans/) | pm-scrum-master | none | open |
| low | Confirm WebSocket infra ownership for 8A.3 sync (ADR-008) is scheduled | pm-tech-lead | pm-scrum-master | open |

## From gap-scan

| Priority | Action | Owner | Dependency | Status |
|---|---|---|---|---|
| high | Wire `TwoFactorAuthPage` to `/api/v1/auth/mfa/*` — add `useMfa` hooks (9-1 TOTP) | pm-security | — | in-progress |
| high | Build WebSocket realtime sync infra for notification preferences (8a-3) | pm-backend | Epic 2B WebSocket infra | open |
| high | Complete frontend API integration for documents permission-based access (7a-3) | pm-backend | — | in-progress |
| high | Wire login form/logout/session cleanup to `AuthContext` (79-2) | pm-frontend | — | in-progress |
| high | Implement mobile (RN) document upload UI with metadata (7a-1) | pm-frontend | — | in-progress |
| high | Implement mobile document sharing UI (7a-5) | pm-frontend | — | in-progress |
| high | Implement PDF.js client-side preview for documents (7a-4) | pm-frontend | — | done |
| high | Promote direct messaging screens from `apiStatus: stub` to integrated (6-5) | pm-frontend | — | open |
| high | Build dedicated folder-tree UI page for document organization (7a-2) | pm-frontend | — | open |
| high | Implement handler bodies for 10b-4/5/6/7 (route stubs only; return 501 until real) | pm-backend | — | open |
