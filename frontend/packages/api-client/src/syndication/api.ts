/**
 * Portal Syndication API client (Epic 105 — Real Estate Portal Webhooks).
 *
 * Read surface over the org-scoped syndication endpoints. These endpoints are
 * authenticated (`TenantExtractor`) and mounted on the api-server but are not
 * emitted into the generated OpenAPI client, so — following the `integrations`
 * and `oauth-grants` modules — the calls are hand-written on top of the shared
 * `authenticatedFetchJson` helper (bearer token + MFA challenge handling).
 *
 *   GET /api/v1/listings/syndication/dashboard        — getSyndicationDashboard
 *   GET /api/v1/listings/syndication/stats            — getOrganizationSyndicationStats
 *   GET /api/v1/listings/{id}/syndications            — getListingSyndications
 *   GET /api/v1/listings/{id}/syndication/status      — getListingSyndicationStatus
 */

import { authenticatedFetchJson } from '../lib/fetch';
import type {
  ListingSyndication,
  OrganizationSyndicationStats,
  SyndicationDashboardQuery,
  SyndicationDashboardResponse,
  SyndicationStatusDashboard,
} from './types';

const LISTINGS_BASE = '/api/v1/listings';

function buildQueryString(params: SyndicationDashboardQuery): string {
  const search = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined && value !== null && value !== '') {
      search.append(key, String(value));
    }
  }
  const qs = search.toString();
  return qs ? `?${qs}` : '';
}

/** Organization-wide syndication dashboard (per-listing rows + aggregate stats). */
export async function getSyndicationDashboard(
  query: SyndicationDashboardQuery = {}
): Promise<SyndicationDashboardResponse> {
  return authenticatedFetchJson<SyndicationDashboardResponse>(
    `${LISTINGS_BASE}/syndication/dashboard${buildQueryString(query)}`
  );
}

/** Organization-wide aggregated syndication statistics. */
export async function getOrganizationSyndicationStats(): Promise<OrganizationSyndicationStats> {
  return authenticatedFetchJson<OrganizationSyndicationStats>(`${LISTINGS_BASE}/syndication/stats`);
}

/** Raw syndication records for a single listing. */
export async function getListingSyndications(listingId: string): Promise<ListingSyndication[]> {
  return authenticatedFetchJson<ListingSyndication[]>(
    `${LISTINGS_BASE}/${encodeURIComponent(listingId)}/syndications`
  );
}

/** Per-portal syndication status dashboard for a single listing. */
export async function getListingSyndicationStatus(
  listingId: string
): Promise<SyndicationStatusDashboard> {
  return authenticatedFetchJson<SyndicationStatusDashboard>(
    `${LISTINGS_BASE}/${encodeURIComponent(listingId)}/syndication/status`
  );
}
