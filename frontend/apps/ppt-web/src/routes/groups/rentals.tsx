/**
 * Short-Term Rentals route group (Epic 18, UC-29/30).
 *
 * Owns the rentals API↔UI mappers, the `useRentalsAuth` helper, the lazy page
 * imports, the route-wrapper components, and the `<Route>` table fragment.
 * Extracted from App.tsx to isolate rentals work.
 */
import type {
  RentalsGuestRegistration,
  RentalsPlatformConnection,
  RentalsRentalPlatform,
  RentalsReservation,
  RentalsSyncStatus,
} from '@ppt/api-client';
import {
  getToken,
  rentalsApiCheckIn,
  rentalsApiCheckOut,
  rentalsApiCreateConnection,
  rentalsApiGetReservation,
  rentalsApiListConnections,
  rentalsApiListGuests,
  rentalsApiListReservations,
  rentalsApiSyncPlatforms,
} from '@ppt/api-client';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { lazy, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Route, useNavigate, useParams } from 'react-router-dom';
import { ProtectedRoute } from '../../components';
import { AuthError, type AuthErrorCode, useAuth } from '../../contexts';
import type {
  BookingListParams,
  BookingWithGuests,
  RentalBooking,
  BookingStatus as RentalBookingStatus,
  CalendarEvent as RentalCalendarEvent,
  RentalGuest,
  PlatformConnection as RentalPlatformConnection,
  PlatformType as RentalPlatformType,
  RentalStatistics,
  TaxReportData,
} from '../../features/rentals/types';

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
 * Map a generated RentalsRentalPlatform → the page-level BookingSource.
 * The generated platform enum carries 'vrbo' which the page lacks; fold it to 'other'.
 */
function mapRentalPlatformToSource(platform: RentalsRentalPlatform): RentalBooking['source'] {
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

/** Map a generated RentalsRentalPlatform → the page PlatformType (airbnb | booking). */
function mapRentalPlatformToType(platform: RentalsRentalPlatform): RentalPlatformType {
  return platform === 'booking' ? 'booking' : 'airbnb';
}

/** Map a generated RentalsSyncStatus → the page ConnectionStatus. */
function mapSyncStatusToConnectionStatus(
  syncStatus: RentalsSyncStatus,
  isActive: boolean
): RentalPlatformConnection['status'] {
  if (!isActive) return 'disconnected';
  switch (syncStatus) {
    case 'synced':
      return 'connected';
    case 'pending':
      return 'pending';
    case 'error':
      return 'error';
    default:
      return 'disconnected';
  }
}

/** Map a generated RentalsReservation → the page RentalBooking. */
function mapReservationToBooking(res: RentalsReservation): RentalBooking {
  return {
    id: res.id,
    unitId: res.unitId,
    // The reservation payload does not carry building context; left blank.
    buildingId: '',
    guestName: res.guestName,
    guestEmail: res.guestEmail,
    guestPhone: res.guestPhone,
    checkIn: res.checkIn,
    checkOut: res.checkOut,
    status: res.status as RentalBookingStatus,
    source: mapRentalPlatformToSource(res.platform),
    platformBookingId: res.externalId,
    totalPrice: res.totalPrice?.amount,
    currency: res.totalPrice?.currency,
    guestCount: res.guestCount,
    notes: res.notes,
    createdAt: res.createdAt,
    updatedAt: res.updatedAt,
  };
}

/** Map a generated RentalsPlatformConnection → the page PlatformConnection. */
function mapApiConnectionToUi(conn: RentalsPlatformConnection): RentalPlatformConnection {
  return {
    id: conn.id,
    unitId: conn.unitId,
    platform: mapRentalPlatformToType(conn.platform),
    status: mapSyncStatusToConnectionStatus(conn.syncStatus, conn.isActive),
    platformPropertyId: conn.externalListingId,
    lastSyncAt: conn.lastSyncAt,
    isAutoSyncEnabled: conn.isActive,
    createdAt: conn.createdAt,
  };
}

/** Map a generated RentalsGuestRegistration → the page RentalGuest. */
function mapApiGuestToUi(guest: RentalsGuestRegistration): RentalGuest {
  return {
    id: guest.id,
    bookingId: guest.reservationId ?? '',
    firstName: guest.firstName,
    lastName: guest.lastName,
    dateOfBirth: guest.dateOfBirth,
    nationality: guest.nationality,
    documentType: guest.documentType,
    documentNumber: guest.documentNumber,
    documentExpiry: guest.documentExpiry,
    registrationStatus: guest.submittedToAuthorities ? 'registered' : 'pending',
    registeredAt: guest.submittedAt,
    isPrimary: false,
    createdAt: guest.createdAt,
  };
}

/**
 * Build the auth header + tenant id required by the generated
 * ShortTermRentalsService methods. Returns null when either is missing so
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
 * Build the auth headers the generated ShortTermRentalsService methods require,
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

/** Route wrapper for the rentals dashboard. */
function RentalsDashboardPageRoute() {
  const navigate = useNavigate();
  const auth = useRentalsAuth();

  const { data, isLoading } = useQuery({
    queryKey: ['rentals', 'reservations', auth?.xTenantId],
    queryFn: () =>
      rentalsApiListReservations({
        headers: requireRentalsAuthHeaders(auth),
      }),
    enabled: !!auth,
  });
  const { data: connData } = useQuery({
    queryKey: ['rentals', 'connections', auth?.xTenantId],
    queryFn: () =>
      rentalsApiListConnections({
        headers: requireRentalsAuthHeaders(auth),
      }),
    enabled: !!auth,
  });

  const bookings = (data?.data?.data ?? []).map(mapReservationToBooking);
  const connections = (connData?.data ?? []).map(mapApiConnectionToUi);
  const now = Date.now();
  const upcomingBookings = bookings.filter((b) => new Date(b.checkIn).getTime() >= now);
  const activeBookings = bookings.filter((b) => b.status === 'checked_in');

  const statistics: RentalStatistics = {
    totalBookings: bookings.length,
    activeBookings: activeBookings.length,
    upcomingBookings: upcomingBookings.length,
    totalRevenue: bookings.reduce((sum, b) => sum + (b.totalPrice ?? 0), 0),
    occupancyRate: 0,
    averageStayDuration: 0,
    pendingGuestRegistrations: 0,
    currency: bookings[0]?.currency ?? 'EUR',
  };

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
  const { logout } = useAuth();
  const queryClient = useQueryClient();

  const { data, isLoading } = useQuery({
    queryKey: ['rentals', 'connections', auth?.xTenantId],
    queryFn: () =>
      rentalsApiListConnections({
        headers: requireRentalsAuthHeaders(auth),
      }),
    enabled: !!auth,
  });

  const createConnection = useMutation({
    mutationFn: (vars: { unitId: string; platform: RentalPlatformType }) =>
      rentalsApiCreateConnection({
        headers: requireRentalsAuthHeaders(auth),
        body: { unitId: vars.unitId, platform: vars.platform as RentalsRentalPlatform },
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['rentals', 'connections'] });
    },
    onError: (error) => handleRentalsAuthError(error, logout),
  });

  const syncPlatforms = useMutation({
    mutationFn: (unitId: string) =>
      rentalsApiSyncPlatforms({
        headers: requireRentalsAuthHeaders(auth),
        query: { unitId },
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['rentals', 'connections'] });
    },
    onError: (error) => handleRentalsAuthError(error, logout),
  });

  const connections = (data?.data ?? []).map(mapApiConnectionToUi);

  return (
    <PlatformConnectionsPage
      connections={connections}
      units={[]}
      isLoading={isLoading}
      onConnect={() => {}}
      onDisconnect={() => {}}
      onSync={(connectionId) => {
        const conn = connections.find((c) => c.id === connectionId);
        if (conn) syncPlatforms.mutate(conn.unitId);
      }}
      onSettings={() => {}}
      onCreateConnection={(unitId, platform) => createConnection.mutate({ unitId, platform })}
      onBack={() => navigate('/rentals')}
    />
  );
}

/** Route wrapper for the bookings list. */
function BookingsPageRoute() {
  const navigate = useNavigate();
  const auth = useRentalsAuth();
  const [filters, setFilters] = useState<BookingListParams>({});

  const { data, isLoading } = useQuery({
    queryKey: ['rentals', 'reservations', auth?.xTenantId, filters],
    queryFn: () =>
      rentalsApiListReservations({
        headers: requireRentalsAuthHeaders(auth),
        query: {
          unitId: filters.unitId,
          status: filters.status as RentalsReservation['status'],
          from: filters.fromDate,
          to: filters.toDate,
          page: filters.page,
          limit: filters.limit,
        },
      }),
    enabled: !!auth,
  });

  const bookings = (data?.data?.data ?? []).map(mapReservationToBooking);

  return (
    <BookingsPage
      bookings={bookings}
      total={data?.data?.pagination?.totalItems ?? bookings.length}
      buildings={[]}
      units={[]}
      isLoading={isLoading}
      onFilterChange={setFilters}
      onViewBooking={(id) => navigate(`/rentals/bookings/${id}`)}
      onEditBooking={(id) => navigate(`/rentals/bookings/${id}`)}
      onCancelBooking={() => {}}
      onCheckIn={() => {}}
      onCheckOut={() => {}}
      onCreateBooking={() => navigate('/rentals/bookings')}
      onBack={() => navigate('/rentals')}
    />
  );
}

/** Route wrapper for a single booking detail. */
function BookingDetailPageRoute() {
  const { bookingId } = useParams<{ bookingId: string }>();
  const navigate = useNavigate();
  const auth = useRentalsAuth();
  const { logout } = useAuth();
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  const { data, isLoading } = useQuery({
    queryKey: ['rentals', 'reservation', bookingId, auth?.xTenantId],
    queryFn: () =>
      rentalsApiGetReservation({
        headers: requireRentalsAuthHeaders(auth),
        path: { id: bookingId! },
      }),
    enabled: !!auth && !!bookingId,
  });

  const checkIn = useMutation({
    mutationFn: () =>
      rentalsApiCheckIn({
        headers: requireRentalsAuthHeaders(auth),
        path: { id: bookingId! },
      }),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: ['rentals', 'reservation', bookingId] }),
    onError: (error) => handleRentalsAuthError(error, logout),
  });
  const checkOut = useMutation({
    mutationFn: () =>
      rentalsApiCheckOut({
        headers: requireRentalsAuthHeaders(auth),
        path: { id: bookingId! },
      }),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: ['rentals', 'reservation', bookingId] }),
    onError: (error) => handleRentalsAuthError(error, logout),
  });

  if (!bookingId) {
    return <div>{t('errors.bookingNotFound', 'Booking not found')}</div>;
  }

  if (isLoading || !data || !data.data) {
    return (
      <div className="flex items-center justify-center py-12">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
      </div>
    );
  }

  const booking: BookingWithGuests = { ...mapReservationToBooking(data.data), guests: [] };

  return (
    <BookingDetailPage
      booking={booking}
      isLoading={isLoading}
      onBack={() => navigate('/rentals/bookings')}
      onEdit={() => {}}
      onCancel={() => {}}
      onCheckIn={() => checkIn.mutate()}
      onCheckOut={() => checkOut.mutate()}
      onAddGuest={() => navigate('/rentals/guests')}
      onEditGuest={() => navigate('/rentals/guests')}
      onRegisterGuest={() => navigate('/rentals/guests')}
      onDeleteGuest={() => {}}
    />
  );
}

/** Route wrapper for the calendar view. No calendar endpoint exists yet; events stay empty. */
function RentalsCalendarPageRoute() {
  const navigate = useNavigate();

  return (
    <RentalsCalendarPage
      units={[]}
      events={[] as RentalCalendarEvent[]}
      isLoading={false}
      onUnitChange={() => {}}
      onMonthChange={() => {}}
      onEventClick={() => {}}
      onAddBlock={() => {}}
      onBack={() => navigate('/rentals')}
    />
  );
}

/** Route wrapper for guest registration. */
function GuestRegistrationPageRoute() {
  const navigate = useNavigate();
  const auth = useRentalsAuth();

  const { data, isLoading } = useQuery({
    queryKey: ['rentals', 'guests', auth?.xTenantId],
    queryFn: () =>
      rentalsApiListGuests({
        headers: requireRentalsAuthHeaders(auth),
      }),
    enabled: !!auth,
  });

  // Group registered guests by reservation so the page can list per-booking
  // pending registrations. Without per-booking metadata we surface a single
  // synthetic grouping keyed by reservationId.
  const guests = (data?.data?.data ?? []).map(mapApiGuestToUi);
  const byBooking = new Map<string, RentalGuest[]>();
  for (const g of guests) {
    const list = byBooking.get(g.bookingId) ?? [];
    list.push(g);
    byBooking.set(g.bookingId, list);
  }
  const bookingsWithPendingGuests = Array.from(byBooking.entries()).map(
    ([bookingId, bookingGuests]) => ({
      id: bookingId,
      guestName: bookingGuests[0]
        ? `${bookingGuests[0].firstName} ${bookingGuests[0].lastName}`
        : '',
      unitName: '',
      checkIn: '',
      checkOut: '',
      guests: bookingGuests,
    })
  );

  return (
    <GuestRegistrationPage
      bookingsWithPendingGuests={bookingsWithPendingGuests}
      isLoading={isLoading}
      onRegisterGuest={() => {}}
      onRegisterAllGuests={() => {}}
      onEditGuest={() => {}}
      onAddGuest={() => {}}
      onBack={() => navigate('/rentals')}
    />
  );
}

/** Route wrapper for tax reports. No tax-report endpoint exists yet; returns an empty report. */
function TaxReportPageRoute() {
  const navigate = useNavigate();

  return (
    <TaxReportPage
      buildings={[]}
      isLoading={false}
      onGenerateReport={async (params) =>
        ({
          year: params.year,
          jurisdiction: {
            country: params.jurisdiction,
            name: params.jurisdiction,
            defaultTaxRate: 0,
            requiresGuestRegistration: false,
          },
          reportType: params.reportType,
          generatedAt: new Date().toISOString(),
          currency: 'EUR',
          summary: {
            totalIncome: 0,
            totalBookings: 0,
            totalNightsOccupied: 0,
            averageOccupancyRate: 0,
            totalExpenses: 0,
            netProfit: 0,
            estimatedTax: 0,
            effectiveTaxRate: 0,
          },
          expensesByCategory: [],
          propertiesCovered: [],
        }) satisfies TaxReportData
      }
      onExportReport={async () => {}}
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
          <ProtectedRoute>
            <RentalsDashboardPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/rentals/connections"
        element={
          <ProtectedRoute>
            <PlatformConnectionsPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/rentals/bookings"
        element={
          <ProtectedRoute>
            <BookingsPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/rentals/bookings/:bookingId"
        element={
          <ProtectedRoute>
            <BookingDetailPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/rentals/calendar"
        element={
          <ProtectedRoute>
            <RentalsCalendarPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/rentals/guests"
        element={
          <ProtectedRoute>
            <GuestRegistrationPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/rentals/reports"
        element={
          <ProtectedRoute>
            <TaxReportPageRoute />
          </ProtectedRoute>
        }
      />
    </>
  );
}
