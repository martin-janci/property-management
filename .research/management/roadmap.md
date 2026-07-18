# Roadmap

<sub>Regenerated 2026-07-18 09:00 UTC from `coverage.json` + `action-list.json`</sub>


## State of the project
- **Done:** 47 · **Partial:** 2 · **Not-started:** 0 · **Total:** 49
- **Done by platform:** backend=32, frontend=37, mobile=10
- **Partial by platform:** backend=2, frontend=2, mobile=0
- **Top 3 gaps:**
  - 84-1-s3-presigned-urls: ppt-web upload flow still server-proxied — wire direct-to-S3 upload via POST /documents/upload-url (api-client binding + UploadDocument integration; endpoint landed in #2309)
  - 84-2-esignature-email: build the signer-facing document-sign page in ppt-web (screen-map ppt/document-sign buildStatus planned→shipped; verify signature-request email delivery end-to-end; NOTE: prior task gap-84-2-no-frontend-ui-component-consuming-the failed with no PR — fresh attempt, scope to the shipped API)
- **Screen coverage:** 25 stories without screen-map · 0 orphan epics · 0 orphan screens · 3 missing UC links

## Ranked plan

### Phase: mvp

- [high] Bind accept_invitation to the invited email (verify principal.email == invitation.email) — closes the second half of the cross-agency escalation chain — owner: pm-security — id: `pm-security-accept-invitation-email-binding`
- [high] Add agency-membership check to reality-server create_invitation handler (invited_by must be an owner/admin member of agency_id) before merge freeze — reality-server security-fast-track hotfix candidate — owner: pm-security — id: `pm-security-agency-invitation-membership-gate`
- [high] Audit reality_agency_invitations/reality_agency_members tables for rows created via this gap since deploy; revoke/expire any unexpected memberships — owner: pm-security — id: `pm-security-reality-invitations-audit`
- [medium] Wire ppt-web direct-to-S3 upload flow to already-shipped POST /documents/upload-url endpoint (story 84-1, mvp, partial) — owner: pm-frontend — id: `pm-scrum-84-1-ppt-web-s3-presigned-upload-consumer`
- [medium] Build the signer-facing document-sign page in ppt-web (story 84-2, mvp, partial — scope tightly to already-shipped backend API) — owner: pm-frontend — id: `pm-scrum-84-2-ppt-web-document-sign-page`
- [medium] Investigate reality-server list_members (agencies.rs:345) — no membership/principal check found; confirm if intended public directory or another IDOR — owner: pm-security — id: `pm-security-list-members-idor-audit`
- [medium] Fix pagination `total = rows.len()` bug across reality-server (reports.rs:342, imports.rs:141+347, agency_imports.rs:183) — clients paginating via total/limit only see first page — owner: pm-tech-lead — id: `pm-tech-lead-reality-server-total-page-length-bug`

⚠ Buffer below half — consider running scan to refresh coverage
Buffer: 7/36 open · 2 candidates ranked but unqueued
