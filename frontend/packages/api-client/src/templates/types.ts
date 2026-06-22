/**
 * Document template types (Epic 7B, Story 7B.2 — Document Templates & Generation).
 *
 * Mirrors backend `db::models::document_template` and the route request/response
 * shapes in `api-server/src/routes/templates.rs`. These endpoints are NOT
 * TypeSpec-generated, so this module is hand-written (mirrors the `esignature`
 * module pattern).
 */

/** Template type — matches backend `template_type::ALL`. */
export type TemplateType =
  | 'lease'
  | 'notice'
  | 'invoice'
  | 'report'
  | 'minutes'
  | 'contract'
  | 'custom';

/** Placeholder data type — matches backend `placeholder_type::ALL`. */
export type PlaceholderType = 'text' | 'date' | 'number' | 'currency';

/** A single `{{name}}` placeholder definition on a template. */
export interface TemplatePlaceholder {
  /** Placeholder name, used in `{{name}}` syntax inside the template content. */
  name: string;
  /** Data type: text, date, number, currency. Serialized as `type` on the wire. */
  type: PlaceholderType;
  /** Whether a value is required to generate. */
  required: boolean;
  /** Default value substituted when no value is provided. */
  default_value?: string | null;
  /** Human-readable description shown next to the input. */
  description?: string | null;
}

/** Full document template entity (`DocumentTemplate`). */
export interface DocumentTemplate {
  id: string;
  organization_id: string;
  name: string;
  description?: string | null;
  template_type: string;
  content: string;
  placeholders: TemplatePlaceholder[];
  usage_count: number;
  created_by: string;
  created_at: string;
  updated_at: string;
  deleted_at?: string | null;
}

/** Template summary row for list views (`DocumentTemplateSummary`). */
export interface DocumentTemplateSummary {
  id: string;
  name: string;
  description?: string | null;
  template_type: string;
  usage_count: number;
  placeholder_count: number;
  created_at: string;
}

/** Template with creator details (`TemplateWithDetails` — flattens the entity). */
export interface TemplateWithDetails extends DocumentTemplate {
  created_by_name: string;
}

/** GET /api/v1/templates response. */
export interface TemplateListResponse {
  templates: DocumentTemplateSummary[];
  count: number;
  total: number;
}

/** GET /api/v1/templates/{id} response. */
export interface TemplateDetailResponse {
  template: TemplateWithDetails;
}

/** Query parameters for listing templates. */
export interface ListDocumentTemplatesParams {
  template_type?: string;
  search?: string;
  limit?: number;
  offset?: number;
}

/** POST /api/v1/templates/{id}/generate request body. */
export interface GenerateDocumentRequest {
  /** Placeholder name → value map. */
  values: Record<string, string>;
  /** Title for the generated document. */
  title: string;
  /** Optional description for the generated document. */
  description?: string;
  /** Document category (see backend `document_category::ALL`). */
  category: string;
  /** Optional folder to place the generated document in. */
  folder_id?: string;
}

/** POST /api/v1/templates/{id}/generate response. */
export interface GenerateDocumentResponse {
  document_id: string;
  message: string;
}

/** POST /api/v1/templates request body. */
export interface CreateTemplateRequest {
  name: string;
  description?: string;
  template_type: string;
  content: string;
  placeholders: TemplatePlaceholder[];
}

/** POST /api/v1/templates response. */
export interface CreateTemplateResponse {
  id: string;
  message: string;
}

/** PUT /api/v1/templates/{id} request body. */
export interface UpdateTemplateRequest {
  name?: string;
  description?: string;
  template_type?: string;
  content?: string;
  placeholders?: TemplatePlaceholder[];
}

/** PUT /api/v1/templates/{id} response. */
export interface TemplateActionResponse {
  message: string;
  template: DocumentTemplate;
}
