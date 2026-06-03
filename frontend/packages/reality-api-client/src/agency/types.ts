/**
 * Agency Types
 *
 * TypeScript types for agency management in Reality Portal (Epic 45).
 */

import type { ListingStatus as BaseListingStatus } from '../listings/types';

// Extended status that includes 'draft' for agency management
export type ListingStatus = BaseListingStatus | 'draft';

export interface Agency {
  id: string;
  name: string;
  slug: string;
  description?: string;
  logoUrl?: string;
  primaryColor?: string;
  secondaryColor?: string;
  website?: string;
  email: string;
  phone?: string;
  address?: AgencyAddress;
  licenseNumber?: string;
  verifiedAt?: string;
  createdAt: string;
  updatedAt: string;
}

export interface AgencyAddress {
  street?: string;
  city: string;
  district?: string;
  postalCode?: string;
  country: string;
}

export interface AgencyStats {
  totalListings: number;
  activeListings: number;
  soldListings: number;
  rentedListings: number;
  totalViews: number;
  totalInquiries: number;
  totalRealtors: number;
  conversionRate: number;
  averageDaysOnMarket: number;
  averageListingPrice: number;
}

export interface AgencyPerformance {
  period: string;
  listings: number;
  views: number;
  inquiries: number;
  conversions: number;
}

export interface Realtor {
  id: string;
  userId: string;
  agencyId: string;
  name: string;
  email: string;
  phone?: string;
  photoUrl?: string;
  title?: string;
  bio?: string;
  licenseNumber?: string;
  specializations: string[];
  activeListings: number;
  totalSales: number;
  rating?: number;
  reviewCount: number;
  status: RealtorStatus;
  invitedAt?: string;
  joinedAt?: string;
  createdAt: string;
  updatedAt: string;
}

export type RealtorStatus = 'invited' | 'active' | 'inactive' | 'suspended';

/**
 * Agency team member as returned by `GET /api/v1/agencies/{id}/members`
 * (reality-server `RealityAgencyMember`). Membership-level info only — richer
 * realtor profiles come from the `useRealtors` endpoint.
 */
export interface AgencyMember {
  id: string;
  agencyId: string;
  userId: string;
  role: string;
  isActive: boolean;
  joinedAt: string;
  leftAt?: string;
}

export interface RealtorStats {
  totalListings: number;
  activeListings: number;
  closedDeals: number;
  totalViews: number;
  totalInquiries: number;
  conversionRate: number;
  averageDaysToClose: number;
  totalRevenue: number;
}

export interface RealtorInvitation {
  email: string;
  name: string;
  title?: string;
  message?: string;
}

export interface AgencyBranding {
  logoUrl?: string;
  primaryColor: string;
  secondaryColor: string;
  accentColor?: string;
  fontFamily?: string;
  coverImageUrl?: string;
}

export interface AgencyListing {
  id: string;
  title: string;
  slug: string;
  propertyType: string;
  transactionType: 'sale' | 'rent';
  price: number;
  currency: string;
  status: ListingStatus;
  realtorId: string;
  realtorName: string;
  views: number;
  inquiries: number;
  primaryPhotoUrl?: string;
  createdAt: string;
  updatedAt: string;
}

export interface CreateAgencyRequest {
  name: string;
  description?: string;
  email: string;
  phone?: string;
  website?: string;
  address?: AgencyAddress;
  licenseNumber?: string;
}

export interface UpdateAgencyRequest {
  name?: string;
  description?: string;
  email?: string;
  phone?: string;
  website?: string;
  address?: AgencyAddress;
  licenseNumber?: string;
}

export interface UpdateBrandingRequest {
  logo?: File;
  primaryColor?: string;
  secondaryColor?: string;
  accentColor?: string;
  fontFamily?: string;
  coverImage?: File;
}

export interface InviteRealtorRequest {
  email: string;
  name: string;
  title?: string;
  message?: string;
}

export interface UpdateRealtorRequest {
  title?: string;
  bio?: string;
  phone?: string;
  specializations?: string[];
  status?: RealtorStatus;
}
