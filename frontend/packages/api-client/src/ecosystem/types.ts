/**
 * API Ecosystem — Integration Marketplace types (Epic 150, Story 150.1).
 *
 * Mirrors the backend `MarketplaceIntegrationSummary` / `MarketplaceIntegrationQuery`
 * structs in `backend/crates/db/src/models/api_ecosystem.rs`.
 *
 * NOTE: those structs carry no `#[serde(rename_all)]`, so the wire format is
 * Rust-default snake_case. The field names below match the JSON exactly.
 */

export type IntegrationStatus = 'available' | 'coming_soon' | 'deprecated' | 'maintenance';

/** Integration summary as returned by `GET /api/v1/ecosystem/marketplace`. */
export interface MarketplaceIntegrationSummary {
  id: string;
  slug: string;
  name: string;
  description: string;
  category: string;
  icon_url?: string | null;
  vendor_name: string;
  status: IntegrationStatus;
  rating_average?: number | null;
  rating_count: number;
  install_count: number;
  is_featured: boolean;
  is_premium: boolean;
}

/** Query parameters for `GET /api/v1/ecosystem/marketplace` (snake_case on the wire). */
export interface MarketplaceIntegrationQuery {
  category?: string;
  status?: string;
  search?: string;
  featured_only?: boolean;
  premium_only?: boolean;
  sort_by?: string;
  sort_order?: string;
  limit?: number;
  offset?: number;
}
