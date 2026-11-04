/**
 * Facility types (Epic 56: Facility Booking).
 */

/** Facility type enum. */
export type FacilityType =
  | 'gym'
  | 'laundry'
  | 'meeting_room'
  | 'party_room'
  | 'sauna'
  | 'pool'
  | 'playground'
  | 'parking'
  | 'storage'
  | 'garden'
  | 'bbq'
  | 'bike_storage'
  | 'other';

/** Booking status enum. */
export type BookingStatus =
  | 'pending'
  | 'approved'
  | 'rejected'
  | 'cancelled'
  | 'completed'
  | 'no_show';

/** Facility entity. */
export interface Facility {
  id: string;
  building_id: string;
  name: string;
  facility_type: FacilityType;
  description: string | null;
  location: string | null;
  capacity: number | null;
  is_active: boolean;
  is_bookable: boolean;
  requires_approval: boolean;
  max_booking_hours: number | null;
  max_advance_days: number | null;
  min_advance_hours: number | null;
  available_from: string | null;
  available_to: string | null;
  available_days: number | null;
  hourly_fee: string | null;
  deposit_amount: string | null;
  photos: string[];
  amenities: string[];
  created_at: string;
  updated_at: string;
}

/** Facility summary for list views. */
export interface FacilitySummary {
  id: string;
  building_id: string;
  name: string;
  facility_type: FacilityType;
  description: string | null;
  location: string | null;
  capacity: number | null;
  is_active: boolean;
  is_bookable: boolean;
  requires_approval: boolean;
  hourly_fee: string | null;
  photos: string[];
}

/** Create facility request. */
export interface CreateFacilityRequest {
  name: string;
  facility_type: string;
  description?: string;
  location?: string;
  capacity?: number;
  is_bookable?: boolean;
  requires_approval?: boolean;
  max_booking_hours?: number;
  max_advance_days?: number;
  min_advance_hours?: number;
  available_from?: string;
  available_to?: string;
  available_days?: number;
  hourly_fee?: string;
  deposit_amount?: string;
  photos?: string[];
  amenities?: string[];
}

/** Update facility request. */
export interface UpdateFacilityRequest {
  name?: string;
  facility_type?: string;
  description?: string;
  location?: string;
  capacity?: number;
  is_active?: boolean;
  is_bookable?: boolean;
  requires_approval?: boolean;
  max_booking_hours?: number;
  max_advance_days?: number;
  min_advance_hours?: number;
  available_from?: string;
  available_to?: string;
  available_days?: number;
  hourly_fee?: string;
  deposit_amount?: string;
  photos?: string[];
  amenities?: string[];
}

/** Facility booking entity. */
export interface FacilityBooking {
  id: string;
  facility_id: string;
  user_id: string;
  start_time: string;
  end_time: string;
  purpose: string | null;
  status: BookingStatus;
  notes: string | null;
  attendees_count: number | null;
  total_fee: string | null;
  deposit_paid: boolean;
  approved_by: string | null;
  approved_at: string | null;
  rejected_by: string | null;
  rejected_at: string | null;
  rejection_reason: string | null;
  cancelled_at: string | null;
  cancellation_reason: string | null;
  created_at: string;
  updated_at: string;
}

/** Booking with related details. */
export interface BookingWithDetails {
  id: string;
  facility_id: string;
  facility_name: string;
  facility_type: FacilityType;
  building_id: string;
  building_name: string | null;
  user_id: string;
  user_name: string | null;
  user_email: string | null;
  start_time: string;
  end_time: string;
  purpose: string | null;
  status: BookingStatus;
  notes: string | null;
  attendees_count: number | null;
  total_fee: string | null;
  deposit_paid: boolean;
  approved_at: string | null;
  rejected_at: string | null;
  rejection_reason: string | null;
  cancelled_at: string | null;
  cancellation_reason: string | null;
  created_at: string;
}

/** Create booking request. */
export interface CreateBookingRequest {
  start_time: string;
  end_time: string;
  purpose?: string;
  notes?: string;
  attendees_count?: number;
}

/** Update booking request. */
export interface UpdateBookingRequest {
  start_time?: string;
  end_time?: string;
  purpose?: string;
  notes?: string;
  attendees_count?: number;
}

/** Cancel booking request. */
export interface CancelBookingRequest {
  reason?: string;
}

/** Reject booking request. */
export interface RejectBookingRequest {
  reason: string;
}

/** Available time slot. */
export interface AvailableSlot {
  start_time: string;
  end_time: string;
  is_available: boolean;
}

/** Availability query parameters. */
export interface AvailabilityQuery {
  /** Calendar day (`YYYY-MM-DD`); expanded to a local-day `from`/`to` range. */
  date: string;
  /** Explicit RFC 3339 range start; overrides `date` together with `to`. */
  from?: string;
  /** Explicit RFC 3339 range end. */
  to?: string;
}

/** List bookings query parameters. */
export interface ListBookingsQuery {
  from_date?: string;
  to_date?: string;
  status?: BookingStatus;
  limit?: number;
  offset?: number;
}

/** List facilities query parameters. */
export interface ListFacilitiesQuery {
  facility_type?: FacilityType;
  is_bookable?: boolean;
  is_active?: boolean;
  limit?: number;
  offset?: number;
}

/** Pending bookings query parameters. */
export interface ListPendingBookingsQuery {
  limit?: number;
  offset?: number;
}

/** My bookings query parameters. */
export interface MyBookingsQuery {
  status?: BookingStatus;
  limit?: number;
  offset?: number;
}

/** Paginated response for facilities. */
export interface FacilityListResponse {
  items: FacilitySummary[];
  total: number;
}

/** Paginated response for bookings. */
export interface BookingListResponse {
  items: BookingWithDetails[];
  total: number;
}
