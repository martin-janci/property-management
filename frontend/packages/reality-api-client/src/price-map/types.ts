/**
 * Reality Portal Price Map Types
 *
 * TypeScript types for the city/district price-aggregation API (UC-31).
 *
 * Field names are snake_case to match the reality-server wire format
 * (`routes/price_map.rs` serialises with serde defaults — no
 * `rename_all`), so responses parse without a transform layer.
 */

/** One aggregated district/city price data point. */
export interface DistrictPriceData {
  /** City used as the district grouping key. */
  district_id: string;
  district_name: string;
  /** Average €/m² for the period; null when no priced listings matched. */
  avg_price_per_m2: number | null;
  listing_count: number;
  /** Quarter-on-quarter trend percentage (positive = rising); null when no prior period. */
  trend_pct_qoq: number | null;
}

/** Price map endpoint response. */
export interface PriceMapResponse {
  districts: DistrictPriceData[];
  mode: string;
  property_type: string | null;
  city: string | null;
  time_window: string;
  cached: boolean;
}

export type PriceMapMode = 'sale' | 'rent';
export type PriceMapPropertyType = 'all' | 'apartment' | 'house' | 'land';
/** Server-supported windows; default is 3m. */
export type PriceMapTimeWindow = '1m' | '3m' | '12m' | '5y';

/** Query parameters for {@link usePriceMap}. */
export interface PriceMapParams {
  mode?: PriceMapMode;
  /** 'all' is treated as "no filter" and omitted from the request. */
  propertyType?: PriceMapPropertyType;
  timeWindow?: PriceMapTimeWindow;
  city?: string;
}
