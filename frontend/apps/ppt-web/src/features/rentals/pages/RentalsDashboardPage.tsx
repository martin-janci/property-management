/**
 * RentalsDashboardPage - Overview with stats and upcoming bookings.
 * Epic 18: Short-Term Rental Integration
 */

import { useTranslation } from 'react-i18next';
import { BookingCard } from '../components/BookingCard';
import { TaxSummaryCard } from '../components/TaxSummaryCard';
import type { PlatformConnection, RentalBooking, RentalStatistics, TaxSummary } from '../types';

interface RentalsDashboardPageProps {
  statistics: RentalStatistics;
  upcomingBookings: RentalBooking[];
  activeBookings: RentalBooking[];
  platformConnections: PlatformConnection[];
  taxSummary?: TaxSummary;
  isLoading?: boolean;
  /**
   * Load error flag/message. When set, the dashboard renders an error state
   * (with an optional retry) instead of all-zero stats, so a failed fetch is
   * not mistaken for genuinely empty data.
   */
  isError?: boolean;
  error?: string | null;
  onRetry?: () => void;
  isTaxSummaryLoading?: boolean;
  onViewBooking: (id: string) => void;
  onCheckIn: (id: string) => void;
  onCheckOut: (id: string) => void;
  onViewAllBookings: () => void;
  onViewCalendar: () => void;
  onViewConnections: () => void;
  onViewGuestRegistration: () => void;
  onViewTaxReport: () => void;
  onExportTaxReport?: () => void;
}

function formatCurrency(amount: number, currency: string, language: string): string {
  return new Intl.NumberFormat(language, {
    style: 'currency',
    currency,
  }).format(amount);
}

function formatPercentage(value: number, language: string): string {
  return new Intl.NumberFormat(language, {
    style: 'percent',
    maximumFractionDigits: 1,
  }).format(value);
}

export function RentalsDashboardPage({
  statistics,
  upcomingBookings,
  activeBookings,
  platformConnections,
  taxSummary,
  isLoading,
  isError,
  error,
  onRetry,
  isTaxSummaryLoading,
  onViewBooking,
  onCheckIn,
  onCheckOut,
  onViewAllBookings,
  onViewCalendar,
  onViewConnections,
  onViewGuestRegistration,
  onViewTaxReport,
  onExportTaxReport,
}: RentalsDashboardPageProps) {
  const { t, i18n } = useTranslation();
  const connectedPlatforms = platformConnections.filter((c) => c.status === 'connected').length;
  const totalPlatforms = platformConnections.length;

  return (
    <div className="min-h-screen bg-gray-100">
      {/* Header */}
      <div className="bg-white shadow">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6">
          <div className="flex items-center justify-between">
            <div>
              <h1 className="text-2xl font-bold text-gray-900">{t('rentals.dashboard.title')}</h1>
              <p className="mt-1 text-sm text-gray-500">{t('rentals.dashboard.subtitle')}</p>
            </div>
            <div className="flex items-center gap-3">
              <button
                type="button"
                onClick={onViewTaxReport}
                className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50"
              >
                {t('rentals.dashboard.taxReport')}
              </button>
              <button
                type="button"
                onClick={onViewCalendar}
                className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50"
              >
                {t('rentals.dashboard.viewCalendar')}
              </button>
              <button
                type="button"
                onClick={onViewAllBookings}
                className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700"
              >
                {t('rentals.dashboard.viewAllBookings')}
              </button>
            </div>
          </div>
        </div>
      </div>

      {/* Main Content */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {isLoading ? (
          <div className="flex items-center justify-center h-64">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
          </div>
        ) : isError || error ? (
          <div className="bg-white rounded-lg shadow p-8 text-center" role="alert">
            <p className="text-red-700">{t('rentals.dashboard.loadFailed')}</p>
            {error && <p className="mt-1 text-sm text-gray-500">{error}</p>}
            {onRetry && (
              <button
                type="button"
                onClick={onRetry}
                className="mt-4 px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50"
              >
                {t('rentals.dashboard.retry')}
              </button>
            )}
          </div>
        ) : (
          <>
            {/* Statistics Cards */}
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8">
              <div className="bg-white rounded-lg shadow p-6">
                <div className="flex items-center justify-between">
                  <div>
                    <p className="text-sm font-medium text-gray-500">
                      {t('rentals.dashboard.totalBookings')}
                    </p>
                    <p className="mt-2 text-3xl font-bold text-gray-900">
                      {statistics.totalBookings}
                    </p>
                  </div>
                  <div className="p-3 bg-blue-100 rounded-full">
                    <svg
                      className="w-6 h-6 text-blue-600"
                      fill="none"
                      viewBox="0 0 24 24"
                      stroke="currentColor"
                      aria-hidden="true"
                    >
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M8 7V3m8 4V3m-9 8h10M5 21h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"
                      />
                    </svg>
                  </div>
                </div>
                <p className="mt-2 text-sm text-gray-500">
                  {statistics.activeBookings} {t('rentals.dashboard.active')},{' '}
                  {statistics.upcomingBookings} {t('rentals.dashboard.upcoming')}
                </p>
              </div>

              <div className="bg-white rounded-lg shadow p-6">
                <div className="flex items-center justify-between">
                  <div>
                    <p className="text-sm font-medium text-gray-500">
                      {t('rentals.dashboard.totalRevenue')}
                    </p>
                    <p className="mt-2 text-3xl font-bold text-gray-900">
                      {formatCurrency(statistics.totalRevenue, statistics.currency, i18n.language)}
                    </p>
                  </div>
                  <div className="p-3 bg-green-100 rounded-full">
                    <svg
                      className="w-6 h-6 text-green-600"
                      fill="none"
                      viewBox="0 0 24 24"
                      stroke="currentColor"
                      aria-hidden="true"
                    >
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 8V7m0 1v8m0 0v1m0-1c-1.11 0-2.08-.402-2.599-1M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                      />
                    </svg>
                  </div>
                </div>
                <p className="mt-2 text-sm text-gray-500">{t('rentals.dashboard.thisMonth')}</p>
              </div>

              <div className="bg-white rounded-lg shadow p-6">
                <div className="flex items-center justify-between">
                  <div>
                    <p className="text-sm font-medium text-gray-500">
                      {t('rentals.dashboard.occupancyRate')}
                    </p>
                    <p className="mt-2 text-3xl font-bold text-gray-900">
                      {formatPercentage(statistics.occupancyRate, i18n.language)}
                    </p>
                  </div>
                  <div className="p-3 bg-purple-100 rounded-full">
                    <svg
                      className="w-6 h-6 text-purple-600"
                      fill="none"
                      viewBox="0 0 24 24"
                      stroke="currentColor"
                      aria-hidden="true"
                    >
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"
                      />
                    </svg>
                  </div>
                </div>
                <p className="mt-2 text-sm text-gray-500">
                  {t('rentals.dashboard.avgStay')}: {statistics.averageStayDuration.toFixed(1)}{' '}
                  {t('rentals.nights')}
                </p>
              </div>

              <div className="bg-white rounded-lg shadow p-6">
                <div className="flex items-center justify-between">
                  <div>
                    <p className="text-sm font-medium text-gray-500">
                      {t('rentals.dashboard.guestRegistrations')}
                    </p>
                    <p className="mt-2 text-3xl font-bold text-gray-900">
                      {statistics.pendingGuestRegistrations}
                    </p>
                  </div>
                  <div className="p-3 bg-orange-100 rounded-full">
                    <svg
                      className="w-6 h-6 text-orange-600"
                      fill="none"
                      viewBox="0 0 24 24"
                      stroke="currentColor"
                      aria-hidden="true"
                    >
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0z"
                      />
                    </svg>
                  </div>
                </div>
                <button
                  type="button"
                  onClick={onViewGuestRegistration}
                  className="mt-2 text-sm text-orange-600 hover:text-orange-800"
                >
                  {t('rentals.dashboard.viewPending')}
                </button>
              </div>
            </div>

            {/* Tax Summary Widget */}
            {taxSummary && (
              <div className="mb-8">
                <TaxSummaryCard
                  summary={taxSummary}
                  onExport={onExportTaxReport}
                  onViewFullReport={onViewTaxReport}
                  isLoading={isTaxSummaryLoading}
                />
              </div>
            )}

            {/* Quick Actions */}
            <div className="grid grid-cols-1 lg:grid-cols-3 gap-6 mb-8">
              {/* Platform Connections Summary */}
              <div className="bg-white rounded-lg shadow p-6">
                <div className="flex items-center justify-between mb-4">
                  <h2 className="text-lg font-semibold text-gray-900">
                    {t('rentals.dashboard.platformConnections')}
                  </h2>
                  <button
                    type="button"
                    onClick={onViewConnections}
                    className="text-sm text-blue-600 hover:text-blue-800"
                  >
                    {t('rentals.dashboard.manage')}
                  </button>
                </div>
                <div className="space-y-3">
                  {platformConnections.length === 0 ? (
                    <p className="text-sm text-gray-500">{t('rentals.dashboard.noConnections')}</p>
                  ) : (
                    <>
                      <div className="flex items-center justify-between text-sm">
                        <span className="text-gray-600">{t('rentals.dashboard.connected')}</span>
                        <span className="font-medium text-green-600">
                          {connectedPlatforms} / {totalPlatforms}
                        </span>
                      </div>
                      {platformConnections.slice(0, 3).map((connection) => (
                        <div
                          key={connection.id}
                          className="flex items-center justify-between py-2 border-t"
                        >
                          <span className="text-sm font-medium text-gray-700">
                            {connection.platform === 'airbnb' ? 'Airbnb' : 'Booking.com'}
                          </span>
                          <span
                            className={`text-xs px-2 py-1 rounded ${
                              connection.status === 'connected'
                                ? 'bg-green-100 text-green-700'
                                : connection.status === 'error'
                                  ? 'bg-red-100 text-red-700'
                                  : 'bg-gray-100 text-gray-700'
                            }`}
                          >
                            {t(`rentals.connection.status.${connection.status}`)}
                          </span>
                        </div>
                      ))}
                    </>
                  )}
                </div>
              </div>

              {/* Calendar Preview */}
              <div className="bg-white rounded-lg shadow p-6">
                <div className="flex items-center justify-between mb-4">
                  <h2 className="text-lg font-semibold text-gray-900">
                    {t('rentals.dashboard.todaysActivity')}
                  </h2>
                  <button
                    type="button"
                    onClick={onViewCalendar}
                    className="text-sm text-blue-600 hover:text-blue-800"
                  >
                    {t('rentals.dashboard.fullCalendar')}
                  </button>
                </div>
                <div className="space-y-3">
                  <div className="flex items-center gap-3 p-3 bg-green-50 rounded-lg">
                    <div className="w-8 h-8 bg-green-100 rounded-full flex items-center justify-center">
                      <svg
                        className="w-4 h-4 text-green-600"
                        fill="none"
                        viewBox="0 0 24 24"
                        stroke="currentColor"
                        aria-hidden="true"
                      >
                        <path
                          strokeLinecap="round"
                          strokeLinejoin="round"
                          strokeWidth={2}
                          d="M5 13l4 4L19 7"
                        />
                      </svg>
                    </div>
                    <div>
                      <p className="text-sm font-medium text-green-900">
                        {t('rentals.dashboard.checkInsToday')}
                      </p>
                      <p className="text-2xl font-bold text-green-700">
                        {
                          upcomingBookings.filter(
                            (b) =>
                              b.status === 'confirmed' &&
                              new Date(b.checkIn).toDateString() === new Date().toDateString()
                          ).length
                        }
                      </p>
                    </div>
                  </div>
                  <div className="flex items-center gap-3 p-3 bg-blue-50 rounded-lg">
                    <div className="w-8 h-8 bg-blue-100 rounded-full flex items-center justify-center">
                      <svg
                        className="w-4 h-4 text-blue-600"
                        fill="none"
                        viewBox="0 0 24 24"
                        stroke="currentColor"
                        aria-hidden="true"
                      >
                        <path
                          strokeLinecap="round"
                          strokeLinejoin="round"
                          strokeWidth={2}
                          d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1"
                        />
                      </svg>
                    </div>
                    <div>
                      <p className="text-sm font-medium text-blue-900">
                        {t('rentals.dashboard.checkOutsToday')}
                      </p>
                      <p className="text-2xl font-bold text-blue-700">
                        {
                          activeBookings.filter(
                            (b) =>
                              b.status === 'checked_in' &&
                              new Date(b.checkOut).toDateString() === new Date().toDateString()
                          ).length
                        }
                      </p>
                    </div>
                  </div>
                </div>
              </div>

              {/* Guest Registration Alert */}
              {statistics.pendingGuestRegistrations > 0 && (
                <div className="bg-orange-50 border border-orange-200 rounded-lg shadow p-6">
                  <div className="flex items-start gap-3">
                    <div className="w-10 h-10 bg-orange-100 rounded-full flex items-center justify-center flex-shrink-0">
                      <svg
                        className="w-5 h-5 text-orange-600"
                        fill="currentColor"
                        viewBox="0 0 20 20"
                        aria-hidden="true"
                      >
                        <path
                          fillRule="evenodd"
                          d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z"
                          clipRule="evenodd"
                        />
                      </svg>
                    </div>
                    <div>
                      <h3 className="text-lg font-semibold text-orange-900">
                        {t('rentals.dashboard.registrationRequired')}
                      </h3>
                      <p className="mt-1 text-sm text-orange-700">
                        {t('rentals.dashboard.registrationRequiredDesc', {
                          count: statistics.pendingGuestRegistrations,
                        })}
                      </p>
                      <button
                        type="button"
                        onClick={onViewGuestRegistration}
                        className="mt-3 px-4 py-2 text-sm font-medium text-white bg-orange-600 rounded-md hover:bg-orange-700"
                      >
                        {t('rentals.dashboard.registerGuests')}
                      </button>
                    </div>
                  </div>
                </div>
              )}
            </div>

            {/* Active Bookings */}
            {activeBookings.length > 0 && (
              <div className="mb-8">
                <div className="flex items-center justify-between mb-4">
                  <h2 className="text-lg font-semibold text-gray-900">
                    {t('rentals.dashboard.currentGuests')}
                  </h2>
                </div>
                <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                  {activeBookings.map((booking) => (
                    <BookingCard
                      key={booking.id}
                      booking={booking}
                      onView={onViewBooking}
                      onCheckOut={onCheckOut}
                    />
                  ))}
                </div>
              </div>
            )}

            {/* Upcoming Bookings */}
            <div>
              <div className="flex items-center justify-between mb-4">
                <h2 className="text-lg font-semibold text-gray-900">
                  {t('rentals.dashboard.upcomingBookings')}
                </h2>
                <button
                  type="button"
                  onClick={onViewAllBookings}
                  className="text-sm text-blue-600 hover:text-blue-800"
                >
                  {t('rentals.dashboard.viewAll')}
                </button>
              </div>
              {upcomingBookings.length === 0 ? (
                <div className="bg-white rounded-lg shadow p-8 text-center">
                  <svg
                    className="w-12 h-12 mx-auto text-gray-400"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke="currentColor"
                    aria-hidden="true"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M8 7V3m8 4V3m-9 8h10M5 21h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"
                    />
                  </svg>
                  <p className="mt-4 text-gray-500">{t('rentals.dashboard.noUpcomingBookings')}</p>
                </div>
              ) : (
                <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                  {upcomingBookings.slice(0, 6).map((booking) => (
                    <BookingCard
                      key={booking.id}
                      booking={booking}
                      onView={onViewBooking}
                      onCheckIn={onCheckIn}
                    />
                  ))}
                </div>
              )}
            </div>
          </>
        )}
      </div>
    </div>
  );
}
