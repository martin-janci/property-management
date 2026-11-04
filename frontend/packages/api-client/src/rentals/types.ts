/**
 * Short-Term Rentals types (Epic 18, UC-29/30).
 *
 * These mirror the api-server `routes/rentals.rs` + `db::models::rental`
 * payloads exactly (snake_case, `rust_decimal` amounts serialized as strings).
 * The TypeSpec-generated `rentalsApi*` SDK describes a different, unimplemented
 * contract (`/rentals/reservations`, `GET /rentals/guests` envelope, …), so
 * ppt-web talks to the real routes through this hand-written module instead.
 */

/** `rental_platform` enum. */
export type RentalPlatformCode = 'airbnb' | 'booking' | 'vrbo' | 'direct';

/** `rental_booking_status` enum. */
export type RentalBookingStatusCode =
  | 'pending'
  | 'confirmed'
  | 'checked_in'
  | 'checked_out'
  | 'cancelled'
  | 'no_show';

/** `guest_registration_status` enum. */
export type RentalGuestStatusCode = 'pending' | 'registered' | 'reported' | 'expired';

/** Row of `GET /api/v1/rentals/bookings` (`BookingSummary`). */
export interface RentalBookingSummary {
  id: string;
  unit_id: string;
  unit_name: string;
  building_name: string;
  platform: string;
  external_booking_id?: string | null;
  guest_name: string;
  guest_count: number;
  check_in: string;
  check_out: string;
  nights: number;
  total_amount?: string | null;
  currency?: string | null;
  status: string;
  guest_registration_status?: string | null;
}

/** `GET /api/v1/rentals/bookings` response (`BookingsResponse`). */
export interface RentalBookingsPage {
  bookings: RentalBookingSummary[];
  total: number;
  page: number;
  limit: number;
}

/** `GET /api/v1/rentals/bookings` query (`BookingListQuery`). */
export interface RentalBookingListQuery {
  unit_id?: string;
  building_id?: string;
  platform?: string;
  status?: string;
  from_date?: string;
  to_date?: string;
  guest_name?: string;
  page?: number;
  limit?: number;
}

/** Full booking entity (`RentalBooking`). */
export interface RentalBookingRecord {
  id: string;
  organization_id: string;
  unit_id: string;
  connection_id?: string | null;
  platform: string;
  external_booking_id?: string | null;
  external_booking_url?: string | null;
  guest_name: string;
  guest_email?: string | null;
  guest_phone?: string | null;
  guest_count: number;
  check_in: string;
  check_out: string;
  check_in_time?: string | null;
  check_out_time?: string | null;
  total_amount?: string | null;
  currency?: string | null;
  platform_fee?: string | null;
  cleaning_fee?: string | null;
  status: string;
  cancelled_at?: string | null;
  cancellation_reason?: string | null;
  guest_notes?: string | null;
  internal_notes?: string | null;
  synced_at?: string | null;
  created_at: string;
  updated_at: string;
}

/** Guest entity (`RentalGuest`). */
export interface RentalGuestRecord {
  id: string;
  organization_id: string;
  booking_id: string;
  first_name: string;
  last_name: string;
  date_of_birth?: string | null;
  nationality?: string | null;
  id_type?: string | null;
  id_number?: string | null;
  id_issuing_country?: string | null;
  id_expiry_date?: string | null;
  id_document_url?: string | null;
  email?: string | null;
  phone?: string | null;
  address_street?: string | null;
  address_city?: string | null;
  address_postal_code?: string | null;
  address_country?: string | null;
  status: string;
  registered_at?: string | null;
  reported_at?: string | null;
  report_reference?: string | null;
  is_primary: boolean;
  created_at: string;
  updated_at: string;
}

/** `GET /api/v1/rentals/bookings/{id}/guests` response (`BookingWithGuests`). */
export interface RentalBookingWithGuests {
  booking: RentalBookingRecord;
  guests: RentalGuestRecord[];
  registration_complete: boolean;
}

/** Row of `GET /api/v1/rentals/guests` (`GuestListItem`). */
export interface RentalGuestListItem extends RentalGuestRecord {
  booking_guest_name: string;
  unit_id: string;
  unit_name: string;
  check_in: string;
  check_out: string;
  booking_status: string;
}

/** `GET /api/v1/rentals/guests` query (`GuestListQuery`). */
export interface RentalGuestListQuery {
  status?: RentalGuestStatusCode;
  booking_id?: string;
  limit?: number;
}

/** `POST /api/v1/rentals/guests` body (`CreateGuest`). */
export interface RentalCreateGuestBody {
  booking_id: string;
  first_name: string;
  last_name: string;
  date_of_birth?: string;
  nationality?: string;
  id_type?: string;
  id_number?: string;
  id_issuing_country?: string;
  id_expiry_date?: string;
  email?: string;
  phone?: string;
  address_street?: string;
  address_city?: string;
  address_postal_code?: string;
  address_country?: string;
  is_primary?: boolean;
}

/** `PUT /api/v1/rentals/guests/{id}` body (`UpdateGuest`). */
export type RentalUpdateGuestBody = Partial<
  Omit<RentalCreateGuestBody, 'booking_id' | 'is_primary'>
>;

/** `PUT /api/v1/rentals/bookings/{id}/status` body (`UpdateBookingStatus`). */
export interface RentalUpdateBookingStatusBody {
  status: RentalBookingStatusCode;
  cancellation_reason?: string;
}

/** Row of `GET /api/v1/rentals/connections` (`PlatformConnectionSummary`). */
export interface RentalConnectionSummary {
  id: string;
  unit_id: string;
  unit_name: string;
  platform: string;
  is_active: boolean;
  last_sync_at?: string | null;
  sync_error?: string | null;
}

/** `POST /api/v1/rentals/connections` body (`CreatePlatformConnection`). */
export interface RentalCreateConnectionBody {
  unit_id: string;
  platform: RentalPlatformCode;
  external_property_id?: string;
  sync_calendar?: boolean;
  sync_interval_minutes?: number;
  block_other_platforms?: boolean;
}

/** `GET /api/v1/rentals/statistics` response (`RentalStatistics`). */
export interface RentalDashboardStatistics {
  total_units: number;
  connected_units: number;
  active_bookings: number;
  upcoming_bookings: number;
  pending_registrations: number;
  /** Percentage, 0–100. */
  occupancy_rate: number;
  revenue_this_month: string;
  revenue_last_month: string;
}

// ============================================
// Calendar (Story 18.2)
// ============================================

/** `CalendarEvent.event_type`: a booking-linked block or a manual block. */
export type RentalCalendarEventType = 'booking' | 'block';

/**
 * Row of `GET /api/v1/rentals/calendar/{unit_id}` (`CalendarEvent`).
 *
 * `id` is the `rental_calendar_blocks` row id (not the booking id). Dates are
 * half-open: `end_date` is the check-out day / first free day.
 */
export interface RentalCalendarEvent {
  id: string;
  unit_id: string;
  start_date: string;
  end_date: string;
  event_type: RentalCalendarEventType | string;
  /** Guest name for bookings, "Maintenance" / "Owner Use" / "Blocked" for blocks. */
  title: string;
  platform?: string | null;
  booking_status?: string | null;
  /** Hex colour hint from the server. */
  color: string;
}

/** `GET /api/v1/rentals/calendar/{unit_id}` query (`CalendarQuery`); both required. */
export interface RentalCalendarQuery {
  start_date: string;
  end_date: string;
}

/** `GET /api/v1/rentals/calendar/{unit_id}/availability` response. */
export interface RentalAvailability {
  available: boolean;
  start_date: string;
  end_date: string;
}

/** `block_reason` values accepted for a manual block. */
export type RentalCalendarBlockReason = 'maintenance' | 'owner_use' | 'blocked';

/** `POST /api/v1/rentals/calendar/blocks` body (`CreateCalendarBlock`). */
export interface RentalCreateCalendarBlockBody {
  unit_id: string;
  block_start: string;
  /** Exclusive end (first day the unit is free again). */
  block_end: string;
  reason: RentalCalendarBlockReason;
  notes?: string | null;
}

/** `CalendarBlock` returned by the create route. */
export interface RentalCalendarBlock {
  id: string;
  organization_id: string;
  unit_id: string;
  block_start: string;
  block_end: string;
  reason: string;
  booking_id?: string | null;
  source_platform?: string | null;
  synced_at?: string | null;
  notes?: string | null;
  created_at: string;
}

// ============================================
// Authority guest reports (Story 18.4)
// ============================================

/** `report_type` values. */
export type RentalReportType = 'monthly' | 'quarterly' | 'annual';

/** Authority codes the server knows (`authority_code`). */
export type RentalReportAuthorityCode = 'SK_UHUL' | 'CZ_CIZPOL' | 'AT_ZMR' | 'DE_MELDEWESEN';

/** `report_status` values. */
export type RentalReportStatus = 'draft' | 'generated' | 'submitted' | 'confirmed';

/** `NationalityStats`. */
export interface RentalNationalityStats {
  /** ISO country code, `UNK` when the guest has none. */
  nationality: string;
  country_name: string;
  count: number;
  percentage: number;
}

/** `POST /api/v1/rentals/reports/preview` body (`ReportPreviewRequest`). */
export interface RentalReportPreviewBody {
  building_id: string;
  period_start: string;
  period_end: string;
}

/** `ReportPreview`. Counts only registered/reported guests of stays fully inside the period. */
export interface RentalReportPreview {
  building_id: string;
  building_name: string;
  period_start: string;
  period_end: string;
  total_guests: number;
  by_nationality: RentalNationalityStats[];
  bookings_count: number;
}

/** `GET /api/v1/rentals/reports` query (`ListReportsQuery`). */
export interface RentalReportListQuery {
  building_id?: string;
  limit?: number;
}

/** Row of `GET /api/v1/rentals/reports` (`ReportSummary`). */
export interface RentalReportSummary {
  id: string;
  building_id: string;
  building_name: string;
  report_type: string;
  period_start: string;
  period_end: string;
  authority_code: string;
  authority_name: string;
  total_guests: number;
  status: string;
  report_file_url?: string | null;
  submitted_at?: string | null;
}

/** `POST /api/v1/rentals/reports` body (`GenerateReport`). */
export interface RentalGenerateReportBody {
  building_id: string;
  report_type: RentalReportType;
  period_start: string;
  period_end: string;
  authority_code: RentalReportAuthorityCode;
  /** Defaults server-side; no file is rendered yet. */
  report_format?: string;
}

/** `RentalGuestReport` returned by create / get / submit. */
export interface RentalGuestReport {
  id: string;
  organization_id: string;
  building_id: string;
  report_type: string;
  period_start: string;
  period_end: string;
  authority_code: string;
  authority_name: string;
  total_guests: number;
  guests_by_nationality?: RentalNationalityStats[] | null;
  report_file_url?: string | null;
  report_format: string;
  status: string;
  submitted_at?: string | null;
  submission_reference?: string | null;
  submission_response?: string | null;
  generated_by?: string | null;
  submitted_by?: string | null;
  created_at: string;
  updated_at: string;
}

// ============================================
// Building / unit pickers
// ============================================

/** Row of `GET /api/v1/buildings` (`BuildingSummary`, snake_case on the wire). */
export interface RentalBuildingListItem {
  id: string;
  name?: string | null;
  street: string;
  city: string;
  postal_code: string;
  status: string;
}

/** `GET /api/v1/buildings` response (`BuildingsListResponse`). */
export interface RentalBuildingsPage {
  buildings: RentalBuildingListItem[];
  total: number;
  offset: number;
  limit: number;
}

/** Row of `GET /api/v1/buildings/{id}/units` (`UnitSummary`). */
export interface RentalBuildingUnitItem {
  id: string;
  building_id: string;
  designation: string;
  floor: number;
  unit_type: string;
  status: string;
}

/** `GET /api/v1/buildings/{id}/units` response (`UnitsListResponse`). */
export interface RentalBuildingUnitsPage {
  units: RentalBuildingUnitItem[];
  total: number;
  offset: number;
  limit: number;
}
