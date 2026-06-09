/**
 * Types for the leases API client (Epic 19: Lease Management & Tenant
 * Screening, plus Epic 142 violation tracking mounted at `/api/v1/violations`).
 *
 * These mirror the backend wire shapes (`db::models::lease::*` /
 * `db::models::violations::*`, serde snake_case). Decimal columns are
 * serialized as JSON strings (rust_decimal `serde-str`), so monetary fields
 * arrive as `string` and are converted to `number` in the ppt-web route-group
 * mappers, not here. Dates (`NaiveDate` / `DateTime<Utc>`) also arrive as
 * strings.
 */

// ============================================================================
// Status string unions (backend stores these as free-form `String` columns)
// ============================================================================

/** `db::models::lease::lease_status::*`. */
export type LeaseStatus =
  | 'draft'
  | 'pending_signature'
  | 'active'
  | 'renewed'
  | 'terminated'
  | 'expired'
  | 'cancelled';

/** `db::models::lease::application_status::*`. */
export type ApplicationStatus =
  | 'draft'
  | 'submitted'
  | 'under_review'
  | 'screening_pending'
  | 'screening_complete'
  | 'approved'
  | 'rejected'
  | 'withdrawn';

/** `db::models::lease::screening_status::*`. */
export type ScreeningStatus =
  | 'pending_consent'
  | 'consent_received'
  | 'in_progress'
  | 'completed'
  | 'failed'
  | 'expired';

// ============================================================================
// Core entities
// ============================================================================

/** A lease (`db::models::lease::Lease`). Decimal/date fields arrive as strings. */
export interface Lease {
  id: string;
  organization_id: string;
  unit_id: string;
  application_id?: string | null;
  template_id?: string | null;

  landlord_user_id?: string | null;
  landlord_name: string;
  landlord_address?: string | null;

  tenant_user_id?: string | null;
  tenant_name: string;
  tenant_email: string;
  tenant_phone?: string | null;

  occupants?: unknown;

  start_date: string;
  end_date: string;
  term_months: number;
  is_fixed_term: boolean;

  monthly_rent: string;
  security_deposit: string;
  deposit_held_by?: string | null;
  rent_due_day: number;
  late_fee_amount?: string | null;
  late_fee_grace_days?: number | null;

  utilities_included?: unknown;
  parking_spaces: number;
  storage_units: number;

  pets_allowed: boolean;
  pet_deposit?: string | null;
  max_occupants?: number | null;
  smoking_allowed: boolean;

  document_url?: string | null;
  document_version: number;

  status: string;

  landlord_signed_at?: string | null;
  tenant_signed_at?: string | null;
  signature_request_id?: string | null;

  terminated_at?: string | null;
  termination_reason?: string | null;
  termination_notes?: string | null;
  termination_initiated_by?: string | null;

  previous_lease_id?: string | null;
  renewed_to_lease_id?: string | null;
  renewal_offered_at?: string | null;
  renewal_offer_expires_at?: string | null;

  notes?: string | null;
  created_by?: string | null;
  created_at: string;
  updated_at: string;
}

/** A lease amendment (`db::models::lease::LeaseAmendment`). */
export interface LeaseAmendment {
  id: string;
  lease_id: string;
  amendment_number: number;
  title: string;
  description?: string | null;
  changes: unknown;
  effective_date: string;
  document_url?: string | null;
  landlord_signed_at?: string | null;
  tenant_signed_at?: string | null;
  created_by?: string | null;
  created_at: string;
}

/** A lease payment (`db::models::lease::LeasePayment`). Decimal fields are strings. */
export interface LeasePayment {
  id: string;
  lease_id: string;
  organization_id: string;
  due_date: string;
  amount: string;
  payment_type: string;
  description?: string | null;
  paid_at?: string | null;
  paid_amount?: string | null;
  payment_method?: string | null;
  payment_reference?: string | null;
  is_late: boolean;
  late_fee_applied?: string | null;
  invoice_id?: string | null;
  created_at: string;
  updated_at: string;
}

/** A lease reminder (`db::models::lease::LeaseReminder`). */
export interface LeaseReminder {
  id: string;
  lease_id: string;
  reminder_type: string;
  trigger_date: string;
  days_before_event?: number | null;
  subject: string;
  message: string;
  recipients: unknown;
  sent_at?: string | null;
  acknowledged_at?: string | null;
  acknowledged_by?: string | null;
  is_recurring: boolean;
  recurrence_pattern?: string | null;
  created_at: string;
}

/** A tenant application (`db::models::lease::TenantApplication`). */
export interface TenantApplication {
  id: string;
  organization_id: string;
  unit_id: string;
  applicant_name: string;
  applicant_email: string;
  applicant_phone?: string | null;
  date_of_birth?: string | null;
  national_id?: string | null;
  current_address?: string | null;
  current_landlord_name?: string | null;
  current_landlord_phone?: string | null;
  current_rent_amount?: string | null;
  current_tenancy_start?: string | null;
  employer_name?: string | null;
  employer_phone?: string | null;
  job_title?: string | null;
  employment_start?: string | null;
  monthly_income?: string | null;
  desired_move_in?: string | null;
  desired_lease_term_months?: number | null;
  proposed_rent?: string | null;
  co_applicants?: unknown;
  documents?: unknown;
  status: string;
  submitted_at?: string | null;
  reviewed_by?: string | null;
  reviewed_at?: string | null;
  decision_notes?: string | null;
  source?: string | null;
  referral_code?: string | null;
  created_at: string;
  updated_at: string;
}

/** A tenant screening (`db::models::lease::TenantScreening`). */
export interface TenantScreening {
  id: string;
  application_id: string;
  organization_id: string;
  screening_type: string;
  provider?: string | null;
  consent_requested_at?: string | null;
  consent_received_at?: string | null;
  consent_document_url?: string | null;
  status: string;
  started_at?: string | null;
  completed_at?: string | null;
  result_summary?: string | null;
  risk_score?: number | null;
  passed?: boolean | null;
  flags?: unknown;
  expires_at?: string | null;
  created_at: string;
  updated_at: string;
}

/** A screening summary (`db::models::lease::ScreeningSummary`). */
export interface ScreeningSummary {
  id: string;
  screening_type: string;
  provider?: string | null;
  status: string;
  risk_score?: number | null;
  passed?: boolean | null;
  completed_at?: string | null;
}

/** A lease template (`db::models::lease::LeaseTemplate`). */
export interface LeaseTemplate {
  id: string;
  organization_id: string;
  name: string;
  description?: string | null;
  content_html: string;
  content_variables?: unknown;
  default_term_months?: number | null;
  default_security_deposit_months?: string | null;
  default_notice_period_days?: number | null;
  clauses?: unknown;
  is_default: boolean;
  is_active: boolean;
  version: number;
  created_by?: string | null;
  created_at: string;
  updated_at: string;
}

// ============================================================================
// Summary / aggregate shapes
// ============================================================================

/** `db::models::lease::ApplicationSummary`. */
export interface ApplicationSummary {
  id: string;
  unit_id: string;
  unit_name: string;
  building_name: string;
  applicant_name: string;
  applicant_email: string;
  status: string;
  submitted_at?: string | null;
  monthly_income?: string | null;
  desired_move_in?: string | null;
  screening_count: number;
  screening_passed: boolean;
}

/** `db::models::lease::LeaseSummary`. */
export interface LeaseSummary {
  id: string;
  unit_id: string;
  unit_name: string;
  building_name: string;
  tenant_name: string;
  tenant_email: string;
  start_date: string;
  end_date: string;
  monthly_rent: string;
  status: string;
  days_until_expiry: number;
}

/** `GET /api/v1/leases/{id}` (`db::models::lease::LeaseWithDetails`). */
export interface LeaseWithDetails {
  lease: Lease;
  unit_name: string;
  building_name: string;
  amendments: LeaseAmendment[];
  upcoming_payments: LeasePayment[];
  reminders: LeaseReminder[];
}

/** `GET /api/v1/leases/expiring` (`db::models::lease::ExpirationOverview`). */
export interface ExpirationOverview {
  expiring_30_days: LeaseSummary[];
  expiring_60_days: LeaseSummary[];
  expiring_90_days: LeaseSummary[];
  total_active_leases: number;
  total_expiring_soon: number;
}

/** `GET /api/v1/leases/statistics` (`db::models::lease::LeaseStatistics`). */
export interface LeaseStatistics {
  total_leases: number;
  active_leases: number;
  pending_signatures: number;
  expiring_soon: number;
  total_applications: number;
  pending_applications: number;
  total_monthly_rent: string;
  occupancy_rate: number;
}

// ============================================================================
// List responses (paginated wrappers)
// ============================================================================

/** `GET /api/v1/leases` (`ListLeasesResponse`). */
export interface ListLeasesResponse {
  items: LeaseSummary[];
  total: number;
}

/** `GET /api/v1/leases/applications` (`ListApplicationsResponse`). */
export interface ListApplicationsResponse {
  items: ApplicationSummary[];
  total: number;
}

// ============================================================================
// Request payloads / queries
// ============================================================================

export interface ListLeasesQuery {
  organization_id: string;
  unit_id?: string;
  tenant_id?: string;
  status?: string;
  expiring_within_days?: number;
  limit?: number;
  offset?: number;
}

export interface ListApplicationsQuery {
  organization_id: string;
  unit_id?: string;
  status?: string;
  limit?: number;
  offset?: number;
}

export interface ExpiringLeasesQuery {
  organization_id: string;
  days?: number;
}

/**
 * Body for `POST /api/v1/leases` (`CreateLeaseRequest`: an `organization_id`
 * with the `CreateLease` fields flattened alongside it).
 */
export interface CreateLeaseRequest {
  organization_id: string;
  unit_id: string;
  application_id?: string;
  template_id?: string;
  landlord_user_id?: string;
  landlord_name: string;
  landlord_address?: string;
  tenant_user_id?: string;
  tenant_name: string;
  tenant_email: string;
  tenant_phone?: string;
  occupants?: unknown;
  start_date: string;
  end_date: string;
  term_months: number;
  is_fixed_term?: boolean;
  monthly_rent: number | string;
  security_deposit: number | string;
  deposit_held_by?: string;
  rent_due_day?: number;
  late_fee_amount?: number | string;
  late_fee_grace_days?: number;
  utilities_included?: unknown;
  parking_spaces?: number;
  storage_units?: number;
  pets_allowed?: boolean;
  pet_deposit?: number | string;
  max_occupants?: number;
  smoking_allowed?: boolean;
  notes?: string;
}

/** Body for `POST /api/v1/leases/applications` (`CreateApplicationRequest`). */
export interface CreateApplicationRequest {
  organization_id: string;
  unit_id: string;
  applicant_name: string;
  applicant_email: string;
  applicant_phone?: string;
  date_of_birth?: string;
  national_id?: string;
  current_address?: string;
  current_landlord_name?: string;
  current_landlord_phone?: string;
  current_rent_amount?: number | string;
  current_tenancy_start?: string;
  employer_name?: string;
  employer_phone?: string;
  job_title?: string;
  employment_start?: string;
  monthly_income?: number | string;
  desired_move_in?: string;
  desired_lease_term_months?: number;
  proposed_rent?: number | string;
  co_applicants?: unknown;
  source?: string;
  referral_code?: string;
}

// ============================================================================
// Violations (`/api/v1/violations`, `db::models::violations::*`)
// ============================================================================

export type ViolationCategory =
  | 'noise'
  | 'parking'
  | 'pet'
  | 'maintenance'
  | 'architectural'
  | 'common_area'
  | 'lease'
  | 'payment'
  | 'safety'
  | 'other';

export type ViolationSeverity = 'minor' | 'moderate' | 'major' | 'critical';

export type ViolationStatus =
  | 'reported'
  | 'under_review'
  | 'confirmed'
  | 'disputed'
  | 'resolved'
  | 'dismissed'
  | 'escalated';

/** A violation (`db::models::violations::Violation`). */
export interface Violation {
  id: string;
  organization_id: string;
  building_id?: string | null;
  unit_id?: string | null;
  violation_number: string;
  rule_id?: string | null;
  category: ViolationCategory;
  severity: ViolationSeverity;
  status: ViolationStatus;
  title: string;
  description: string;
  location?: string | null;
  violator_id?: string | null;
  violator_name?: string | null;
  violator_unit?: string | null;
  reporter_id?: string | null;
  assigned_to?: string | null;
  occurred_at: string;
  reported_at: string;
  reviewed_at?: string | null;
  resolved_at?: string | null;
  evidence_description?: string | null;
  witness_count: number;
  resolution_notes?: string | null;
  resolution_type?: string | null;
  offense_number: number;
  previous_violation_id?: string | null;
  created_at: string;
  updated_at: string;
}

/** `GET /api/v1/violations` row (`db::models::violations::ViolationSummary`). */
export interface ViolationSummary {
  id: string;
  violation_number: string;
  title: string;
  category: ViolationCategory;
  severity: ViolationSeverity;
  status: ViolationStatus;
  violator_name?: string | null;
  violator_unit?: string | null;
  occurred_at: string;
  has_fine: boolean;
  fine_amount?: string | null;
}

/** Query params for `GET /api/v1/violations`. */
export interface ViolationQuery {
  building_id?: string;
  unit_id?: string;
  category?: ViolationCategory;
  severity?: ViolationSeverity;
  status?: ViolationStatus;
  violator_id?: string;
  assigned_to?: string;
  from_date?: string;
  to_date?: string;
  limit?: number;
  offset?: number;
}

/** Body for `POST /api/v1/violations` (`db::models::violations::CreateViolation`). */
export interface CreateViolationRequest {
  building_id?: string;
  unit_id?: string;
  rule_id?: string;
  category: ViolationCategory;
  severity?: ViolationSeverity;
  title: string;
  description: string;
  location?: string;
  violator_id?: string;
  violator_name?: string;
  violator_unit?: string;
  occurred_at: string;
  evidence_description?: string;
  witness_count?: number;
}
