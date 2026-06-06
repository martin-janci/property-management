/**
 * API Ecosystem — Integration Marketplace API client (Epic 150, Story 150.1).
 *
 * Wraps `GET /api/v1/ecosystem/marketplace`. Uses the shared
 * `authenticatedFetchJson` helper so the bearer token + MFA-retry behaviour is
 * inherited (see lib/fetch.ts, #486).
 */

import { authenticatedFetchJson } from '../lib/fetch';
import type { MarketplaceIntegrationQuery, MarketplaceIntegrationSummary } from './types';

const API_BASE = '/api/v1/ecosystem';

function buildQueryString(params: MarketplaceIntegrationQuery): string {
  const searchParams = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined && value !== null && value !== '') {
      searchParams.append(key, String(value));
    }
  }
  const qs = searchParams.toString();
  return qs ? `?${qs}` : '';
}

/** List marketplace integrations. */
export async function listMarketplaceIntegrations(
  query: MarketplaceIntegrationQuery = {}
): Promise<MarketplaceIntegrationSummary[]> {
  return authenticatedFetchJson<MarketplaceIntegrationSummary[]>(
    `${API_BASE}/marketplace${buildQueryString(query)}`
  );
}
