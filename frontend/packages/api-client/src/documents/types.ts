/**
 * Document Intelligence types (Epic 39).
 */

// Base types
export interface Document {
  id: string;
  title: string;
  description?: string;
  category: string;
  folder_id?: string;
  file_key: string;
  file_name: string;
  mime_type: string;
  size_bytes: number;
  access_scope: AccessScope;
  created_at: string;
  updated_at: string;
  created_by: string;
  organization_id: string;
  building_id?: string;
  // Intelligence fields (Epic 28)
  ocr_status?: OcrStatus;
  ocr_text?: string;
  ocr_processed_at?: string;
  predicted_category?: string;
  classification_confidence?: number;
  classified_at?: string;
  classification_accepted?: boolean;
  summary?: string;
  summary_generated_at?: string;
}

export type AccessScope = 'organization' | 'building' | 'unit' | 'user' | 'public';
export type OcrStatus = 'pending' | 'processing' | 'completed' | 'failed' | 'not_applicable';

export interface DocumentSummary {
  id: string;
  title: string;
  description?: string;
  category: string;
  /** Owning folder id; absent/null means the document lives at the root. */
  folder_id?: string | null;
  file_name: string;
  mime_type: string;
  size_bytes: number;
  created_at: string;
  updated_at: string;
  ocr_status?: OcrStatus;
  predicted_category?: string;
  classification_confidence?: number;
  summary?: string;
  /** RLS-enforced audience scope returned from the server (7a-3). */
  access_scope?: AccessScope;
  /** Publication status; absent means published (backward compat) */
  status?: DocumentStatus;
}

export interface DocumentFolder {
  id: string;
  name: string;
  description?: string;
  parent_id?: string;
  organization_id: string;
  building_id?: string;
  created_at: string;
  updated_at: string;
}

export interface FolderWithCount extends DocumentFolder {
  document_count: number;
  subfolder_count: number;
}

export interface FolderTreeNode {
  folder: DocumentFolder;
  children: FolderTreeNode[];
  document_count: number;
}

// Search types (Story 28.2)
export interface DocumentSearchRequest {
  query: string;
  organization_id?: string;
  building_id?: string;
  categories?: string[];
  date_from?: string;
  date_to?: string;
  ocr_status?: OcrStatus[];
  has_summary?: boolean;
  /**
   * RLS-aware audience pre-filter (7a-3). The backend enforces row-level
   * security on top — callers cannot escalate access by omitting this field.
   */
  access_scope?: AccessScope;
  limit?: number;
  offset?: number;
}

export interface DocumentSearchResult {
  document: DocumentSummary;
  score: number;
  highlights: SearchHighlight[];
}

export interface SearchHighlight {
  field: 'title' | 'description' | 'ocr_text' | 'summary';
  snippet: string;
}

export interface DocumentSearchResponse {
  results: DocumentSearchResult[];
  total: number;
  took_ms: number;
}

// Classification types (Story 28.3)
export interface ClassificationResponse {
  document_id: string;
  predicted_category?: string;
  confidence?: number;
  classified_at?: string;
  accepted?: boolean;
}

export interface ClassificationFeedback {
  accepted: boolean;
  correct_category?: string;
}

export interface ClassificationHistoryEntry {
  id: string;
  predicted_category: string;
  confidence: number;
  classified_at: string;
  feedback_accepted?: boolean;
  feedback_correct_category?: string;
  feedback_at?: string;
}

// Summarization types (Story 28.4)
export interface GenerateSummaryRequest {
  max_length?: number;
  style?: 'brief' | 'detailed' | 'bullets';
}

export interface SummarizationResponse {
  message: string;
  queue_id: string;
}

export interface DocumentSummaryView {
  document_id: string;
  summary: string;
  key_points?: string[];
  generated_at: string;
}

// OCR types (Story 28.1)
export interface OcrReprocessResponse {
  message: string;
  queue_id?: string;
}

// Intelligence stats
export interface DocumentIntelligenceStats {
  organization_id: string;
  total_documents: number;
  ocr_completed: number;
  ocr_pending: number;
  ocr_failed: number;
  classified_documents: number;
  summarized_documents: number;
  avg_classification_confidence: number;
}

// Document status (publication state)
export type DocumentStatus = 'published' | 'draft' | 'archived';

// List query params
export interface DocumentListQuery {
  folder_id?: string;
  category?: string;
  search?: string;
  /**
   * RLS-aware audience pre-filter: the backend further restricts results to
   * documents the caller is permitted to see regardless of this filter.
   */
  access_scope?: AccessScope;
  limit?: number;
  offset?: number;
  /** Filter by publication status (maps to document state machine on backend) */
  status?: DocumentStatus;
  created_by?: string;
}

// Responses
export interface DocumentListResponse {
  documents: DocumentSummary[];
  count: number;
  total: number;
}

export interface CreateDocumentRequest {
  title: string;
  description?: string;
  category: string;
  folder_id?: string;
  file_key: string;
  file_name: string;
  mime_type: string;
  size_bytes: number;
  access_scope?: AccessScope;
  access_target_ids?: string[];
  access_roles?: string[];
}

export interface UpdateDocumentRequest {
  title?: string;
  description?: string;
  category?: string;
  folder_id?: string;
  access_scope?: AccessScope;
  access_target_ids?: string[];
  access_roles?: string[];
}

// Document categories
export const DOCUMENT_CATEGORIES = [
  'contract',
  'invoice',
  'receipt',
  'report',
  'minutes',
  'policy',
  'notice',
  'maintenance',
  'insurance',
  'legal',
  'financial',
  'correspondence',
  'other',
] as const;

export type DocumentCategory = (typeof DOCUMENT_CATEGORIES)[number];

// --- Document Sharing types (Story 7A.5) ---

export const SHARE_TYPE = {
  USER: 'user',
  ROLE: 'role',
  BUILDING: 'building',
  LINK: 'link',
} as const;

export type ShareType = (typeof SHARE_TYPE)[keyof typeof SHARE_TYPE];

export interface DocumentShare {
  id: string;
  document_id: string;
  share_type: ShareType;
  target_id?: string | null;
  target_role?: string | null;
  shared_by: string;
  share_token?: string | null;
  expires_at?: string | null;
  revoked_at?: string | null;
  created_at: string;
}

export interface ShareWithDocument extends DocumentShare {
  document_title: string;
  shared_by_name: string;
}

export interface ShareListResponse {
  shares: ShareWithDocument[];
}

// --- Document Versions types (Story 7B.1, #974) ---

/**
 * A single version row as returned by the backend version-history endpoint
 * (`GET /api/v1/documents/{id}/versions`). Mirrors the Rust `DocumentVersion`
 * model — snake_case, with `created_by_name` resolved server-side from the
 * uploader's profile and `is_current_version` flagging the live version.
 */
export interface DocumentVersion {
  id: string;
  version_number: number;
  is_current_version: boolean;
  file_key: string;
  file_name: string;
  mime_type: string;
  size_bytes: number;
  created_by: string;
  created_by_name: string;
  created_at: string;
}

/**
 * Inner `history` object of the version-history response. Mirrors the Rust
 * `DocumentVersionHistory` model.
 */
export interface DocumentVersionHistory {
  document_id: string;
  title: string;
  total_versions: number;
  versions: DocumentVersion[];
}

/**
 * Response envelope for `GET /api/v1/documents/{id}/versions`. Mirrors the Rust
 * `VersionHistoryResponse { history }` wrapper. The generated OpenAPI client
 * models this endpoint as a bare camelCase array, so callers that need the real
 * backend shape should type against this instead.
 */
export interface VersionHistoryResponse {
  history: DocumentVersionHistory;
}

export interface CreateShareRequest {
  share_type: ShareType;
  target_id?: string;
  target_role?: string;
  password?: string;
  expires_at?: string;
}

export interface CreateShareResponse {
  id: string;
  share_token?: string | null;
  share_url?: string | null;
  message: string;
}
