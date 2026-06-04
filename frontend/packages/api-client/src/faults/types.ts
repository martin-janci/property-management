/**
 * Fault API types (Epic 4: Fault Reporting & Resolution).
 */

/** Fault categories matching backend enum */
export type FaultCategory =
  | 'plumbing'
  | 'electrical'
  | 'heating'
  | 'structural'
  | 'exterior'
  | 'elevator'
  | 'common_area'
  | 'security'
  | 'cleaning'
  | 'other';

/** Fault priority levels */
export type FaultPriority = 'low' | 'medium' | 'high' | 'urgent';

/**
 * Fault status values.
 *
 * Mirrors the backend `fault_status` enum (`db/src/models/fault.rs`). The
 * backend emits `new` (not `reported`) and `waiting_parts` (not `on_hold`).
 */
export type FaultStatus =
  | 'new'
  | 'triaged'
  | 'in_progress'
  | 'waiting_parts'
  | 'scheduled'
  | 'resolved'
  | 'closed'
  | 'reopened';

/** Fault summary for list views */
export interface FaultSummary {
  id: string;
  title: string;
  description: string;
  category: FaultCategory;
  priority?: FaultPriority;
  status: FaultStatus;
  building_id: string;
  building_name?: string;
  unit_id?: string;
  unit_designation?: string;
  reporter_id: string;
  reporter_name?: string;
  assigned_to?: string;
  assigned_to_name?: string;
  created_at: string;
  updated_at: string;
  ai_suggested_category?: FaultCategory;
  ai_confidence?: number;
}

/**
 * Full fault details returned by `GET /api/v1/faults/{id}` (the `fault` field
 * of {@link FaultDetailResponse}).
 *
 * Mirrors the backend `FaultWithDetails` model (`db/src/models/fault.rs`):
 * the inner `Fault` is `#[serde(flatten)]`-ed (snake_case, no `rename_all`),
 * then joined display fields are appended. Comments and the timeline are NOT
 * embedded here — the timeline arrives in {@link FaultDetailResponse.timeline}.
 */
export interface FaultWithDetails {
  // --- Flattened Fault fields ---
  id: string;
  organization_id: string;
  building_id: string;
  unit_id?: string | null;
  reporter_id: string;
  title: string;
  description: string;
  location_description?: string | null;
  category: FaultCategory;
  priority: FaultPriority;
  status: FaultStatus;
  ai_category?: FaultCategory | null;
  ai_priority?: FaultPriority | null;
  /** AI confidence (backend `rust_decimal::Decimal` serialized as a number/string). */
  ai_confidence?: number | string | null;
  ai_processed_at?: string | null;
  assigned_to?: string | null;
  assigned_at?: string | null;
  triaged_by?: string | null;
  triaged_at?: string | null;
  resolved_at?: string | null;
  resolved_by?: string | null;
  resolution_notes?: string | null;
  confirmed_at?: string | null;
  confirmed_by?: string | null;
  rating?: number | null;
  feedback?: string | null;
  scheduled_date?: string | null;
  estimated_completion?: string | null;
  idempotency_key?: string | null;
  created_at: string;
  updated_at: string;
  // --- Joined display fields ---
  reporter_name: string;
  reporter_email: string;
  building_name: string;
  building_address: string;
  unit_designation?: string | null;
  assigned_to_name?: string | null;
  attachment_count: number;
  comment_count: number;
}

/** Work note on a fault */
export interface WorkNote {
  id: string;
  fault_id: string;
  author_id: string;
  author_name?: string;
  content: string;
  created_at: string;
}

/** Comment on a fault */
export interface FaultComment {
  id: string;
  fault_id: string;
  author_id: string;
  author_name?: string;
  content: string;
  is_internal: boolean;
  created_at: string;
}

/** Fault attachment (photo, document) */
export interface FaultAttachment {
  id: string;
  fault_id: string;
  file_url: string;
  file_name: string;
  file_type: string;
  file_size: number;
  uploaded_by: string;
  uploaded_by_name?: string;
  created_at: string;
}

/**
 * Timeline entry for fault history, returned in
 * {@link FaultDetailResponse.timeline} by `GET /api/v1/faults/{id}`.
 *
 * Mirrors the backend `FaultTimelineEntryWithUser` model
 * (`db/src/models/fault.rs`): the inner `FaultTimelineEntry` is
 * `#[serde(flatten)]`-ed (snake_case), then `user_name`/`user_email` are
 * appended from the joined user row.
 */
export interface FaultTimelineEntry {
  id: string;
  fault_id: string;
  user_id: string;
  /** Machine action key, e.g. `created`, `status_changed`, `assigned`, `comment`. */
  action: string;
  /** Free-text note / comment body (null for value-only transitions). */
  note?: string | null;
  /** Previous value for transitions (e.g. old status). */
  old_value?: string | null;
  /** New value for transitions (e.g. new status). */
  new_value?: string | null;
  /** Arbitrary structured metadata attached to the event. */
  metadata?: Record<string, unknown>;
  /** Whether this entry is manager-only (internal note). */
  is_internal: boolean;
  created_at: string;
  /** Display name of the acting user (joined). */
  user_name: string;
  /** Email of the acting user (joined). */
  user_email: string;
}

/** AI suggestion response */
export interface AiSuggestion {
  category: FaultCategory;
  confidence: number;
  priority?: FaultPriority;
}

/**
 * Per-status fault count, shared between the manager dashboard
 * (`GET /api/v1/faults/statistics`) and the admin support-data
 * endpoint (`GET /api/v1/platform-admin/support-data`).
 *
 * Mirrors the Rust `StatusCount` model in `db/src/models/fault.rs`.
 */
export interface FaultStatusCount {
  status: string;
  count: number;
}

/**
 * Per-category fault count.
 * Mirrors the Rust `CategoryCount` model in `db/src/models/fault.rs`.
 */
export interface FaultCategoryCount {
  category: string;
  count: number;
}

/**
 * Per-priority fault count.
 * Mirrors the Rust `PriorityCount` model in `db/src/models/fault.rs`.
 */
export interface FaultPriorityCount {
  priority: string;
  count: number;
}

/**
 * Fault statistics returned by `GET /api/v1/faults/statistics`.
 *
 * Field names and shapes match the backend `FaultStatistics` struct
 * (`db/src/models/fault.rs`).  The `by_status`, `by_category`, and
 * `by_priority` fields are **arrays of count objects** (not Record maps)
 * so that zero-count statuses are absent rather than present with a 0.
 *
 * Both the ppt-web manager dashboard and the admin-web support-data page
 * derive their fault-status KPIs from these same buckets — changing the
 * counting window here affects both surfaces equally.
 */
export interface FaultStatistics {
  /** Total fault count across the queried scope. */
  total_count: number;
  /** Faults with any status other than `closed`. */
  open_count: number;
  /** Faults with status `closed`. */
  closed_count: number;
  /** Fault counts grouped by status, ordered by count descending. */
  by_status: FaultStatusCount[];
  /** Fault counts grouped by category, ordered by count descending. */
  by_category: FaultCategoryCount[];
  /** Fault counts grouped by priority, ordered by count descending. */
  by_priority: FaultPriorityCount[];
  /** Mean time from `created_at` to `resolved_at` in hours (null when no resolved faults). */
  average_resolution_time_hours: number | null;
  /** Mean resident rating (1–5) for resolved faults (null when no rated faults). */
  average_rating: number | null;
}

/** Request to create a fault */
export interface CreateFaultRequest {
  building_id: string;
  unit_id?: string;
  title: string;
  description: string;
  location_description?: string;
  category: FaultCategory;
  priority?: FaultPriority;
  /** Photos to attach (base64 encoded or URLs) */
  photos?: string[];
  /** Idempotency key for duplicate prevention */
  idempotency_key?: string;
}

/** Request to update a fault */
export interface UpdateFaultRequest {
  title?: string;
  description?: string;
  location_description?: string;
  category?: FaultCategory;
}

/** Request to triage a fault */
export interface TriageFaultRequest {
  priority: FaultPriority;
  category?: FaultCategory;
  assigned_to?: string;
}

/** Request to resolve a fault */
export interface ResolveFaultRequest {
  resolution_notes: string;
}

/** Request to add a comment */
export interface AddCommentRequest {
  content: string;
  is_internal?: boolean;
}

/** Request to add a work note */
export interface AddWorkNoteRequest {
  content: string;
}

/** Query parameters for listing faults */
export interface FaultListQuery {
  building_id?: string;
  status?: FaultStatus;
  category?: FaultCategory;
  priority?: FaultPriority;
  assigned_to?: string;
  search?: string;
  page?: number;
  limit?: number;
}

/** Fault list response */
export interface FaultListResponse {
  faults: FaultSummary[];
  count: number;
}

/** Fault detail response */
export interface FaultDetailResponse {
  fault: FaultWithDetails;
  timeline: FaultTimelineEntry[];
  attachments: FaultAttachment[];
}

/** Response for creating a fault */
export interface CreateFaultResponse {
  id: string;
  message: string;
}

/** Response for AI suggestion */
export interface AiSuggestionResponse {
  suggestion: AiSuggestion;
}
