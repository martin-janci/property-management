/**
 * API Ecosystem — Integration Marketplace React Query hooks (Epic 150, Story 150.1).
 */

import { useQuery } from '@tanstack/react-query';
import * as api from './api';
import type { MarketplaceIntegrationQuery } from './types';

export const ecosystemKeys = {
  all: ['ecosystem'] as const,
  marketplace: (query: MarketplaceIntegrationQuery) =>
    [...ecosystemKeys.all, 'marketplace', query] as const,
};

/** List marketplace integrations from `GET /api/v1/ecosystem/marketplace`. */
export function useMarketplaceIntegrations(query: MarketplaceIntegrationQuery = {}) {
  return useQuery({
    queryKey: ecosystemKeys.marketplace(query),
    queryFn: () => api.listMarketplaceIntegrations(query),
  });
}
