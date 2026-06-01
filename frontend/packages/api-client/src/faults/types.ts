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

/** Fault status values */
export type FaultStatus =
  | 'reported'
  | 'triaged'
  | 'in_progress'
  | 'scheduled'
  | 'on_hold'
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

/** Full fault details */
export interface FaultWithDetails extends FaultSummary {
  location_description?: string;
  resolution_notes?: string;
  resolved_at?: string;
  resolved_by?: string;
  scheduled_date?: string;
  work_notes?: WorkNote[];
  comments?: FaultComment[];
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

/** Timeline entry for fault history */
export interface FaultTimelineEntry {
  id: string;
  fault_id: string;
  event_type: string;
  description: string;
  actor_id: string;
  actor_name?: string;
  created_at: string;
  metadata?: Record<string, unknown>;
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
