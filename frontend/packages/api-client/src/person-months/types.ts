/**
 * Person-months API types (Epic 3, Story 3.5).
 *
 * Wire shapes for `/api/v1/buildings/{building_id}/.../person-months`. These
 * mirror the backend `routes/person_months.rs` response/request structs and are
 * therefore snake_case. Domain-prefixed names (`PersonMonthMonthlyCount`,
 * `PersonMonthBuildingSummary`, …) avoid collisions with the `reports` and
 * `buildings` modules that already export `MonthlyCount` / `BuildingSummary`.
 */

/** Source of a person-month entry. */
export type PersonMonthSource = 'manual' | 'calculated' | 'imported';

/** A single person-month entry for a unit (unit-level response). */
export interface PersonMonth {
  id: string;
  unit_id: string;
  year: number;
  month: number;
  count: number;
  source: string;
  source_display: string;
  period: string;
  notes?: string | null;
  created_at: string;
  updated_at: string;
}

/** Person-month entry enriched with its unit designation (building-level list). */
export interface PersonMonthWithUnit {
  id: string;
  unit_id: string;
  unit_designation: string;
  year: number;
  month: number;
  count: number;
  source: string;
}

/** Count for one month within a yearly summary. */
export interface PersonMonthMonthlyCount {
  month: number;
  count: number;
  source: string;
}

/** Yearly summary of person-months for a single unit. */
export interface YearlyPersonMonthSummary {
  unit_id: string;
  year: number;
  months: PersonMonthMonthlyCount[];
  total: number;
}

/** Building-level aggregate summary for a month. */
export interface PersonMonthBuildingSummary {
  building_id: string;
  year: number;
  month: number;
  total_count: number;
  unit_count: number;
}

/** Result for a single entry in a bulk upsert. */
export interface BulkPersonMonthUpsertEntryResult {
  unit_id: string;
  success: boolean;
  person_month_id?: string | null;
  error?: string | null;
}

/** Outcome of a bulk upsert. */
export interface BulkPersonMonthUpsertResult {
  successful: number;
  failed: number;
  results: BulkPersonMonthUpsertEntryResult[];
}

// ============================================================================
// Requests / queries
// ============================================================================

/** Query for unit person-months: a year, optionally narrowed to one month. */
export interface GetUnitPersonMonthsQuery {
  year: number;
  month?: number;
}

/** Create-or-update a unit's person-month entry. */
export interface UpsertPersonMonthRequest {
  year: number;
  month: number;
  count: number;
  /** Defaults to `manual` server-side when omitted. */
  source?: PersonMonthSource;
  notes?: string | null;
}

/** Partial update of an existing person-month entry. */
export interface UpdatePersonMonthRequest {
  count?: number;
  source?: PersonMonthSource;
  notes?: string | null;
}

/** A single unit's count within a bulk upsert request. */
export interface BulkPersonMonthEntry {
  unit_id: string;
  count: number;
}

/** Bulk upsert of person-months for every unit in a building for one period. */
export interface BulkUpsertPersonMonthsRequest {
  year: number;
  month: number;
  entries: BulkPersonMonthEntry[];
}

/** Query for building-level person-month operations (a specific period). */
export interface BuildingPersonMonthsQuery {
  year: number;
  month: number;
}

/** Request to derive a person-month count from a unit's resident history. */
export interface CalculatePersonMonthRequest {
  year: number;
  month?: number;
}
