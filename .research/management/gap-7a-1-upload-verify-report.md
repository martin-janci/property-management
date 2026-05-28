# Gap 7A-1 Upload Verify Report — 2026-05-25

## Verdict: NOT promotable — backend `/api/v1/documents/upload` endpoint is missing

### What was verified

| Layer | File | Status |
|-------|------|--------|
| Backend: document router | `backend/servers/api-server/src/routes/documents/core.rs` | `POST /` exists (JSON body, two-step flow) |
| Backend: `/upload` multipart | Searched all routes under `src/routes/documents/` | **MISSING** |
| Mobile (PR #447, merged) | `frontend/apps/mobile/src/screens/documents/DocumentUploadScreen.tsx` | Calls `POST /api/v1/documents/upload` (multipart) |
| Web api-client | `frontend/packages/api-client/src/documents/api.ts` | `uploadDocument()` calls `POST /api/v1/documents/upload` (XHR multipart) |
| ppt-web upload page | `frontend/apps/ppt-web/src/features/documents/pages/DocumentUploadPage.tsx` | Uses `useUploadDocument` hook → same missing endpoint |

### The gap

Both frontends (web + mobile) call `POST /api/v1/documents/upload` with `multipart/form-data` (file + title + category + description fields in one request).

The backend document router (`core.rs` + `mod.rs`) mounts only:
- `POST /api/v1/documents` — JSON body (`CreateDocumentRequest`), expects a `file_key` (S3 key already uploaded by client separately)
- No `POST /api/v1/documents/upload` multipart handler exists anywhere in the codebase

The backend has S3 integration (`backend/crates/integrations/src/storage.rs`) but no endpoint that accepts a raw file via multipart, uploads it to S3, and then creates the document record atomically.

### Band A verify results (this branch, no code changes)

```
cargo check -p api-server          → exit 0
pnpm -F @ppt/mobile typecheck      → exit 0
pnpm --filter @ppt/web typecheck   → exit 0
pnpm check (Biome)                 → exit 0, 15 pre-existing warnings only
```

### Sprint-status recommendation

**Cannot promote 7a-1 to done.** The upload flow will return HTTP 404/405 at runtime for both mobile and web clients. A follow-up backend task is needed:

> Implement `POST /api/v1/documents/upload` (multipart) in `backend/servers/api-server/src/routes/documents/core.rs`:
> 1. Accept `multipart/form-data` with fields: `file` (bytes), `title`, `category`, `description?`, `folder_id?`
> 2. Stream file to S3 via `StorageService`, derive `file_key`
> 3. Validate MIME type, size, category (same rules as `create_document`)
> 4. Insert document record via `document_repo.create_rls()`
> 5. Return `{ id, message }` — same shape as `CreateDocumentResponse`

This is owner: `rust-backend`, priority: high.
