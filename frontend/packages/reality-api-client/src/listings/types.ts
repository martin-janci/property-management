/**
 * Reality Portal Listing Types
 *
 * TypeScript types for property listings API (Epic 44).
 */

// Property Types
export type PropertyType = 'apartment' | 'house' | 'land' | 'commercial' | 'office' | 'garage';

export type TransactionType = 'sale' | 'rent';

export type ListingStatus = 'active' | 'pending' | 'sold' | 'rented' | 'withdrawn';

// Address
export interface ListingAddress {
  street?: string;
  city: string;
  district?: string;
  postalCode?: string;
  country: string;
  latitude?: number;
  longitude?: number;
}

// Features
export interface ListingFeatures {
  balcony?: boolean;
  terrace?: boolean;
  garden?: boolean;
  parking?: boolean;
  garage?: boolean;
  elevator?: boolean;
  cellar?: boolean;
  airConditioning?: boolean;
  furnished?: boolean;
  petFriendly?: boolean;
  disabledAccess?: boolean;
}

// Photo
export interface ListingPhoto {
  id: string;
  url: string;
  thumbnailUrl: string;
  caption?: string;
  isPrimary: boolean;
  order: number;
}

// Agent
export interface ListingAgent {
  id: string;
  name: string;
  email: string;
  phone?: string;
  avatarUrl?: string;
  agencyName?: string;
  agencyLogo?: string;
}

// Listing Summary (for cards/lists)
export interface ListingSummary {
  id: string;
  title: string;
  /**
   * SEO-friendly URL slug, `{slugified-title}-{uuid}` (#3616). Always populated
   * by reality-server. The trailing UUID is the authoritative detail-lookup
   * identifier — use this value to build the listing detail route / fetch the
   * detail (a bare UUID is also accepted for backward-compatible deep links).
   */
  slug: string;
  propertyType: PropertyType;
  transactionType: TransactionType;
  status: ListingStatus;
  price: number;
  currency: string;
  pricePerSqm?: number;
  area: number;
  rooms?: number;
  bedrooms?: number;
  bathrooms?: number;
  floor?: number;
  totalFloors?: number;
  address: ListingAddress;
  primaryPhoto?: ListingPhoto;
  isFeatured: boolean;
  isFavorite?: boolean;
  createdAt: string;
  updatedAt: string;
}

// Full Listing Detail
export interface ListingDetail extends ListingSummary {
  description: string;
  features: ListingFeatures;
  photos: ListingPhoto[];
  agent: ListingAgent;
  yearBuilt?: number;
  energyRating?: string;
  monthlyCharges?: number;
  availableFrom?: string;
  virtualTourUrl?: string;
  floorPlanUrl?: string;
  viewCount: number;
  favoriteCount: number;
}

// Search/Filter Types
export interface ListingFilters {
  query?: string;
  propertyType?: PropertyType[];
  transactionType?: TransactionType;
  priceMin?: number;
  priceMax?: number;
  areaMin?: number;
  areaMax?: number;
  roomsMin?: number;
  roomsMax?: number;
  bedroomsMin?: number;
  city?: string;
  district?: string;
  features?: (keyof ListingFeatures)[];
  sortBy?: ListingSortField;
  sortOrder?: 'asc' | 'desc';
}

export type ListingSortField = 'price' | 'area' | 'createdAt' | 'updatedAt' | 'rooms';

// Paginated Response
export interface PaginatedListings {
  data: ListingSummary[];
  total: number;
  page: number;
  pageSize: number;
  totalPages: number;
}

// Featured Listings Response
export interface FeaturedListingsResponse {
  sale: ListingSummary[];
  rent: ListingSummary[];
  new: ListingSummary[];
}

// Category counts for homepage
export interface CategoryCount {
  type: PropertyType;
  count: number;
  label: string;
  icon: string;
}

// Search Suggestions
export interface SearchSuggestion {
  type: 'city' | 'district' | 'address' | 'listing';
  value: string;
  label: string;
  count?: number;
}
