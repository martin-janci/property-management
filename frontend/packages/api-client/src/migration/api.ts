/**
 * Migration Import API Client (Epic 90 - Frontend API Integration).
 * Originally from Epic 66, integrated as part of Epic 90.
 *
 * API functions for bulk data import functionality.
 */

import { getToken } from '../auth';
import type {
  ImportJobListResponse,
  ImportJobProgress,
  ImportJobResponse,
  ImportPreviewResponse,
  ImportTemplateListResponse,
  ImportTemplateResponse,
  RetryImportRequest,
  StartImportRequest,
  TemplateDownloadResponse,
} from './types';

const API_BASE = '/api/v1/import';

function getAuthHeaders(): HeadersInit {
  const token = getToken();
  return token ? { Authorization: `Bearer ${token}` } : {};
}

async function fetchApi<T>(url: string, options: RequestInit = {}): Promise<T> {
  const response = await fetch(url, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...getAuthHeaders(),
      ...options.headers,
    },
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({
      message: `Request failed with status ${response.status} ${response.statusText}`.trim(),
    }));
    throw new Error(error.message || `HTTP ${response.status} ${response.statusText}`.trim());
  }

  return response.json();
}

// Template operations
export async function listTemplates(): Promise<ImportTemplateListResponse> {
  return fetchApi(`${API_BASE}/templates`);
}

export async function getTemplate(id: string): Promise<ImportTemplateResponse> {
  return fetchApi(`${API_BASE}/templates/${id}`);
}

export async function downloadTemplate(
  id: string,
  format: 'csv' | 'xlsx'
): Promise<TemplateDownloadResponse> {
  return fetchApi(`${API_BASE}/templates/${id}/download?format=${format}`);
}

// Job operations
export async function listJobs(params?: {
  status?: string;
  limit?: number;
  offset?: number;
}): Promise<ImportJobListResponse> {
  const searchParams = new URLSearchParams();
  if (params?.status) searchParams.set('status', params.status);
  if (params?.limit) searchParams.set('limit', params.limit.toString());
  if (params?.offset) searchParams.set('offset', params.offset.toString());

  const queryString = searchParams.toString();
  return fetchApi(`${API_BASE}/jobs${queryString ? `?${queryString}` : ''}`);
}

export async function getJob(id: string): Promise<ImportJobResponse> {
  return fetchApi(`${API_BASE}/jobs/${id}`);
}

export async function getJobProgress(id: string): Promise<ImportJobProgress> {
  return fetchApi(`${API_BASE}/jobs/${id}/progress`);
}

export async function getJobErrors(
  id: string
): Promise<{ errors: Array<{ row: number; column: string; message: string }> }> {
  return fetchApi(`${API_BASE}/jobs/${id}/errors`);
}

// Upload and import
export async function uploadFile(
  templateId: string,
  file: File,
  onProgress?: (progress: number) => void
): Promise<ImportPreviewResponse> {
  const formData = new FormData();
  formData.append('templateId', templateId);
  formData.append('file', file);

  return new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest();

    xhr.upload.addEventListener('progress', (event) => {
      if (event.lengthComputable && onProgress) {
        const progress = Math.round((event.loaded / event.total) * 100);
        onProgress(progress);
      }
    });

    xhr.addEventListener('load', () => {
      if (xhr.status >= 200 && xhr.status < 300) {
        try {
          const response = JSON.parse(xhr.responseText);
          resolve(response as ImportPreviewResponse);
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
      reject(new Error('Network error occurred'));
    });

    xhr.addEventListener('abort', () => {
      reject(new Error('Upload was cancelled'));
    });

    xhr.open('POST', `${API_BASE}/upload`);

    const token = getToken();
    if (token) {
      xhr.setRequestHeader('Authorization', `Bearer ${token}`);
    }

    xhr.send(formData);
  });
}

export async function startImport(request: StartImportRequest): Promise<ImportJobResponse> {
  return fetchApi(`${API_BASE}/jobs/${request.jobId}/start`, {
    method: 'POST',
    body: JSON.stringify({ acknowledgeWarnings: request.acknowledgeWarnings }),
  });
}

export async function retryImport(request: RetryImportRequest): Promise<ImportJobResponse> {
  return fetchApi(`${API_BASE}/jobs/${request.jobId}/retry`, {
    method: 'POST',
    body: JSON.stringify({ skipFailedRows: request.skipFailedRows }),
  });
}

export async function cancelImport(jobId: string): Promise<void> {
  await fetchApi(`${API_BASE}/jobs/${jobId}/cancel`, { method: 'POST' });
}
