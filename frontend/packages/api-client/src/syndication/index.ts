/**
 * Portal Syndication module (Epic 105 — Real Estate Portal Webhooks).
 *
 * Manager-facing read surface over listing syndication status/stats — the
 * downstream product of inbound portal webhooks (views, inquiries, per-portal
 * sync state).
 */

export {
  getListingSyndicationStatus,
  getListingSyndications,
  getOrganizationSyndicationStats,
  getSyndicationDashboard,
} from './api';
export {
  syndicationKeys,
  useListingSyndicationStatus,
  useListingSyndications,
  useOrganizationSyndicationStats,
  useSyndicationDashboard,
} from './hooks';
export type {
  ListingSyndication,
  OrganizationSyndicationStats,
  PortalStats,
  PortalSyndicationStatus,
  SyndicationDashboardQuery,
  SyndicationDashboardResponse,
  SyndicationHealthStatus,
  SyndicationStatusDashboard,
} from './types';
