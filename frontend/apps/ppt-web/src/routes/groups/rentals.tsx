/**
 * Short-Term Rentals route group (Epic 18, UC-29/30).
 *
 * Owns the rentals API↔UI mappers, the `useRentalsAuth` helper, the lazy page
 * imports, the route-wrapper components, and the `<Route>` table fragment.
 * Extracted from App.tsx to isolate rentals work.
 *
 * Data comes from the real api-server routes under `/api/v1/rentals`
 * (`bookings`, `bookings/{id}/guests`, `bookings/{id}/status`, `guests`,
 * `connections`, `statistics`, `calendar/{unit_id}`, `calendar/blocks`,
 * `reports`, `reports/preview`) via the hand-written `@ppt/api-client` rentals
 * module. Building/unit pickers load `GET /buildings` + `GET /buildings/{id}/units`. The TypeSpec-generated `rentalsApi*` SDK targets routes the server
 * never implemented (`/rentals/reservations`, `/rentals/sync`, …) and 404/405s.
 */
import type {
  RentalBookingRecord,
  RentalBookingSummary,
  RentalConnectionSummary,
  RentalDashboardStatistics,
  RentalGuestListItem,
  RentalGuestRecord,
  RentalReportType,
  RentalsRequestHeaders,
  RentalUpdateGuestBody,
} from '@ppt/api-client';
import {
  checkRentalAvailability,
  createRentalCalendarBlock,
  createRentalConnection,
  createRentalGuest,
  createRentalReport,
  deleteRentalCalendarBlock,
  deleteRentalGuest,
  getRentalBookingWithGuests,
  getRentalCalendar,
  getRentalStatistics,
  getToken,
  listRentalBookings,
  listRentalBuildings,
  listRentalBuildingUnits,
  listRentalConnections,
  listRentalGuests,
  listRentalReports,
  previewRentalReport,
  registerRentalGuest,
  submitRentalReport,
  syncAirbnb,
  syncBooking,
  updateRentalBookingStatus,
  updateRentalGuest,
} from '@ppt/api-client';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { lazy, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Route, useNavigate, useParams, useSearchParams } from 'react-router-dom';
import { ProtectedRoute, type Toast, useToast } from '../../components';
import { AuthError, type AuthErrorCode, useAuth } from '../../contexts';
import {
  addDaysIso,
  buildPropertyOptions,
  calendarFetchRange,
  mapCalendarEventToUi,
  mapReportPreviewToUi,
  mapReportSummaryToUi,
} from '../../features/rentals/mappers';
import type { CalendarBlockInput } from '../../features/rentals/pages/CalendarPage';
import type {
  BookingListParams,
  BookingWithGuests,
  CreateGuestRequest,
  GuestReportParams,
  RentalBooking,
  BookingStatus as RentalBookingStatus,
  RentalBuildingOption,
  CalendarEvent as RentalCalendarEvent,
  RentalGuest,
  PlatformConnection as RentalPlatformConnection,
  PlatformType as RentalPlatformType,
  RentalStatistics,
  RentalUnitOption,
  UpdateGuestRequest,
} from '../../features/rentals/types';
import { MANAGER_ROLES } from '../shared';

// Rentals feature pages (Epic 18 / UC-29, UC-30) — lazy-loaded for code splitting.
const RentalsDashboardPage = lazy(() =>
  import('../../features/rentals').then((m) => ({ default: m.RentalsDashboardPage }))
);
const PlatformConnectionsPage = lazy(() =>
  import('../../features/rentals').then((m) => ({ default: m.PlatformConnectionsPage }))
);
const BookingsPage = lazy(() =>
  import('../../features/rentals').then((m) => ({ default: m.BookingsPage }))
);
const BookingDetailPage = lazy(() =>
  import('../../features/rentals').then((m) => ({ default: m.BookingDetailPage }))
);
const RentalsCalendarPage = lazy(() =>
  import('../../features/rentals').then((m) => ({ default: m.CalendarPage }))
);
const GuestRegistrationPage = lazy(() =>
  import('../../features/rentals').then((m) => ({ default: m.GuestRegistrationPage }))
);
const TaxReportPage = lazy(() =>
  import('../../features/rentals').then((m) => ({ default: m.TaxReportPage }))
);

/**
 * Map a backend `rental_platform` value → the page-level BookingSource.
 * The backend enum carries 'vrbo' which the page lacks; fold it (and anything
 * unknown) to 'other'.
 */
export function mapRentalPlatformToSource(platform: string): RentalBooking['source'] {
  switch (platform) {
    case 'airbnb':
      return 'airbnb';
    case 'booking':
      return 'booking';
    case 'direct':
      return 'direct';
    default:
      return 'other';
  }
}

/** Map a backend `rental_platform` value → the page PlatformType (airbnb | booking). */
export function mapRentalPlatformToType(platform: string): RentalPlatformType {
  return platform === 'booking' ? 'booking' : 'airbnb';
}

/** Parse a `rust_decimal` amount (serialized as a string) into a number. */
export function parseRentalAmount(amount: string | null | undefined): number | undefined {
  if (amount === null || amount === undefined || amount === '') return undefined;
  const n = Number(amount);
  return Number.isFinite(n) ? n : undefined;
}

/** Map a `GET /rentals/bookings` row → the page RentalBooking. */
export function mapBookingSummaryToUi(row: RentalBookingSummary): RentalBooking {
  return {
    id: row.id,
    unitId: row.unit_id,
    unitName: row.unit_name,
    // The summary carries the building name but not its id.
    buildingId: '',
    buildingName: row.building_name || undefined,
    guestName: row.guest_name,
    checkIn: row.check_in,
    checkOut: row.check_out,
    status: row.status as RentalBookingStatus,
    source: mapRentalPlatformToSource(row.platform),
    platformBookingId: row.external_booking_id ?? undefined,
    totalPrice: parseRentalAmount(row.total_amount),
    currency: row.currency ?? undefined,
    guestCount: row.guest_count,
    // The list row has no timestamps; detail views use the full record.
    createdAt: '',
    updatedAt: '',
  };
}

/** Map a full `RentalBooking` record → the page RentalBooking. */
export function mapBookingRecordToUi(rec: RentalBookingRecord): RentalBooking {
  return {
    id: rec.id,
    unitId: rec.unit_id,
    buildingId: '',
    guestName: rec.guest_name,
    guestEmail: rec.guest_email ?? undefined,
    guestPhone: rec.guest_phone ?? undefined,
    checkIn: rec.check_in,
    checkOut: rec.check_out,
    status: rec.status as RentalBookingStatus,
    source: mapRentalPlatformToSource(rec.platform),
    platformBookingId: rec.external_booking_id ?? undefined,
    totalPrice: parseRentalAmount(rec.total_amount),
    currency: rec.currency ?? undefined,
    guestCount: rec.guest_count,
    notes: rec.guest_notes ?? rec.internal_notes ?? undefined,
    createdAt: rec.created_at,
    updatedAt: rec.updated_at,
  };
}

/**
 * Map a `GET /rentals/connections` row → the page PlatformConnection.
 * Inactive → 'disconnected'; a recorded sync error → 'error'; a completed sync
 * → 'connected'; otherwise the connection has not synced yet → 'pending'.
 */
export function mapConnectionSummaryToUi(conn: RentalConnectionSummary): RentalPlatformConnection {
  let status: RentalPlatformConnection['status'];
  if (!conn.is_active) status = 'disconnected';
  else if (conn.sync_error) status = 'error';
  else if (conn.last_sync_at) status = 'connected';
  else status = 'pending';
  return {
    id: conn.id,
    unitId: conn.unit_id,
    unitName: conn.unit_name,
    platform: mapRentalPlatformToType(conn.platform),
    status,
    lastSyncAt: conn.last_sync_at ?? undefined,
    syncErrorMessage: conn.sync_error ?? undefined,
    isAutoSyncEnabled: conn.is_active,
    // The summary row carries no creation timestamp.
    createdAt: '',
  };
}

/**
 * Map a backend `guest_registration_status` → the page status.
 * 'registered' and 'reported' (already sent to the authority) are both done;
 * 'expired' is a failed registration.
 */
export function mapGuestStatusToUi(status: string): RentalGuest['registrationStatus'] {
  switch (status) {
    case 'registered':
    case 'reported':
      return 'registered';
    case 'expired':
      return 'failed';
    default:
      return 'pending';
  }
}

/** Map a backend `RentalGuest` → the page RentalGuest. */
export function mapGuestRecordToUi(guest: RentalGuestRecord): RentalGuest {
  return {
    id: guest.id,
    bookingId: guest.booking_id,
    firstName: guest.first_name,
    lastName: guest.last_name,
    email: guest.email ?? undefined,
    phone: guest.phone ?? undefined,
    dateOfBirth: guest.date_of_birth ?? undefined,
    nationality: guest.nationality ?? undefined,
    documentType: guest.id_type ?? undefined,
    documentNumber: guest.id_number ?? undefined,
    documentExpiry: guest.id_expiry_date ?? undefined,
    address: guest.address_street ?? undefined,
    city: guest.address_city ?? undefined,
    country: guest.address_country ?? undefined,
    registrationStatus: mapGuestStatusToUi(guest.status),
    registeredAt: guest.registered_at ?? undefined,
    isPrimary: guest.is_primary,
    createdAt: guest.created_at,
  };
}

/** A booking + its guests, as the guest-registration page lists them. */
export interface GuestRegistrationBooking {
  id: string;
  guestName: string;
  unitName: string;
  checkIn: string;
  checkOut: string;
  guests: RentalGuest[];
}

/**
 * Group `GET /rentals/guests` rows by booking, keeping only bookings that still
 * have at least one guest awaiting registration (the page's purpose). Row order
 * from the server (check-in, then primary guest first) is preserved.
 */
export function groupGuestsByBooking(rows: RentalGuestListItem[]): GuestRegistrationBooking[] {
  const byBooking = new Map<string, GuestRegistrationBooking>();
  for (const row of rows) {
    let entry = byBooking.get(row.booking_id);
    if (!entry) {
      entry = {
        id: row.booking_id,
        guestName: row.booking_guest_name,
        unitName: row.unit_name,
        checkIn: row.check_in,
        checkOut: row.check_out,
        guests: [],
      };
      byBooking.set(row.booking_id, entry);
    }
    entry.guests.push(mapGuestRecordToUi(row));
  }
  return Array.from(byBooking.values()).filter((b) =>
    b.guests.some((g) => g.registrationStatus === 'pending')
  );
}

/** Map the page guest form → the backend `UpdateGuest` body. */
export function mapGuestFormToApi(data: UpdateGuestRequest): RentalUpdateGuestBody {
  return {
    first_name: data.firstName,
    last_name: data.lastName,
    email: data.email,
    phone: data.phone,
    date_of_birth: data.dateOfBirth,
    nationality: data.nationality,
    id_type: data.documentType,
    id_number: data.documentNumber,
    id_expiry_date: data.documentExpiry,
    address_street: data.address,
    address_city: data.city,
    address_country: data.country,
  };
}

/**
 * Map backend dashboard statistics → the page RentalStatistics.
 * The server reports occupancy as a percentage (0–100); the page formats a
 * fraction, so it is divided by 100 here. The average stay is derived from the
 * listed bookings' `nights` since the statistics endpoint does not carry it.
 */
export function mapStatisticsToUi(
  stats: RentalDashboardStatistics | undefined,
  bookings: RentalBookingSummary[],
  totalBookings: number
): RentalStatistics {
  const stays = bookings.filter((b) => b.nights > 0);
  const averageStayDuration =
    stays.length > 0 ? stays.reduce((sum, b) => sum + b.nights, 0) / stays.length : 0;
  return {
    totalBookings,
    activeBookings: stats?.active_bookings ?? 0,
    upcomingBookings: stats?.upcoming_bookings ?? 0,
    totalRevenue: parseRentalAmount(stats?.revenue_this_month) ?? 0,
    occupancyRate: (stats?.occupancy_rate ?? 0) / 100,
    averageStayDuration,
    pendingGuestRegistrations: stats?.pending_registrations ?? 0,
    currency: bookings.find((b) => b.currency)?.currency ?? 'EUR',
  };
}

/**
 * Build the auth header + tenant id required by the rentals routes (which
 * resolve the tenant from `X-Tenant-ID`). Returns null when either is missing so
 * the calling query stays disabled.
 */
function useRentalsAuth(): RentalsAuth | null {
  const { user } = useAuth();
  const token = getToken();
  if (!token || !user?.organizationId) return null;
  return { authorization: `Bearer ${token}`, xTenantId: user.organizationId };
}

type RentalsAuth = { authorization: string; xTenantId: string };
type RentalsAuthHeaders = { Authorization: string; 'X-Tenant-ID': string };

/**
 * Build the auth headers every rentals request requires,
 * or throw a typed {@link AuthError} when the session was lost mid-flight.
 *
 * Queries gate on `enabled: !!auth`, so their `queryFn` never runs while auth is
 * null. Mutations have no such gate — TanStack Query invokes a `mutationFn`
 * unconditionally on `.mutate()`. Previously the mutations dereferenced
 * `auth!.authorization`, so a mid-session token loss surfaced as an opaque
 * `TypeError: Cannot read properties of null` with no auth semantics. Throwing
 * an `AuthError('SESSION_EXPIRED')` instead routes through TanStack Query's
 * rejected-promise path, so the mutation lands in a handled error state and
 * `onError` (see {@link handleRentalsAuthError}) can surface the app's
 * session-expired UX (redirect to /login) instead of an uncaught crash.
 */
export function requireRentalsAuthHeaders(auth: RentalsAuth | null): RentalsAuthHeaders {
  if (!auth) {
    throw new AuthError('Your session has expired. Please sign in again.', 'SESSION_EXPIRED');
  }
  return { Authorization: auth.authorization, 'X-Tenant-ID': auth.xTenantId };
}

/**
 * Auth-error codes that mean "the session is gone" — a failed rentals mutation
 * carrying one of these should trigger the logout/redirect flow.
 */
const SESSION_LOST_CODES: ReadonlySet<AuthErrorCode> = new Set<AuthErrorCode>([
  'SESSION_EXPIRED',
  'TOKEN_EXPIRED',
  'TOKEN_INVALID',
]);

/**
 * Shared `onError` logic for the rentals mutations: when a mutation fails
 * because the session was lost, clear it and redirect to /login (via the
 * AuthContext `logout`), matching how the rest of the app handles a dropped
 * session. Non-auth errors are left for the caller to surface. Returns whether
 * the error was handled as a session loss.
 */
export function handleRentalsAuthError(error: unknown, logout: () => void): boolean {
  if (error instanceof AuthError && SESSION_LOST_CODES.has(error.code)) {
    logout();
    return true;
  }
  return false;
}

/**
 * `onError` handler shared by the rentals mutations (create connection, sync
 * platforms, guest check-in/out).
 *
 * Previously each mutation's `onError` called {@link handleRentalsAuthError}
 * alone and discarded its boolean result. That silently swallowed every
 * *non-auth* failure — a network drop, a 4xx validation error, a 5xx — so a
 * failed "Create connection" / "Sync" / "Check in" / "Check out" left the UI
 * unchanged with no feedback, and the user had no idea the action never
 * happened (code-review-ppt-web-core-rentals-mutation-silent).
 *
 * This wrapper keeps the session-loss path (redirect to /login) and, for every
 * other error, surfaces an error toast — matching how the rest of the app's
 * mutations report failures. The `title` / `fallbackMessage` are passed already
 * translated so this helper stays pure and unit-testable.
 */
export function handleRentalsMutationError(
  error: unknown,
  opts: {
    logout: () => void;
    showToast: (toast: Omit<Toast, 'id'>) => void;
    title: string;
    fallbackMessage: string;
  }
): void {
  if (handleRentalsAuthError(error, opts.logout)) return;
  opts.showToast({
    type: 'error',
    title: opts.title,
    message: error instanceof Error ? error.message : opts.fallbackMessage,
  });
}

/** Route wrapper for the rentals dashboard. */
function RentalsDashboardPageRoute() {
  const navigate = useNavigate();
  const auth = useRentalsAuth();

  const { data: bookingsPage, isLoading } = useQuery({
    queryKey: ['rentals', 'bookings', auth?.xTenantId, 'dashboard'],
    queryFn: () => listRentalBookings({ limit: 100 }, requireRentalsAuthHeaders(auth)),
    enabled: !!auth,
  });
  const { data: stats } = useQuery({
    queryKey: ['rentals', 'statistics', auth?.xTenantId],
    queryFn: () => getRentalStatistics(requireRentalsAuthHeaders(auth)),
    enabled: !!auth,
  });
  const { data: connData } = useQuery({
    queryKey: ['rentals', 'connections', auth?.xTenantId],
    queryFn: () => listRentalConnections(requireRentalsAuthHeaders(auth)),
    enabled: !!auth,
  });

  const rows = bookingsPage?.bookings ?? [];
  const bookings = rows.map(mapBookingSummaryToUi);
  const connections = (connData ?? []).map(mapConnectionSummaryToUi);
  const today = new Date().toISOString().slice(0, 10);
  const upcomingBookings = bookings
    .filter((b) => b.checkIn > today && (b.status === 'pending' || b.status === 'confirmed'))
    .sort((a, b) => a.checkIn.localeCompare(b.checkIn));
  const activeBookings = bookings.filter((b) => b.status === 'checked_in');

  const statistics = mapStatisticsToUi(stats, rows, bookingsPage?.total ?? bookings.length);

  return (
    <RentalsDashboardPage
      statistics={statistics}
      upcomingBookings={upcomingBookings}
      activeBookings={activeBookings}
      platformConnections={connections}
      isLoading={isLoading}
      onViewBooking={(id) => navigate(`/rentals/bookings/${id}`)}
      onCheckIn={(id) => navigate(`/rentals/bookings/${id}`)}
      onCheckOut={(id) => navigate(`/rentals/bookings/${id}`)}
      onViewAllBookings={() => navigate('/rentals/bookings')}
      onViewCalendar={() => navigate('/rentals/calendar')}
      onViewConnections={() => navigate('/rentals/connections')}
      onViewGuestRegistration={() => navigate('/rentals/guests')}
      onViewTaxReport={() => navigate('/rentals/reports')}
    />
  );
}

/** Route wrapper for platform connections. */
function PlatformConnectionsPageRoute() {
  const navigate = useNavigate();
  const auth = useRentalsAuth();
  const properties = useRentalPropertyOptions(auth);
  const { logout } = useAuth();
  const queryClient = useQueryClient();
  const { showToast } = useToast();
  const { t } = useTranslation();

  const { data, isLoading } = useQuery({
    queryKey: ['rentals', 'connections', auth?.xTenantId],
    queryFn: () => listRentalConnections(requireRentalsAuthHeaders(auth)),
    enabled: !!auth,
  });

  const createConnection = useMutation({
    mutationFn: (vars: { unitId: string; platform: RentalPlatformType }) =>
      createRentalConnection(
        { unit_id: vars.unitId, platform: vars.platform },
        requireRentalsAuthHeaders(auth)
      ),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['rentals', 'connections'] });
    },
    onError: (error) =>
      handleRentalsMutationError(error, {
        logout,
        showToast,
        title: t('rentals.connections.createFailed', {
          defaultValue: 'Failed to create connection',
        }),
        fallbackMessage: t('common.pleaseTryAgain', { defaultValue: 'Please try again.' }),
      }),
  });

  // There is no per-connection sync route; each channel syncs org-wide through
  // its integrations endpoint (`/integrations/organizations/{org}/{airbnb|booking}/sync`).
  const syncPlatform = useMutation({
    mutationFn: (platform: RentalPlatformType) => {
      const { 'X-Tenant-ID': orgId } = requireRentalsAuthHeaders(auth);
      return platform === 'booking' ? syncBooking(orgId) : syncAirbnb(orgId);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['rentals', 'connections'] });
    },
    onError: (error) =>
      handleRentalsMutationError(error, {
        logout,
        showToast,
        title: t('rentals.connections.syncFailed', { defaultValue: 'Failed to sync platform' }),
        fallbackMessage: t('common.pleaseTryAgain', { defaultValue: 'Please try again.' }),
      }),
  });

  const connections = (data ?? []).map(mapConnectionSummaryToUi);

  return (
    <PlatformConnectionsPage
      connections={connections}
      units={properties.data?.units ?? []}
      isLoading={isLoading}
      onConnect={() => {}}
      onDisconnect={() => {}}
      onSync={(connectionId) => {
        const conn = connections.find((c) => c.id === connectionId);
        if (conn) syncPlatform.mutate(conn.platform);
      }}
      onSettings={() => {}}
      onCreateConnection={(unitId, platform) => createConnection.mutate({ unitId, platform })}
      onBack={() => navigate('/rentals')}
    />
  );
}

/**
 * Shared booking-status mutation (check-in / check-out / cancel) via
 * `PUT /rentals/bookings/{id}/status`. Invalidates every rentals booking query.
 */
function useBookingStatusMutation(auth: RentalsAuth | null) {
  const { logout } = useAuth();
  const queryClient = useQueryClient();
  const { showToast } = useToast();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: (vars: { id: string; status: 'checked_in' | 'checked_out' | 'cancelled' }) =>
      updateRentalBookingStatus(vars.id, { status: vars.status }, requireRentalsAuthHeaders(auth)),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['rentals'] });
    },
    onError: (error, vars) =>
      handleRentalsMutationError(error, {
        logout,
        showToast,
        title:
          vars.status === 'checked_in'
            ? t('rentals.bookingDetail.checkInFailed', { defaultValue: 'Failed to check in' })
            : vars.status === 'checked_out'
              ? t('rentals.bookingDetail.checkOutFailed', { defaultValue: 'Failed to check out' })
              : t('rentals.bookingDetail.cancelFailed', {
                  defaultValue: 'Failed to cancel booking',
                }),
        fallbackMessage: t('common.pleaseTryAgain', { defaultValue: 'Please try again.' }),
      }),
  });
}

/** Route wrapper for the bookings list. */
function BookingsPageRoute() {
  const navigate = useNavigate();
  const auth = useRentalsAuth();
  const [filters, setFilters] = useState<BookingListParams>({});
  const updateStatus = useBookingStatusMutation(auth);
  const properties = useRentalPropertyOptions(auth);

  const { data, isLoading } = useQuery({
    queryKey: ['rentals', 'bookings', auth?.xTenantId, filters],
    queryFn: () =>
      listRentalBookings(
        {
          unit_id: filters.unitId,
          building_id: filters.buildingId,
          platform: filters.platform,
          status: filters.status,
          from_date: filters.fromDate,
          to_date: filters.toDate,
          page: filters.page,
          limit: filters.limit,
        },
        requireRentalsAuthHeaders(auth)
      ),
    enabled: !!auth,
  });

  const bookings = (data?.bookings ?? []).map(mapBookingSummaryToUi);

  return (
    <BookingsPage
      bookings={bookings}
      total={data?.total ?? bookings.length}
      buildings={properties.data?.buildings ?? []}
      units={properties.data?.units ?? []}
      isLoading={isLoading}
      onFilterChange={setFilters}
      onViewBooking={(id) => navigate(`/rentals/bookings/${id}`)}
      onEditBooking={(id) => navigate(`/rentals/bookings/${id}`)}
      onCancelBooking={(id) => updateStatus.mutate({ id, status: 'cancelled' })}
      onCheckIn={(id) => updateStatus.mutate({ id, status: 'checked_in' })}
      onCheckOut={(id) => updateStatus.mutate({ id, status: 'checked_out' })}
      onCreateBooking={() => navigate('/rentals/bookings')}
      onBack={() => navigate('/rentals')}
    />
  );
}

/** Guest mutations shared by the booking detail and guest-registration pages. */
function useGuestMutations(auth: RentalsAuth | null) {
  const { logout } = useAuth();
  const queryClient = useQueryClient();
  const { showToast } = useToast();
  const { t } = useTranslation();

  const onError = (titleKey: string, defaultTitle: string) => (error: unknown) =>
    handleRentalsMutationError(error, {
      logout,
      showToast,
      title: t(titleKey, { defaultValue: defaultTitle }),
      fallbackMessage: t('common.pleaseTryAgain', { defaultValue: 'Please try again.' }),
    });
  const onSuccess = () => queryClient.invalidateQueries({ queryKey: ['rentals'] });

  const register = useMutation({
    mutationFn: (guestIds: string[]) =>
      Promise.all(guestIds.map((id) => registerRentalGuest(id, requireRentalsAuthHeaders(auth)))),
    onSuccess,
    onError: onError('rentals.guests.registerFailed', 'Failed to register guest'),
  });
  const update = useMutation({
    mutationFn: (vars: { id: string; data: UpdateGuestRequest }) =>
      updateRentalGuest(vars.id, mapGuestFormToApi(vars.data), requireRentalsAuthHeaders(auth)),
    onSuccess,
    onError: onError('rentals.guests.updateFailed', 'Failed to update guest'),
  });
  const create = useMutation({
    mutationFn: (data: CreateGuestRequest) =>
      createRentalGuest(
        {
          ...mapGuestFormToApi(data),
          booking_id: data.bookingId,
          first_name: data.firstName,
          last_name: data.lastName,
          is_primary: data.isPrimary,
        },
        requireRentalsAuthHeaders(auth)
      ),
    onSuccess,
    onError: onError('rentals.guests.createFailed', 'Failed to add guest'),
  });
  const remove = useMutation({
    mutationFn: (id: string) => deleteRentalGuest(id, requireRentalsAuthHeaders(auth)),
    onSuccess,
    onError: onError('rentals.guests.deleteFailed', 'Failed to delete guest'),
  });

  return { register, update, create, remove };
}

/** Route wrapper for a single booking detail. */
function BookingDetailPageRoute() {
  const { bookingId } = useParams<{ bookingId: string }>();
  const navigate = useNavigate();
  const auth = useRentalsAuth();
  const { t } = useTranslation();
  const updateStatus = useBookingStatusMutation(auth);
  const guestMutations = useGuestMutations(auth);

  const { data, isLoading, isError } = useQuery({
    queryKey: ['rentals', 'booking', bookingId, auth?.xTenantId],
    queryFn: () => getRentalBookingWithGuests(bookingId!, requireRentalsAuthHeaders(auth)),
    enabled: !!auth && !!bookingId,
  });

  if (!bookingId || isError) {
    return <div>{t('errors.bookingNotFound', 'Booking not found')}</div>;
  }

  if (isLoading || !data) {
    return (
      <div className="flex items-center justify-center py-12">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
      </div>
    );
  }

  const booking: BookingWithGuests = {
    ...mapBookingRecordToUi(data.booking),
    guests: data.guests.map(mapGuestRecordToUi),
  };

  return (
    <BookingDetailPage
      booking={booking}
      isLoading={isLoading}
      onBack={() => navigate('/rentals/bookings')}
      onEdit={() => {}}
      onCancel={(id) => updateStatus.mutate({ id, status: 'cancelled' })}
      onCheckIn={(id) => updateStatus.mutate({ id, status: 'checked_in' })}
      onCheckOut={(id) => updateStatus.mutate({ id, status: 'checked_out' })}
      onAddGuest={() => navigate('/rentals/guests')}
      onEditGuest={() => navigate('/rentals/guests')}
      onRegisterGuest={(guestId) => guestMutations.register.mutate([guestId])}
      onDeleteGuest={(guestId) => guestMutations.remove.mutate(guestId)}
    />
  );
}

/**
 * Load the building + unit picker model for the rentals pages:
 * `GET /buildings?organization_id=…` then `GET /buildings/{id}/units` per
 * building. A building whose units fail to load is kept with no units rather
 * than failing the whole picker.
 */
export async function loadRentalPropertyOptions(
  organizationId: string,
  headers: RentalsRequestHeaders
): Promise<{ buildings: RentalBuildingOption[]; units: RentalUnitOption[] }> {
  const page = await listRentalBuildings(organizationId, headers);
  const results = await Promise.allSettled(
    page.buildings.map((b) => listRentalBuildingUnits(b.id, headers))
  );
  const unitsByBuilding: Record<
    string,
    Awaited<ReturnType<typeof listRentalBuildingUnits>>['units']
  > = {};
  results.forEach((r, i) => {
    if (r.status === 'fulfilled') unitsByBuilding[page.buildings[i].id] = r.value.units;
  });
  return buildPropertyOptions(page.buildings, unitsByBuilding);
}

/** Shared building/unit picker query (cached for 5 min across rentals pages). */
function useRentalPropertyOptions(auth: RentalsAuth | null) {
  return useQuery({
    queryKey: ['rentals', 'properties', auth?.xTenantId],
    queryFn: () =>
      loadRentalPropertyOptions(auth?.xTenantId ?? '', requireRentalsAuthHeaders(auth)),
    enabled: !!auth,
    staleTime: 5 * 60_000,
  });
}

/** Route wrapper for the unit availability calendar. */
function RentalsCalendarPageRoute() {
  const navigate = useNavigate();
  const auth = useRentalsAuth();
  const { logout } = useAuth();
  const { showToast } = useToast();
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [searchParams, setSearchParams] = useSearchParams();
  const [month, setMonth] = useState(() => new Date());

  const properties = useRentalPropertyOptions(auth);
  const units = properties.data?.units ?? [];
  // `?unit=` keeps the selection shareable; a single unit is picked automatically.
  const selectedUnitId = searchParams.get('unit') ?? (units.length === 1 ? units[0].id : '');
  const range = calendarFetchRange(month);

  const calendar = useQuery({
    queryKey: [
      'rentals',
      'calendar',
      auth?.xTenantId,
      selectedUnitId,
      range.startDate,
      range.endDate,
    ],
    queryFn: () =>
      getRentalCalendar(
        selectedUnitId,
        { start_date: range.startDate, end_date: range.endDate },
        requireRentalsAuthHeaders(auth)
      ),
    enabled: !!auth && !!selectedUnitId,
  });
  const events = (calendar.data ?? []).map(mapCalendarEventToUi);

  const onMutationError = (titleKey: string) => (error: unknown) =>
    handleRentalsMutationError(error, {
      logout,
      showToast,
      title: t(titleKey),
      fallbackMessage: t('common.pleaseTryAgain'),
    });
  const invalidateCalendar = () =>
    queryClient.invalidateQueries({ queryKey: ['rentals', 'calendar'] });

  const createBlock = useMutation({
    mutationFn: async (input: CalendarBlockInput) => {
      const headers = requireRentalsAuthHeaders(auth);
      // Blocks are half-open on the server: the end is the first free day.
      const blockEnd = addDaysIso(input.endDate, 1);
      const availability = await checkRentalAvailability(
        input.unitId,
        { start_date: input.startDate, end_date: blockEnd },
        headers
      );
      if (!availability.available) throw new Error(t('rentals.calendar.overlap'));
      return createRentalCalendarBlock(
        {
          unit_id: input.unitId,
          block_start: input.startDate,
          block_end: blockEnd,
          reason: input.reason,
          notes: input.notes ?? null,
        },
        headers
      );
    },
    onSuccess: () => {
      showToast({ type: 'success', title: t('rentals.calendar.blockCreated') });
      return invalidateCalendar();
    },
    onError: onMutationError('rentals.calendar.blockCreateFailed'),
  });

  const deleteBlock = useMutation({
    mutationFn: (id: string) => deleteRentalCalendarBlock(id, requireRentalsAuthHeaders(auth)),
    onSuccess: () => {
      showToast({ type: 'success', title: t('rentals.calendar.blockRemoved') });
      return invalidateCalendar();
    },
    onError: onMutationError('rentals.calendar.blockRemoveFailed'),
  });

  // Calendar events carry the block id, not the booking id: look the booking up
  // by unit + exact stay dates, falling back to the bookings list.
  const openBooking = async (event: RentalCalendarEvent) => {
    try {
      const page = await queryClient.fetchQuery({
        queryKey: ['rentals', 'bookings', auth?.xTenantId, 'calendar-lookup', event.id],
        queryFn: () =>
          listRentalBookings(
            {
              unit_id: event.unitId,
              from_date: event.startDate,
              to_date: event.endDate,
              limit: 20,
            },
            requireRentalsAuthHeaders(auth)
          ),
      });
      const matches = page.bookings.filter(
        (b) => b.check_in === event.startDate && b.check_out === event.endDate
      );
      const match = matches.find((b) => b.guest_name === event.title) ?? matches[0];
      navigate(match ? `/rentals/bookings/${match.id}` : '/rentals/bookings');
    } catch (error) {
      if (!handleRentalsAuthError(error, logout)) navigate('/rentals/bookings');
    }
  };

  return (
    <RentalsCalendarPage
      units={units}
      isLoadingUnits={properties.isLoading}
      isUnitsError={properties.isError}
      events={events}
      selectedUnitId={selectedUnitId}
      month={month}
      isLoading={calendar.isLoading && !!selectedUnitId}
      isError={calendar.isError}
      isSavingBlock={createBlock.isPending}
      onRetry={() => {
        if (properties.isError) properties.refetch();
        else calendar.refetch();
      }}
      onUnitChange={(unitId) => setSearchParams(unitId ? { unit: unitId } : {}, { replace: true })}
      onMonthChange={setMonth}
      onEventClick={openBooking}
      onAddBlock={(input) => createBlock.mutate(input)}
      onDeleteBlock={(id) => deleteBlock.mutate(id)}
      onBack={() => navigate('/rentals')}
    />
  );
}

/** Route wrapper for guest registration. */
function GuestRegistrationPageRoute() {
  const navigate = useNavigate();
  const auth = useRentalsAuth();
  const guestMutations = useGuestMutations(auth);

  const { data, isLoading } = useQuery({
    queryKey: ['rentals', 'guests', auth?.xTenantId],
    queryFn: () => listRentalGuests({}, requireRentalsAuthHeaders(auth)),
    enabled: !!auth,
  });

  const bookingsWithPendingGuests = groupGuestsByBooking(data ?? []);

  return (
    <GuestRegistrationPage
      bookingsWithPendingGuests={bookingsWithPendingGuests}
      isLoading={isLoading}
      onRegisterGuest={(guestId) => guestMutations.register.mutate([guestId])}
      onRegisterAllGuests={(bookingId) => {
        const booking = bookingsWithPendingGuests.find((b) => b.id === bookingId);
        const pendingIds = (booking?.guests ?? [])
          .filter((g) => g.registrationStatus === 'pending')
          .map((g) => g.id);
        if (pendingIds.length > 0) guestMutations.register.mutate(pendingIds);
      }}
      onEditGuest={(id, guestData) => guestMutations.update.mutate({ id, data: guestData })}
      onAddGuest={(guestData) => guestMutations.create.mutate(guestData)}
      onBack={() => navigate('/rentals')}
    />
  );
}

/** Route wrapper for the daňový výkaz (authority guest report). */
function TaxReportPageRoute() {
  const navigate = useNavigate();
  const auth = useRentalsAuth();
  const { logout } = useAuth();
  const { showToast } = useToast();
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  const properties = useRentalPropertyOptions(auth);
  const reports = useQuery({
    queryKey: ['rentals', 'reports', auth?.xTenantId],
    queryFn: () => listRentalReports({ limit: 50 }, requireRentalsAuthHeaders(auth)),
    enabled: !!auth,
  });

  const submit = useMutation({
    mutationFn: (id: string) => submitRentalReport(id, requireRentalsAuthHeaders(auth)),
    onSuccess: () => {
      showToast({ type: 'success', title: t('rentals.tax.submittedToast') });
      return queryClient.invalidateQueries({ queryKey: ['rentals', 'reports'] });
    },
    onError: (error) =>
      handleRentalsMutationError(error, {
        logout,
        showToast,
        title: t('rentals.tax.errors.submitFailed'),
        fallbackMessage: t('common.pleaseTryAgain'),
      }),
  });

  /** Session loss → login redirect; any other error goes back to the page. */
  const withAuthHandling = async <T,>(run: () => Promise<T>): Promise<T> => {
    try {
      return await run();
    } catch (error) {
      handleRentalsAuthError(error, logout);
      throw error;
    }
  };

  return (
    <TaxReportPage
      buildings={properties.data?.buildings ?? []}
      isLoadingBuildings={properties.isLoading}
      isBuildingsError={properties.isError}
      reports={(reports.data ?? []).map(mapReportSummaryToUi)}
      isLoadingReports={reports.isLoading}
      submittingReportId={submit.isPending ? submit.variables : undefined}
      onPreview={(params: GuestReportParams) =>
        withAuthHandling(async () =>
          mapReportPreviewToUi(
            await previewRentalReport(
              {
                building_id: params.buildingId,
                period_start: params.periodStart,
                period_end: params.periodEnd,
              },
              requireRentalsAuthHeaders(auth)
            )
          )
        )
      }
      onSaveReport={(params: GuestReportParams) =>
        withAuthHandling(async () => {
          await createRentalReport(
            {
              building_id: params.buildingId,
              report_type: params.periodType satisfies RentalReportType,
              period_start: params.periodStart,
              period_end: params.periodEnd,
              authority_code: params.authorityCode,
            },
            requireRentalsAuthHeaders(auth)
          );
          await queryClient.invalidateQueries({ queryKey: ['rentals', 'reports'] });
        })
      }
      onSubmitReport={(id) => submit.mutate(id)}
      onBack={() => navigate('/rentals')}
    />
  );
}

/** Short-Term Rentals routes (Epic 18, UC-29/30). */
export function rentalRoutes() {
  return (
    <>
      <Route
        path="/rentals"
        element={
          <ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>
            <RentalsDashboardPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/rentals/connections"
        element={
          <ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>
            <PlatformConnectionsPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/rentals/bookings"
        element={
          <ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>
            <BookingsPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/rentals/bookings/:bookingId"
        element={
          <ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>
            <BookingDetailPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/rentals/calendar"
        element={
          <ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>
            <RentalsCalendarPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/rentals/guests"
        element={
          <ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>
            <GuestRegistrationPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/rentals/reports"
        element={
          <ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>
            <TaxReportPageRoute />
          </ProtectedRoute>
        }
      />
    </>
  );
}
