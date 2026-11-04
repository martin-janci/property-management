/**
 * Types for Short-Term Rentals feature.
 * Epic 18: Short-Term Rental Integration (UC-29, UC-30)
 */

export type BookingStatus =
  | 'pending'
  | 'confirmed'
  | 'checked_in'
  | 'checked_out'
  | 'cancelled'
  | 'no_show';

export type BookingSource = 'airbnb' | 'booking' | 'direct' | 'other';

export type PlatformType = 'airbnb' | 'booking';

export type ConnectionStatus = 'connected' | 'disconnected' | 'pending' | 'error';

export type CalendarEventType = 'booking' | 'block' | 'maintenance';

export type GuestRegistrationStatus = 'pending' | 'registered' | 'failed';

export interface RentalBooking {
  id: string;
  unitId: string;
  unitName?: string;
  buildingId: string;
  buildingName?: string;
  guestName: string;
  guestEmail?: string;
  guestPhone?: string;
  checkIn: string;
  checkOut: string;
  status: BookingStatus;
  source: BookingSource;
  platformBookingId?: string;
  totalPrice?: number;
  currency?: string;
  guestCount: number;
  notes?: string;
  createdAt: string;
  updatedAt: string;
}

export interface RentalGuest {
  id: string;
  bookingId: string;
  firstName: string;
  lastName: string;
  email?: string;
  phone?: string;
  dateOfBirth?: string;
  nationality?: string;
  documentType?: string;
  documentNumber?: string;
  documentExpiry?: string;
  address?: string;
  city?: string;
  country?: string;
  registrationStatus: GuestRegistrationStatus;
  registeredAt?: string;
  isPrimary: boolean;
  createdAt: string;
}

export interface CalendarEvent {
  id: string;
  unitId: string;
  type: CalendarEventType;
  title: string;
  startDate: string;
  endDate: string;
  bookingId?: string;
  guestName?: string;
  source?: BookingSource;
  status?: BookingStatus;
  color?: string;
  notes?: string;
  /** Manual blocks only. */
  blockReason?: CalendarBlockReason;
}

export interface PlatformConnection {
  id: string;
  unitId: string;
  unitName?: string;
  platform: PlatformType;
  status: ConnectionStatus;
  platformPropertyId?: string;
  lastSyncAt?: string;
  syncErrorMessage?: string;
  isAutoSyncEnabled: boolean;
  createdAt: string;
}

export interface RentalStatistics {
  totalBookings: number;
  activeBookings: number;
  upcomingBookings: number;
  totalRevenue: number;
  occupancyRate: number;
  averageStayDuration: number;
  pendingGuestRegistrations: number;
  currency: string;
}

export interface BookingWithGuests extends RentalBooking {
  guests: RentalGuest[];
}

export interface CreateBookingRequest {
  unitId: string;
  guestName: string;
  guestEmail?: string;
  guestPhone?: string;
  checkIn: string;
  checkOut: string;
  source: BookingSource;
  totalPrice?: number;
  currency?: string;
  guestCount: number;
  notes?: string;
}

export interface CreateGuestRequest {
  bookingId: string;
  firstName: string;
  lastName: string;
  email?: string;
  phone?: string;
  dateOfBirth?: string;
  nationality?: string;
  documentType?: string;
  documentNumber?: string;
  documentExpiry?: string;
  address?: string;
  city?: string;
  country?: string;
  isPrimary: boolean;
}

export interface UpdateGuestRequest {
  firstName?: string;
  lastName?: string;
  email?: string;
  phone?: string;
  dateOfBirth?: string;
  nationality?: string;
  documentType?: string;
  documentNumber?: string;
  documentExpiry?: string;
  address?: string;
  city?: string;
  country?: string;
}

export interface CreatePlatformConnectionRequest {
  unitId: string;
  platform: PlatformType;
  platformPropertyId?: string;
  isAutoSyncEnabled: boolean;
}

export interface CalendarBlockRequest {
  unitId: string;
  startDate: string;
  endDate: string;
  reason?: string;
}

export interface BookingListParams {
  unitId?: string;
  buildingId?: string;
  platform?: BookingSource;
  status?: BookingStatus;
  fromDate?: string;
  toDate?: string;
  page?: number;
  limit?: number;
}

/**
 * Calendar + property pickers (Story 18.2).
 */

/** Building option for the rentals pickers. */
export interface RentalBuildingOption {
  id: string;
  name: string;
}

/** Unit option for the rentals pickers (unit designation + its building). */
export interface RentalUnitOption {
  id: string;
  name: string;
  buildingId: string;
  buildingName: string;
}

/** Reason for a manual calendar block (`block_reason`). */
export type CalendarBlockReason = 'maintenance' | 'owner_use' | 'blocked';

/**
 * Daňový výkaz / authority guest report (Story 18.4, UC-29).
 *
 * The api-server reports guest counts by nationality per building and period
 * (the basis for the local accommodation tax and the authority filing). It does
 * not compute rental income, expenses or income tax.
 */

export type GuestReportPeriodType = 'monthly' | 'quarterly' | 'annual';

export type GuestReportAuthority = 'SK_UHUL' | 'CZ_CIZPOL' | 'AT_ZMR' | 'DE_MELDEWESEN';

export type GuestReportStatus = 'draft' | 'generated' | 'submitted' | 'confirmed';

export interface GuestReportParams {
  buildingId: string;
  authorityCode: GuestReportAuthority;
  periodType: GuestReportPeriodType;
  /** ISO date, first day of the period. */
  periodStart: string;
  /** ISO date, last day of the period. */
  periodEnd: string;
}

export interface GuestReportNationalityRow {
  /** ISO country code, `UNK` when unknown. */
  nationality: string;
  countryName: string;
  count: number;
  /** 0–100. */
  percentage: number;
}

export interface GuestReportPreviewData {
  buildingId: string;
  buildingName: string;
  periodStart: string;
  periodEnd: string;
  totalGuests: number;
  bookingsCount: number;
  byNationality: GuestReportNationalityRow[];
}

export interface GuestReportSummaryItem {
  id: string;
  buildingId: string;
  buildingName: string;
  periodType: string;
  periodStart: string;
  periodEnd: string;
  authorityCode: string;
  authorityName: string;
  totalGuests: number;
  status: GuestReportStatus | string;
  fileUrl?: string;
  submittedAt?: string;
}

export interface TaxSummary {
  year: number;
  ytdIncome: number;
  ytdExpenses: number;
  ytdNetProfit: number;
  estimatedTax: number;
  taxRate: number;
  previousYearIncome: number;
  previousYearTax: number;
  incomeChangePercent: number;
  currency: string;
}
