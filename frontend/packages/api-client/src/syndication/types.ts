/**
 * Portal Syndication types (Epic 105 — Real Estate Portal Webhooks).
 *
 * Mirrors the backend DTOs in `backend/crates/db/src/models/listing.rs`. These
 * back the manager-facing syndication read endpoints under
 * `/api/v1/listings/syndication/*`, which surface the view/inquiry counts and
 * per-portal status that inbound portal webhooks feed into. The endpoints are
 * wired + org-scoped on the server but are not (yet) part of the generated
 * OpenAPI client, so this module is hand-written like `integrations/` and
 * `oauth-grants/`.
 */

/** Overall syndication health for a listing. */
export type SyndicationHealthStatus = 'healthy' | 'degraded' | 'unhealthy' | 'not_configured';

/** A single listing↔portal syndication record. */
export interface ListingSyndication {
  id: string;
  listing_id: string;
  portal: string;
  external_id: string | null;
  status: string;
  last_error: string | null;
  synced_at: string | null;
  created_at: string;
  updated_at: string;
}

/** Syndication status + activity for one portal on one listing. */
export interface PortalSyndicationStatus {
  portal: string;
  status: string;
  external_id: string | null;
  synced_at: string | null;
  last_error: string | null;
  /** Views reported by inbound portal webhooks. */
  views: number;
  /** Inquiries reported by inbound portal webhooks. */
  inquiries: number;
  last_activity_at: string | null;
}

/** Per-listing syndication dashboard row. */
export interface SyndicationStatusDashboard {
  listing_id: string;
  listing_title: string;
  listing_status: string;
  portal_statuses: PortalSyndicationStatus[];
  overall_status: SyndicationHealthStatus;
  total_views: number;
  total_inquiries: number;
}

/** Aggregated per-portal statistics across the organization. */
export interface PortalStats {
  portal: string;
  active_count: number;
  pending_count: number;
  failed_count: number;
  views: number;
  inquiries: number;
}

/** Organization-wide syndication statistics. */
export interface OrganizationSyndicationStats {
  total_active: number;
  total_pending: number;
  total_failed: number;
  total_views: number;
  total_inquiries: number;
  by_portal: PortalStats[];
}

/** Filter/paging parameters for the org syndication dashboard. */
export interface SyndicationDashboardQuery {
  portal?: string;
  status?: string;
  from_date?: string;
  to_date?: string;
  page?: number;
  limit?: number;
}

/** Paged org syndication dashboard response. */
export interface SyndicationDashboardResponse {
  listings: SyndicationStatusDashboard[];
  total: number;
  page: number;
  limit: number;
  organization_stats: OrganizationSyndicationStats;
}
