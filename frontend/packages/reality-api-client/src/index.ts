/**
 * Reality Portal API Client
 *
 * Generated from reality-server OpenAPI spec.
 * Used by: reality-web (Next.js), mobile-native (KMP via shared types)
 */

// Export generated types and client
// export * from './generated';

export * from './agency/hooks';
// Agency module - export with renamed ListingStatus to avoid conflict
export type {
  Agency,
  AgencyAddress,
  AgencyBranding,
  AgencyListing,
  AgencyMember,
  AgencyPerformance,
  AgencyStats,
  CreateAgencyRequest,
  InviteRealtorRequest,
  ListingStatus as AgencyListingStatus,
  Realtor,
  RealtorInvitation,
  RealtorStats,
  RealtorStatus,
  UpdateAgencyRequest,
  UpdateBrandingRequest,
  UpdateRealtorRequest,
} from './agency/types';
export * from './favorites';
// Import module - property import functionality (Epic 46)
export * from './import';
export * from './inquiries';
// Export domain-specific modules
export * from './listings';
// Price map module - city/district price aggregations (UC-31)
export * from './price-map';

// API version
export const REALITY_API_VERSION = '1.0.0';
