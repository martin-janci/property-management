/**
 * Types for the meters API client (Epic 12: self-readings & consumption).
 *
 * These mirror the backend wire shapes (`db::models::*`, serde snake_case).
 * Decimal columns are serialized as JSON strings (rust_decimal `serde-str`),
 * so reading/consumption values arrive as `string` and are converted to
 * `number` in the ppt-web route-group mappers, not here.
 */

export type MeterType =
  | 'electricity'
  | 'gas'
  | 'water'
  | 'heat'
  | 'cold_water'
  | 'hot_water'
  | 'solar'
  | 'other';

export type ReadingSource = 'manual' | 'photo' | 'automatic' | 'estimated';

export type ReadingStatus = 'pending' | 'approved' | 'rejected' | 'estimated';

/** A meter (`db::models::Meter`). Decimal fields arrive as strings. */
export interface Meter {
  id: string;
  organization_id: string;
  building_id: string;
  unit_id?: string | null;
  meter_number: string;
  meter_type: MeterType;
  location?: string | null;
  description?: string | null;
  initial_reading: string;
  current_reading?: string | null;
  last_reading_date?: string | null;
  unit_of_measure: string;
  is_smart_meter: boolean;
  smart_meter_provider?: string | null;
  is_active: boolean;
  is_shared: boolean;
  installed_at?: string | null;
  decommissioned_at?: string | null;
  replaced_meter_id?: string | null;
  replacement_reading?: string | null;
  created_at: string;
  updated_at: string;
}

/** A meter reading (`db::models::MeterReading`). Decimal fields arrive as strings. */
export interface MeterReading {
  id: string;
  meter_id: string;
  reading: string;
  reading_date: string;
  reading_time?: string | null;
  source: ReadingSource;
  photo_url?: string | null;
  ocr_reading?: string | null;
  status: ReadingStatus;
  validated_by?: string | null;
  validated_at?: string | null;
  validation_notes?: string | null;
  consumption?: string | null;
  previous_reading_id?: string | null;
  submission_window_id?: string | null;
  is_anomaly: boolean;
  anomaly_reason?: string | null;
  submitted_by?: string | null;
  created_at: string;
  updated_at: string;
}

/** `GET /api/v1/meters/{id}` (`db::models::MeterResponse`). */
export interface MeterResponse {
  meter: Meter;
  recent_readings: MeterReading[];
}

/** `GET /api/v1/meters/buildings/{building_id}` (`db::models::ListMetersResponse`). */
export interface ListMetersResponse {
  meters: Meter[];
  total: number;
}

/** `GET /api/v1/meters/{meter_id}/readings` (`db::models::ListMeterReadingsResponse`). */
export interface ListMeterReadingsResponse {
  readings: MeterReading[];
  total: number;
}

export interface ConsumptionDataPoint {
  date: string;
  reading: string;
  consumption: string;
}

export interface ConsumptionComparison {
  period: string;
  current_consumption: string;
  previous_consumption: string;
  change_percentage: string;
  vs_building_average: string;
}

/** `GET /api/v1/meters/{meter_id}/consumption` (`db::models::ConsumptionHistoryResponse`). */
export interface ConsumptionHistoryResponse {
  meter_id: string;
  meter_type: MeterType;
  unit_of_measure: string;
  history: ConsumptionDataPoint[];
  building_average?: string | null;
  comparison?: ConsumptionComparison | null;
}

// ============================================================================
// Request payloads / queries
// ============================================================================

export interface ListMetersQuery {
  limit?: number;
  offset?: number;
}

export interface ListReadingsQuery {
  from?: string;
  to?: string;
  limit?: number;
  offset?: number;
}

export interface PendingReadingsQuery {
  organization_id: string;
  building_id?: string;
}

export interface ConsumptionQuery {
  from: string;
  to: string;
}

/** Body for `POST /api/v1/meters/readings` (`db::models::SubmitReading`). */
export interface SubmitReadingRequest {
  meter_id: string;
  reading: number | string;
  reading_date?: string;
  reading_time?: string;
  source: ReadingSource;
  photo_url?: string;
}

/** Body for `PUT /api/v1/meters/readings/{id}/validate` (`db::models::ValidateReading`). */
export interface ValidateReadingRequest {
  status: ReadingStatus;
  corrected_reading?: number | string;
  notes?: string;
}
