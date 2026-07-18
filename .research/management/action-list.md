# Action list

<sub>Last regenerated: 2026-07-18 09:00 UTC by routine</sub>


| ID | Action | Owner | Priority | Status | Deps |
|----|--------|-------|----------|--------|------|
| `pm-security-accept-invitation-email-binding` | Bind accept_invitation to the invited email (verify principal.email == invitation.email) — closes th | pm-security | high | open | pm-security-agency-invitation-membership-gate |
| `pm-security-agency-invitation-membership-gate` | Add agency-membership check to reality-server create_invitation handler (invited_by must be an owner | pm-security | high | open | - |
| `pm-security-reality-invitations-audit` | Audit reality_agency_invitations/reality_agency_members tables for rows created via this gap since d | pm-security | high | open | pm-security-agency-invitation-membership-gate |
| `pm-scrum-84-1-ppt-web-s3-presigned-upload-consumer` | Wire ppt-web direct-to-S3 upload flow to already-shipped POST /documents/upload-url endpoint (story  | pm-frontend | medium | open | - |
| `pm-scrum-84-2-ppt-web-document-sign-page` | Build the signer-facing document-sign page in ppt-web (story 84-2, mvp, partial — scope tightly to a | pm-frontend | medium | open | - |
| `pm-security-list-members-idor-audit` | Investigate reality-server list_members (agencies.rs:345) — no membership/principal check found; con | pm-security | medium | open | - |
| `pm-tech-lead-reality-server-total-page-length-bug` | Fix pagination `total = rows.len()` bug across reality-server (reports.rs:342, imports.rs:141+347, a | pm-tech-lead | medium | open | - |

**Total items:** 8 — **Open:** 7 · Done: 1
