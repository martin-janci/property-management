/**
 * Reality Portal Price Map types.
 *
 * Mirror the reality-server `GET /api/v1/price-map` response
 * (routes/price_map.rs). Fields are snake_case because that handler
 * serialises with serde's default naming (no `rename_all`).
 */

export type PriceMapMode = 'sale' | 'rent';
export type PriceMapTimeWindow = '1m' | '3m' | '12m' | '5y';

/** A single city/district price aggregation. */
export interface DistrictPriceData {
  /** City used as the district grouping key. */
  district_id: string;
  district_name: string;
  /** Average €/m² for the period, or null when no priced listings. */
  avg_price_per_m2: number | null;
  listing_count: number;
  /** Period-on-period trend percentage (positive = rising). */
  trend_pct_qoq: number | null;
}

/** Response body of `GET /api/v1/price-map`. */
export interface PriceMapResponse {
  districts: DistrictPriceData[];
  mode: string;
  property_type: string | null;
  city: string | null;
  time_window: string;
  cached: boolean;
}

/** Client-side filter inputs for the price map query. */
export interface PriceMapFilters {
  mode?: PriceMapMode;
  /** apartment | house | land | commercial — omit for all property types. */
  propertyType?: string;
  city?: string;
  timeWindow?: PriceMapTimeWindow;
}
