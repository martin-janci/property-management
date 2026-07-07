# Story 7A.5: Document Sharing

Status: done

## Story

As a **property manager**,
I want to **share documents with specific users or groups**,
So that **relevant parties can access them**.

## Acceptance Criteria

1. **AC-1: Share with Recipients**
   - Given a manager shares a document
   - When they select recipients (users, roles, buildings)
   - Then recipients receive notification
   - And the document appears in their shared documents

2. **AC-2: Share Link Generation**
   - Given a share link is generated
   - When a valid link is accessed
   - Then the document is viewable/downloadable
   - And access is logged

3. **AC-3: Share Revocation**
   - Given a share is revoked
   - When the revocation is processed
   - Then recipients lose access immediately
   - And share links stop working

## Tasks / Subtasks

- [ ] Task 1: Database Schema & Migrations (AC: 1, 2, 3)
  - [ ] 1.1 Create `document_shares` table: id, document_id, share_type (user, role, building, link), target_id (nullable), shared_by, share_token (for links), password_hash (optional), expires_at, revoked_at, created_at
  - [ ] 1.2 Create `document_share_access_log` table: id, share_id, accessed_by (nullable for link), accessed_at, ip_address
  - [ ] 1.3 Add RLS policies for share management
  - [ ] 1.4 Add indexes: idx_shares_document, idx_shares_token, idx_shares_target

- [ ] Task 2: Backend Domain Models & Repository (AC: 1, 2, 3)
  - [ ] 2.1 Create DocumentShare, ShareType enum, CreateShare, RevokeShare models
  - [ ] 2.2 Implement ShareRepository with CRUD operations
  - [ ] 2.3 Add methods: find_by_token, find_by_document, check_share_access, log_access
  - [ ] 2.4 Implement share token generation (secure random)

- [ ] Task 3: Backend API Handlers (AC: 1, 2, 3)
  - [ ] 3.1 Create POST `/api/v1/documents/{id}/share` handler for creating shares
  - [ ] 3.2 Create GET `/api/v1/documents/{id}/shares` handler for listing shares
  - [ ] 3.3 Create DELETE `/api/v1/documents/{id}/shares/{share_id}` handler for revocation
  - [ ] 3.4 Create GET `/api/v1/documents/shared/{token}` handler for link access (no auth required)
  - [ ] 3.5 Create POST `/api/v1/documents/shared/{token}/access` for password-protected links
  - [ ] 3.6 Add notification trigger on share creation (deferred to Epic 2B)

- [ ] Task 4: TypeSpec API Specification (AC: 1, 2, 3)
  - [ ] 4.1 Define DocumentShare model in TypeSpec
  - [ ] 4.2 Define CreateShareRequest with share_type, target_id, password, expires_at
  - [ ] 4.3 Define ShareLinkResponse with url and metadata
  - [ ] 4.4 Document all endpoints

- [ ] Task 5: Frontend Components - ppt-web (AC: 1, 2, 3)
  - [ ] 5.1 Create ShareDocumentDialog component
  - [ ] 5.2 Create ShareTypeSelector (users, roles, buildings, link)
  - [ ] 5.3 Create ShareLinkGenerator with copy-to-clipboard
  - [ ] 5.4 Create SharesList component showing active shares
  - [ ] 5.5 Create PasswordProtectionToggle for link shares
  - [ ] 5.6 Create SharedDocumentView for public link access

- [ ] Task 6: Frontend State & API Integration (AC: 1, 2, 3)
  - [ ] 6.1 Create useDocumentShares hook
  - [ ] 6.2 Create useCreateShare mutation hook
  - [ ] 6.3 Create useRevokeShare mutation hook
  - [ ] 6.4 Create useAccessSharedDocument hook (for public links)

- [ ] Task 7: Integration Testing (AC: 1, 2, 3)
  - [ ] 7.1 Write backend tests for share creation by type
  - [ ] 7.2 Write backend tests for share link access
  - [ ] 7.3 Write backend tests for password-protected links
  - [ ] 7.4 Write backend tests for share revocation
  - [ ] 7.5 Write backend tests for access logging

## Dev Notes

### Architecture Requirements
- Share types: user, role, building, link
- Link shares can have optional password protection
- All link accesses are logged with timestamp and IP
- Revocation is immediate - no grace period

### Technical Specifications
- Share tokens: 32-character secure random string
- Password hashing: Argon2id (same as user passwords)
- Link format: `/shared/{token}`

### Share Resolution
1. User shares: target_id = user UUID
2. Role shares: target_id = role enum value (stored as string)
3. Building shares: target_id = building UUID
4. Link shares: no target_id, uses share_token

### Security Considerations
- Rate limit link access attempts
- Log all access for audit
- Expired/revoked links return 404 (not 403) for security
- Password-protected links require POST to access

### Project Structure Notes

**Backend files to create/modify:**
- `backend/crates/db/migrations/00023_create_document_shares.sql`
- `backend/crates/db/src/models/document_share.rs`
- `backend/crates/db/src/repositories/document_share.rs`
- `backend/servers/api-server/src/routes/documents.rs` (add share routes)

**Frontend files to create:**
- `frontend/apps/ppt-web/src/features/documents/components/ShareDocumentDialog.tsx`
- `frontend/apps/ppt-web/src/features/documents/components/SharesList.tsx`
- `frontend/apps/ppt-web/src/features/documents/components/ShareLinkGenerator.tsx`
- `frontend/apps/ppt-web/src/features/documents/pages/SharedDocumentPage.tsx`

### References

- [Source: _bmad-output/epics.md#Epic-7A-Story-7A.5]
- [Source: Story 7A.3 for access patterns]

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

N/A

### Completion Notes List

(To be filled during implementation)

### File List

(To be filled during implementation)

## Change Log

| Date | Change |
|------|--------|
| 2025-12-21 | Story created |
