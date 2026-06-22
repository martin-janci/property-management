/**
 * Document template API client (Epic 7B, Story 7B.2).
 *
 * Endpoints (hand-written — NOT TypeSpec-generated):
 *   GET    /api/v1/templates
 *   GET    /api/v1/templates/{id}
 *   POST   /api/v1/templates
 *   PUT    /api/v1/templates/{id}
 *   DELETE /api/v1/templates/{id}
 *   POST   /api/v1/templates/{id}/generate
 */

import { getToken } from '../auth';
import type {
  CreateTemplateRequest,
  CreateTemplateResponse,
  GenerateDocumentRequest,
  GenerateDocumentResponse,
  ListDocumentTemplatesParams,
  TemplateActionResponse,
  TemplateDetailResponse,
  TemplateListResponse,
  UpdateTemplateRequest,
} from './types';

const TEMPLATES_BASE = '/api/v1/templates';

function getAuthHeaders(): HeadersInit {
  const token = getToken();
  return token ? { Authorization: `Bearer ${token}` } : {};
}

async function apiRequest<T>(url: string, options: RequestInit = {}): Promise<T> {
  const response = await fetch(url, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...getAuthHeaders(),
      ...options.headers,
    },
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({}));
    throw new Error(error.message || `HTTP ${response.status}`);
  }

  // 204 No Content (e.g. DELETE) has no JSON body.
  if (response.status === 204 || response.headers.get('content-length') === '0') {
    return undefined as T;
  }

  return response.json();
}

function buildQuery(params?: ListDocumentTemplatesParams): string {
  if (!params) return '';
  const search = new URLSearchParams();
  if (params.template_type) search.set('template_type', params.template_type);
  if (params.search) search.set('search', params.search);
  if (params.limit != null) search.set('limit', String(params.limit));
  if (params.offset != null) search.set('offset', String(params.offset));
  const qs = search.toString();
  return qs ? `?${qs}` : '';
}

/**
 * List document templates.
 * GET /api/v1/templates
 */
export async function listDocumentTemplates(
  params?: ListDocumentTemplatesParams
): Promise<TemplateListResponse> {
  return apiRequest<TemplateListResponse>(`${TEMPLATES_BASE}${buildQuery(params)}`);
}

/**
 * Get a single template (with placeholders + content) by ID.
 * GET /api/v1/templates/{id}
 */
export async function getDocumentTemplate(id: string): Promise<TemplateDetailResponse> {
  return apiRequest<TemplateDetailResponse>(`${TEMPLATES_BASE}/${id}`);
}

/**
 * Generate a document from a template.
 * POST /api/v1/templates/{id}/generate
 */
export async function generateDocument(
  id: string,
  body: GenerateDocumentRequest
): Promise<GenerateDocumentResponse> {
  return apiRequest<GenerateDocumentResponse>(`${TEMPLATES_BASE}/${id}/generate`, {
    method: 'POST',
    body: JSON.stringify(body),
  });
}

/**
 * Create a new template (manager-only on the server).
 * POST /api/v1/templates
 */
export async function createTemplate(body: CreateTemplateRequest): Promise<CreateTemplateResponse> {
  return apiRequest<CreateTemplateResponse>(TEMPLATES_BASE, {
    method: 'POST',
    body: JSON.stringify(body),
  });
}

/**
 * Update an existing template.
 * PUT /api/v1/templates/{id}
 */
export async function updateTemplate(
  id: string,
  body: UpdateTemplateRequest
): Promise<TemplateActionResponse> {
  return apiRequest<TemplateActionResponse>(`${TEMPLATES_BASE}/${id}`, {
    method: 'PUT',
    body: JSON.stringify(body),
  });
}

/**
 * Delete (soft-delete) a template.
 * DELETE /api/v1/templates/{id}
 */
export async function deleteTemplate(id: string): Promise<void> {
  await apiRequest<unknown>(`${TEMPLATES_BASE}/${id}`, { method: 'DELETE' });
}
