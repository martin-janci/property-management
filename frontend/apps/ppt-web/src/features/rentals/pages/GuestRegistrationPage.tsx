/**
 * GuestRegistrationPage - Register guests for government reporting.
 * Epic 18: Short-Term Rental Integration (Story 18.3)
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { GuestForm } from '../components/GuestForm';
import { parseIsoDate } from '../mappers';
import type { CreateGuestRequest, RentalGuest, UpdateGuestRequest } from '../types';

interface BookingWithGuests {
  id: string;
  guestName: string;
  unitName: string;
  checkIn: string;
  checkOut: string;
  guests: RentalGuest[];
}

interface GuestRegistrationPageProps {
  bookingsWithPendingGuests: BookingWithGuests[];
  isLoading?: boolean;
  isSubmitting?: boolean;
  onRegisterGuest: (guestId: string) => void;
  onRegisterAllGuests: (bookingId: string) => void;
  onEditGuest: (guestId: string, data: UpdateGuestRequest) => void;
  onAddGuest: (data: CreateGuestRequest) => void;
  onBack: () => void;
}

function formatDateRange(checkIn: string, checkOut: string, language: string): string {
  const fmt = new Intl.DateTimeFormat(language, { month: 'short', day: 'numeric' });
  return `${fmt.format(parseIsoDate(checkIn))} – ${fmt.format(parseIsoDate(checkOut))}`;
}

export function GuestRegistrationPage({
  bookingsWithPendingGuests,
  isLoading,
  isSubmitting,
  onRegisterGuest,
  onRegisterAllGuests,
  onEditGuest,
  onAddGuest,
  onBack,
}: GuestRegistrationPageProps) {
  const { t, i18n } = useTranslation();
  const [expandedBookingId, setExpandedBookingId] = useState<string | null>(null);
  const [editingGuestId, setEditingGuestId] = useState<string | null>(null);
  const [addingToBookingId, setAddingToBookingId] = useState<string | null>(null);

  const totalPendingGuests = bookingsWithPendingGuests.reduce(
    (sum, b) => sum + b.guests.filter((g) => g.registrationStatus === 'pending').length,
    0
  );

  const handleEditSubmit = (guestId: string, data: UpdateGuestRequest) => {
    onEditGuest(guestId, data);
    setEditingGuestId(null);
  };

  const handleAddSubmit = (data: CreateGuestRequest) => {
    onAddGuest(data);
    setAddingToBookingId(null);
  };

  return (
    <div className="min-h-screen bg-gray-100">
      {/* Header */}
      <div className="bg-white shadow">
        <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 py-6">
          <div className="flex items-center gap-4">
            <button type="button" onClick={onBack} className="text-gray-500 hover:text-gray-700">
              <svg
                className="w-6 h-6"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
                aria-hidden="true"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M10 19l-7-7m0 0l7-7m-7 7h18"
                />
              </svg>
            </button>
            <div>
              <h1 className="text-2xl font-bold text-gray-900">
                {t('rentals.guestRegistration.title')}
              </h1>
              <p className="mt-1 text-sm text-gray-500">
                {t('rentals.guestRegistration.subtitle')}
              </p>
            </div>
          </div>
        </div>
      </div>

      {/* Alert Banner */}
      {totalPendingGuests > 0 && (
        <div className="bg-orange-50 border-b border-orange-200">
          <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 py-4">
            <div className="flex items-center gap-3">
              <svg
                className="w-6 h-6 text-orange-500"
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
              <div>
                <p className="font-medium text-orange-800">
                  {t('rentals.guestRegistration.pendingCount', { count: totalPendingGuests })}
                </p>
                <p className="text-sm text-orange-700">
                  {t('rentals.guestRegistration.pendingDesc')}
                </p>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Content */}
      <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {isLoading ? (
          <div className="flex items-center justify-center h-64">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
          </div>
        ) : bookingsWithPendingGuests.length === 0 ? (
          <div className="bg-white rounded-lg shadow p-12 text-center">
            <svg
              className="w-16 h-16 mx-auto text-green-400"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              aria-hidden="true"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"
              />
            </svg>
            <h2 className="mt-4 text-lg font-medium text-gray-900">
              {t('rentals.guestRegistration.allRegistered')}
            </h2>
            <p className="mt-2 text-sm text-gray-500">
              {t('rentals.guestRegistration.allRegisteredDesc')}
            </p>
          </div>
        ) : (
          <div className="space-y-6">
            {bookingsWithPendingGuests.map((booking) => {
              const pendingGuests = booking.guests.filter(
                (g) => g.registrationStatus === 'pending'
              );
              const isExpanded = expandedBookingId === booking.id;

              return (
                <div key={booking.id} className="bg-white rounded-lg shadow overflow-hidden">
                  {/* Booking Header */}
                  <div
                    className="p-4 cursor-pointer hover:bg-gray-50"
                    onClick={() => setExpandedBookingId(isExpanded ? null : booking.id)}
                  >
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-4">
                        <div className="flex-shrink-0">
                          <div className="w-12 h-12 bg-blue-100 rounded-lg flex items-center justify-center">
                            <span className="text-lg font-bold text-blue-600">
                              {pendingGuests.length}
                            </span>
                          </div>
                        </div>
                        <div>
                          <h3 className="font-semibold text-gray-900">{booking.guestName}</h3>
                          <p className="text-sm text-gray-500">
                            {booking.unitName} |{' '}
                            {formatDateRange(booking.checkIn, booking.checkOut, i18n.language)}
                          </p>
                        </div>
                      </div>
                      <div className="flex items-center gap-3">
                        <button
                          type="button"
                          onClick={(e) => {
                            e.stopPropagation();
                            onRegisterAllGuests(booking.id);
                          }}
                          disabled={isSubmitting || pendingGuests.length === 0}
                          className="px-3 py-1 text-sm bg-green-600 text-white rounded-md hover:bg-green-700 disabled:opacity-50"
                        >
                          {t('rentals.guestRegistration.registerAll')}
                        </button>
                        <svg
                          className={`w-5 h-5 text-gray-400 transition-transform ${
                            isExpanded ? 'rotate-180' : ''
                          }`}
                          fill="none"
                          viewBox="0 0 24 24"
                          stroke="currentColor"
                          aria-hidden="true"
                        >
                          <path
                            strokeLinecap="round"
                            strokeLinejoin="round"
                            strokeWidth={2}
                            d="M19 9l-7 7-7-7"
                          />
                        </svg>
                      </div>
                    </div>
                  </div>

                  {/* Guest List */}
                  {isExpanded && (
                    <div className="border-t">
                      {booking.guests.map((guest) => {
                        const isEditing = editingGuestId === guest.id;
                        const isPending = guest.registrationStatus === 'pending';

                        if (isEditing) {
                          return (
                            <div key={guest.id} className="p-4 bg-gray-50">
                              <h4 className="font-medium text-gray-900 mb-4">
                                {t('rentals.guest.editGuest')}
                              </h4>
                              <GuestForm
                                bookingId={booking.id}
                                initialData={guest}
                                isEditing
                                isSubmitting={isSubmitting}
                                onSubmit={(data) =>
                                  handleEditSubmit(guest.id, data as UpdateGuestRequest)
                                }
                                onCancel={() => setEditingGuestId(null)}
                              />
                            </div>
                          );
                        }

                        return (
                          <div
                            key={guest.id}
                            className="p-4 border-b last:border-b-0 flex items-center justify-between"
                          >
                            <div className="flex items-center gap-3">
                              <div className="w-10 h-10 bg-gray-200 rounded-full flex items-center justify-center">
                                <span className="text-sm font-medium text-gray-600">
                                  {guest.firstName[0]}
                                  {guest.lastName[0]}
                                </span>
                              </div>
                              <div>
                                <div className="flex items-center gap-2">
                                  <p className="font-medium text-gray-900">
                                    {guest.firstName} {guest.lastName}
                                  </p>
                                  {guest.isPrimary && (
                                    <span className="px-2 py-0.5 text-xs bg-blue-100 text-blue-700 rounded">
                                      {t('rentals.guest.primary')}
                                    </span>
                                  )}
                                  <span
                                    className={`px-2 py-0.5 text-xs rounded ${
                                      isPending
                                        ? 'bg-yellow-100 text-yellow-700'
                                        : 'bg-green-100 text-green-700'
                                    }`}
                                  >
                                    {t(
                                      `rentals.guest.registrationStatus.${guest.registrationStatus}`
                                    )}
                                  </span>
                                </div>
                                <p className="text-sm text-gray-500">
                                  {guest.nationality &&
                                    t(`rentals.guest.countries.${guest.nationality}`)}{' '}
                                  |{' '}
                                  {guest.documentType &&
                                    t(`rentals.guest.documentTypes.${guest.documentType}`)}{' '}
                                  {guest.documentNumber}
                                </p>
                              </div>
                            </div>
                            <div className="flex items-center gap-2">
                              {isPending && (
                                <button
                                  type="button"
                                  onClick={() => onRegisterGuest(guest.id)}
                                  disabled={isSubmitting}
                                  className="px-3 py-1 text-sm bg-green-600 text-white rounded-md hover:bg-green-700 disabled:opacity-50"
                                >
                                  {t('rentals.guest.register')}
                                </button>
                              )}
                              <button
                                type="button"
                                onClick={() => setEditingGuestId(guest.id)}
                                className="px-3 py-1 text-sm text-gray-700 border border-gray-300 rounded-md hover:bg-gray-50"
                              >
                                {t('common.edit')}
                              </button>
                            </div>
                          </div>
                        );
                      })}

                      {/* Add Guest Section */}
                      {addingToBookingId === booking.id ? (
                        <div className="p-4 bg-gray-50 border-t">
                          <h4 className="font-medium text-gray-900 mb-4">
                            {t('rentals.guest.addGuest')}
                          </h4>
                          <GuestForm
                            bookingId={booking.id}
                            isSubmitting={isSubmitting}
                            onSubmit={(data) => handleAddSubmit(data as CreateGuestRequest)}
                            onCancel={() => setAddingToBookingId(null)}
                          />
                        </div>
                      ) : (
                        <div className="p-4 border-t">
                          <button
                            type="button"
                            onClick={() => setAddingToBookingId(booking.id)}
                            className="text-sm text-blue-600 hover:text-blue-800"
                          >
                            + {t('rentals.guest.addGuest')}
                          </button>
                        </div>
                      )}
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}

        {/* Help Section */}
        <div className="mt-8 bg-blue-50 border border-blue-200 rounded-lg p-6">
          <h3 className="text-lg font-semibold text-blue-900">
            {t('rentals.guestRegistration.helpTitle')}
          </h3>
          <p className="mt-2 text-sm text-blue-700">{t('rentals.guestRegistration.helpDesc')}</p>
          <ul className="mt-4 space-y-2 text-sm text-blue-700">
            <li className="flex items-start gap-2">
              <svg
                className="w-5 h-5 text-blue-500 flex-shrink-0"
                fill="currentColor"
                viewBox="0 0 20 20"
                aria-hidden="true"
              >
                <path
                  fillRule="evenodd"
                  d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"
                  clipRule="evenodd"
                />
              </svg>
              {t('rentals.guestRegistration.helpItem1')}
            </li>
            <li className="flex items-start gap-2">
              <svg
                className="w-5 h-5 text-blue-500 flex-shrink-0"
                fill="currentColor"
                viewBox="0 0 20 20"
                aria-hidden="true"
              >
                <path
                  fillRule="evenodd"
                  d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"
                  clipRule="evenodd"
                />
              </svg>
              {t('rentals.guestRegistration.helpItem2')}
            </li>
            <li className="flex items-start gap-2">
              <svg
                className="w-5 h-5 text-blue-500 flex-shrink-0"
                fill="currentColor"
                viewBox="0 0 20 20"
                aria-hidden="true"
              >
                <path
                  fillRule="evenodd"
                  d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"
                  clipRule="evenodd"
                />
              </svg>
              {t('rentals.guestRegistration.helpItem3')}
            </li>
          </ul>
        </div>
      </div>
    </div>
  );
}
