/**
 * Document API client (Epic 39).
 */

import type {
  ClassificationFeedback,
  ClassificationHistoryEntry,
  ClassificationResponse,
  CreateDocumentRequest,
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
  SummarizationResponse,
  UpdateDocumentRequest,
} from './types';

const API_BASE = '/api/v1/documents';

async function fetchApi<T>(url: string, options: RequestInit = {}): Promise<T> {
  const response = await fetch(url, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options.headers,
    },
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({ message: 'Request failed' }));
    throw new Error(error.message || `HTTP ${response.status}`);
  }

  return response.json();
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

export async function deleteFolder(id: string, cascade = false): Promise<{ message: string }> {
  return fetchApi(`${API_BASE}/folders/${id}?cascade=${cascade}`, {
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

    // Attach Authorization header if an access token is available
    try {
      const token =
        typeof window !== 'undefined' && window.localStorage
          ? window.localStorage.getItem('token')
          : null;
      if (token) {
        xhr.setRequestHeader('Authorization', `Bearer ${token}`);
      }
    } catch {
      // If accessing storage fails, proceed without auth header
    }

    xhr.send(formData);
  });
}
