/**
 * Document API client (Epic 39).
 */

import { getToken } from '../auth';
import { authenticatedFetchJson } from '../lib/fetch';
import type {
  ClassificationFeedback,
  ClassificationHistoryEntry,
  ClassificationResponse,
  CreateDocumentRequest,
  CreateShareRequest,
  CreateShareResponse,
  CreateUploadUrlRequest,
  CreateUploadUrlResponse,
  Document,
  DocumentIntelligenceStats,
  DocumentListQuery,
  DocumentListResponse,
  DocumentSearchRequest,
  DocumentSearchResponse,
  FolderTreeNode,
  FolderWithCount,
  GenerateSummaryRequest,
  OcrReprocessResponse,
  ShareListResponse,
  SummarizationResponse,
  UpdateDocumentRequest,
} from './types';

const API_BASE = '/api/v1/documents';

/**
 * All document requests go through the shared authenticated transport
 * (`authenticatedFetchJson`) so they:
 *   - carry the `Authorization: Bearer …` header from the registered token
 *     provider — document routes are auth-protected and previously 401'd
 *     because this local transport never set the header (#751),
 *   - handle `204 No Content` uniformly (e.g. `deleteFolder`,
 *     `revokeDocumentShare`) without trying to `JSON.parse` an empty body,
 *   - share the same 401/MFA-retry and error-normalisation behaviour as the
 *     rest of the client.
 */
async function fetchApi<T>(url: string, options: RequestInit = {}): Promise<T> {
  return authenticatedFetchJson<T>(url, options);
}

// Document CRUD
export async function createDocument(
  data: CreateDocumentRequest
): Promise<{ id: string; message: string }> {
  return fetchApi(`${API_BASE}`, {
    method: 'POST',
    body: JSON.stringify(data),
  });
}

export async function listDocuments(query?: DocumentListQuery): Promise<DocumentListResponse> {
  const params = new URLSearchParams();
  if (query?.folder_id) params.set('folder_id', query.folder_id);
  if (query?.category) params.set('category', query.category);
  if (query?.search) params.set('search', query.search);
  // RLS-aware audience pre-filter (7a-3): server enforces RLS on top of this.
  if (query?.access_scope) params.set('access_scope', query.access_scope);
  if (query?.limit) params.set('limit', query.limit.toString());
  if (query?.offset) params.set('offset', query.offset.toString());
  if (query?.status) params.set('status', query.status);
  if (query?.created_by) params.set('created_by', query.created_by);

  const queryString = params.toString();
  return fetchApi(`${API_BASE}${queryString ? `?${queryString}` : ''}`);
}

export async function getDocument(id: string): Promise<{ document: Document }> {
  return fetchApi(`${API_BASE}/${id}`);
}

export async function updateDocument(
  id: string,
  data: UpdateDocumentRequest
): Promise<{ message: string; document: Document }> {
  return fetchApi(`${API_BASE}/${id}`, {
    method: 'PUT',
    body: JSON.stringify(data),
  });
}

export async function deleteDocument(id: string): Promise<void> {
  await fetchApi(`${API_BASE}/${id}`, { method: 'DELETE' });
}

// Download/Preview
export async function getDownloadUrl(id: string): Promise<{ url: string; expires_at: string }> {
  return fetchApi(`${API_BASE}/${id}/download`);
}

export async function getPreviewUrl(id: string): Promise<{ url: string; expires_at: string }> {
  return fetchApi(`${API_BASE}/${id}/preview`);
}

// Folders
export async function listFolders(buildingId?: string): Promise<{ folders: FolderWithCount[] }> {
  const params = buildingId ? `?building_id=${buildingId}` : '';
  return fetchApi(`${API_BASE}/folders${params}`);
}

export async function getFolderTree(buildingId?: string): Promise<{ tree: FolderTreeNode[] }> {
  const params = buildingId ? `?building_id=${buildingId}` : '';
  return fetchApi(`${API_BASE}/folders/tree${params}`);
}

export interface CreateFolderRequest {
  name: string;
  description?: string;
  parent_id?: string;
  building_id?: string;
}

export interface UpdateFolderRequest {
  name?: string;
  description?: string;
  parent_id?: string | null;
}

export async function createFolder(
  data: CreateFolderRequest
): Promise<{ id: string; message: string; folder: import('./types').DocumentFolder }> {
  return fetchApi(`${API_BASE}/folders`, {
    method: 'POST',
    body: JSON.stringify(data),
  });
}

export async function updateFolder(
  id: string,
  data: UpdateFolderRequest
): Promise<{ message: string; folder: import('./types').DocumentFolder }> {
  return fetchApi(`${API_BASE}/folders/${id}`, {
    method: 'PUT',
    body: JSON.stringify(data),
  });
}

// Backend `delete_folder` returns 204 No Content — the shared transport
// resolves to `undefined`, so the return type is `void` (was `{ message }`,
// which would have rejected on the empty 204 body, #751).
export async function deleteFolder(id: string, cascade = false): Promise<void> {
  await fetchApi(`${API_BASE}/folders/${id}?cascade=${cascade}`, {
    method: 'DELETE',
  });
}

export async function moveDocument(
  documentId: string,
  folderId: string | null
): Promise<{ message: string }> {
  return fetchApi(`${API_BASE}/${documentId}/move`, {
    method: 'POST',
    body: JSON.stringify({ folder_id: folderId }),
  });
}

// Document Intelligence (Epic 28)

// Story 28.1: OCR
export async function reprocessOcr(id: string): Promise<OcrReprocessResponse> {
  return fetchApi(`${API_BASE}/${id}/ocr/reprocess`, { method: 'POST' });
}

// Story 28.2: Full-text search
export async function searchDocuments(
  request: DocumentSearchRequest
): Promise<DocumentSearchResponse> {
  return fetchApi(`${API_BASE}/search`, {
    method: 'POST',
    body: JSON.stringify(request),
  });
}

// Story 28.3: Classification
export async function getClassification(id: string): Promise<ClassificationResponse> {
  return fetchApi(`${API_BASE}/${id}/classification`);
}

export async function submitClassificationFeedback(
  id: string,
  feedback: ClassificationFeedback
): Promise<{ message: string }> {
  return fetchApi(`${API_BASE}/${id}/classification/feedback`, {
    method: 'POST',
    body: JSON.stringify(feedback),
  });
}

export async function getClassificationHistory(
  id: string
): Promise<{ history: ClassificationHistoryEntry[] }> {
  return fetchApi(`${API_BASE}/${id}/classification/history`);
}

// Story 28.4: Summarization
export async function requestSummarization(
  id: string,
  options?: GenerateSummaryRequest
): Promise<SummarizationResponse> {
  return fetchApi(`${API_BASE}/${id}/summarize`, {
    method: 'POST',
    body: JSON.stringify(options || {}),
  });
}

// Intelligence stats
export async function getIntelligenceStats(): Promise<{
  stats: DocumentIntelligenceStats[];
}> {
  return fetchApi(`${API_BASE}/intelligence/stats`);
}

// Upload document with file (Story 39.2)
export interface UploadDocumentParams {
  file: File;
  title: string;
  description?: string;
  category: string;
  organizationId: string;
  buildingId?: string;
  folderId?: string;
  onProgress?: (progress: number) => void;
}

export async function uploadDocument(
  params: UploadDocumentParams
): Promise<{ id: string; message: string }> {
  const formData = new FormData();
  formData.append('file', params.file);
  formData.append('title', params.title);
  if (params.description) formData.append('description', params.description);
  formData.append('category', params.category);
  formData.append('organization_id', params.organizationId);
  if (params.buildingId) formData.append('building_id', params.buildingId);
  if (params.folderId) formData.append('folder_id', params.folderId);

  return new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest();

    xhr.upload.addEventListener('progress', (event) => {
      if (event.lengthComputable && params.onProgress) {
        const progress = Math.round((event.loaded / event.total) * 100);
        params.onProgress(progress);
      }
    });

    xhr.addEventListener('load', () => {
      if (xhr.status >= 200 && xhr.status < 300) {
        try {
          const response = JSON.parse(xhr.responseText);
          // Validate response structure
          if (
            !response ||
            typeof response !== 'object' ||
            typeof (response as { id?: unknown }).id !== 'string' ||
            typeof (response as { message?: unknown }).message !== 'string'
          ) {
            throw new Error('Invalid response structure');
          }
          resolve(response as { id: string; message: string });
        } catch {
          reject(new Error('Invalid response format'));
        }
      } else {
        try {
          const error = JSON.parse(xhr.responseText);
          reject(new Error(error.message || `HTTP ${xhr.status}`));
        } catch {
          reject(new Error(`HTTP ${xhr.status}`));
        }
      }
    });

    xhr.addEventListener('error', () => {
      reject(
        new Error('Network connection lost. Please check your internet connection and try again.')
      );
    });

    xhr.addEventListener('abort', () => {
      reject(new Error('Upload was cancelled.'));
    });

    xhr.open('POST', `${API_BASE}/upload`);

    // Attach the bearer token from the registered token provider — the same
    // source the rest of the client uses. (Previously read the wrong
    // localStorage key `'token'` directly, so uploads went out unauthenticated
    // and 401'd, #751.)
    const token = getToken();
    if (token) {
      xhr.setRequestHeader('Authorization', `Bearer ${token}`);
    }

    xhr.send(formData);
  });
}

// --- Direct-to-S3 upload (gap-84-1) ---

/**
 * Request a presigned PUT URL for a direct client-to-S3 upload
 * (`POST /api/v1/documents/upload-url`).
 *
 * Goes through the shared authenticated transport (the endpoint is
 * auth-protected + tenant-scoped; the org is derived server-side from the JWT).
 * The returned `url` is a short-lived (5 min) presigned S3 URL — the bytes are
 * PUT straight there via {@link uploadFileToPresignedUrl}, bypassing the
 * api-server byte proxy.
 */
export async function createUploadUrl(
  data: CreateUploadUrlRequest
): Promise<CreateUploadUrlResponse> {
  return fetchApi(`${API_BASE}/upload-url`, {
    method: 'POST',
    body: JSON.stringify(data),
  });
}

/**
 * Best-effort delete of a directly-uploaded S3 object by its `file_key` (#2564).
 *
 * Compensating action for {@link uploadDocumentDirect}: after the bytes are PUT
 * straight to S3 but registration (`POST /api/v1/documents`) fails, the object
 * is orphaned in the bucket. This calls the auth+org-scoped
 * `DELETE /api/v1/documents/by-file-key` route (which wraps `StorageService`
 * behind `validate_file_key_org_scope`) to reap it immediately.
 *
 * The `file_key` is passed as a URL-encoded query parameter (it contains `/`).
 * The route replies `204 No Content`, so the shared transport resolves to
 * `undefined`.
 */
export async function deleteUploadedFile(fileKey: string): Promise<void> {
  await fetchApi(`${API_BASE}/by-file-key?file_key=${encodeURIComponent(fileKey)}`, {
    method: 'DELETE',
  });
}

/**
 * PUT raw file bytes to a presigned S3 URL (gap-84-1).
 *
 * Uses `XMLHttpRequest` for upload-progress events. The request goes directly
 * to the S3-compatible host, so it MUST NOT carry the api-server `Authorization`
 * header — the presigned URL is the credential, and it is signed for exactly
 * the `contentType` passed here, which is set as the `Content-Type` header so
 * the request matches the signature.
 */
export function uploadFileToPresignedUrl(
  url: string,
  file: File,
  contentType: string,
  onProgress?: (progress: number) => void
): Promise<void> {
  return new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest();

    xhr.upload.addEventListener('progress', (event) => {
      if (event.lengthComputable && onProgress) {
        onProgress(Math.round((event.loaded / event.total) * 100));
      }
    });

    xhr.addEventListener('load', () => {
      if (xhr.status >= 200 && xhr.status < 300) {
        resolve();
      } else {
        reject(new Error(`Upload to storage failed (HTTP ${xhr.status})`));
      }
    });

    xhr.addEventListener('error', () => {
      reject(
        new Error('Network connection lost. Please check your internet connection and try again.')
      );
    });

    xhr.addEventListener('abort', () => {
      reject(new Error('Upload was cancelled.'));
    });

    xhr.open('PUT', url);
    // Match the Content-Type the URL was signed for — no auth header (the
    // presigned URL is the credential).
    xhr.setRequestHeader('Content-Type', contentType);
    xhr.send(file);
  });
}

/**
 * Upload a document directly to S3, then register it (gap-84-1).
 *
 * Three-step flow that replaces the byte-proxying multipart {@link uploadDocument}:
 *   1. `POST /api/v1/documents/upload-url` → presigned PUT URL + `file_key`.
 *   2. PUT the bytes straight to S3 (progress reported here).
 *   3. `POST /api/v1/documents` with the `file_key` to register the record.
 *
 * Accepts the same {@link UploadDocumentParams} shape as the legacy multipart
 * path so callers can switch with no call-site change. The organization is
 * derived server-side from the JWT, so `organizationId` is accepted but unused.
 */
export async function uploadDocumentDirect(
  params: UploadDocumentParams
): Promise<{ id: string; message: string }> {
  const presigned = await createUploadUrl({
    file_name: params.file.name,
    mime_type: params.file.type,
    size_bytes: params.file.size,
  });

  await uploadFileToPresignedUrl(
    presigned.url,
    params.file,
    presigned.content_type,
    params.onProgress
  );

  // Building association (#2366): the shipped registration contract
  // (`POST /api/v1/documents`, Rust `CreateDocumentRequest`) has NO `building_id`
  // column or field — building scope is expressed on the shipped API via
  // `access_scope='building'` + `access_target_ids=[buildingId]` (the JSONB
  // array the RLS gate and the building list/search filter both read). gap-84-1
  // switched `DocumentUpload` from the multipart proxy to this direct path and
  // dropped the association entirely; a raw `building_id` field never worked
  // either (the legacy multipart handler drained + ignored it, and the JSON
  // registration route has no such field). Forward it the shipped way so
  // building-scoped uploads keep their association. Only set the scope when a
  // building context was supplied — otherwise leave `access_scope` unset so the
  // server applies its default (organization) scope. `access_target_ids` is
  // required by the server whenever `access_scope='building'`, and we always
  // provide it in the same branch.
  const registration: CreateDocumentRequest = {
    title: params.title,
    description: params.description,
    category: params.category,
    folder_id: params.folderId,
    file_key: presigned.file_key,
    file_name: params.file.name,
    mime_type: presigned.content_type,
    size_bytes: params.file.size,
    ...(params.buildingId
      ? { access_scope: 'building' as const, access_target_ids: [params.buildingId] }
      : {}),
  };

  // ORPHANED-OBJECT CLEANUP (#2534, #2564): by this point step 2 has already PUT
  // the bytes into S3. If registration (step 3) throws — validation, auth, or a
  // dropped connection — the object is left in the bucket with no `documents`
  // row referencing it, so every failed/abandoned upload would leak storage.
  //
  // Deterministic compensating delete (#2564): call the auth+org-scoped
  // `DELETE /api/v1/documents/by-file-key` route (wraps `StorageService::delete`
  // behind `validate_file_key_org_scope`) to reap the orphan immediately. This
  // is best-effort — if the delete itself fails (network, storage unavailable,
  // race), we fall back to making the orphan observable in logs so it is
  // greppable and correlatable with the durable backstop below. Either way we
  // re-throw the ORIGINAL registration error unchanged so callers still see the
  // real failure.
  //
  // Durable backstop (the authorised alternative in #2534): an S3 bucket
  // LIFECYCLE RULE that expires objects under the org upload prefix after ~1
  // day still catches anything the best-effort delete misses (e.g. the client
  // process died before the catch ran). Safe because a successful upload
  // registers within seconds and the presigned PUT URL lives only ~5 min, so
  // any object older than a day with no matching `documents.file_key` is a
  // presigned-but-never-registered orphan. Configure once on the S3/MinIO
  // bucket (infra/DevOps).
  try {
    return await createDocument(registration);
  } catch (err) {
    try {
      await deleteUploadedFile(presigned.file_key);
    } catch (cleanupErr) {
      // Best-effort delete failed — leave a greppable trail for the lifecycle
      // sweep to reconcile. Do not mask the original registration error.
      console.warn(
        `[documents] registration failed after direct-to-S3 upload AND the compensating delete failed; orphaned object left at file_key="${presigned.file_key}" — it will be reaped by the bucket lifecycle rule (see #2534, #2564)`,
        cleanupErr
      );
    }
    throw err;
  }
}

// --- Document Sharing (Story 7A.5) ---

export async function listDocumentShares(documentId: string): Promise<ShareListResponse> {
  return fetchApi(`${API_BASE}/${documentId}/shares`);
}

export async function createDocumentShare(
  documentId: string,
  data: CreateShareRequest
): Promise<CreateShareResponse> {
  return fetchApi(`${API_BASE}/${documentId}/shares`, {
    method: 'POST',
    body: JSON.stringify(data),
  });
}

export async function revokeDocumentShare(documentId: string, shareId: string): Promise<void> {
  await fetchApi(`${API_BASE}/${documentId}/shares/${shareId}`, { method: 'DELETE' });
}
