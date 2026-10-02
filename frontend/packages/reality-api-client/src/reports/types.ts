/**
 * Reality Portal Listing-Report Types
 *
 * TypeScript types for the abuse / problem-report API (UC-23: Report a
 * Listing). Field names are the exact snake_case wire format expected by
 * reality-server's `SubmitReportRequest` — the Rust struct has no
 * `#[serde(rename_all)]`, so the JSON body must use snake_case keys verbatim.
 */

// Problem categories — mirror reality-server's `ProblemType` enum, which is
// serialised `#[serde(rename_all = "snake_case")]`.
export type ReportProblemType =
  | 'incorrect_information'
  | 'fraudulent_listing'
  | 'already_sold'
  | 'price_manipulation'
  | 'inappropriate_content'
  | 'duplicate_listing'
  | 'other';

// Submit-report request body (POST /api/v1/reports).
export interface SubmitReportRequest {
  listing_id: string;
  problem_type: ReportProblemType;
  description: string;
  // Attachment URLs (http/https). The reality-server caps these at 10 and
  // rejects non-http(s) schemes; omitted when there is nothing to send.
  attachments?: string[];
  reporter_email?: string;
  reporter_phone?: string;
}

// A persisted listing report, as returned by the server.
export interface ListingReport {
  id: string;
  listing_id: string;
  reporter_user_id?: string | null;
  problem_type: string;
  description: string;
  attachments?: string[] | null;
  reporter_email?: string | null;
  reporter_phone?: string | null;
  status: string;
  resolution_notes?: string | null;
  created_at: string;
  updated_at: string;
}

// Submit-report response envelope (201 Created).
export interface SubmitReportResponse {
  report: ListingReport;
}
