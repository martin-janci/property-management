/**
 * BookingDetailPage - View booking details with guest info.
 * Epic 18: Short-Term Rental Integration (Story 18.2, 18.3)
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ConfirmationDialog } from '../../../components/ConfirmationDialog';
import { BookingFormDialog } from '../components/BookingFormDialog';
import { GuestForm } from '../components/GuestForm';
import { parseIsoDate, toIsoDate } from '../mappers';
import type {
  BookingFormValues,
  BookingWithGuests,
  CreateGuestRequest,
  RentalGuest,
  UpdateGuestRequest,
} from '../types';

interface BookingDetailPageProps {
  booking: BookingWithGuests;
  isLoading?: boolean;
  onBack: () => void;
  /** Values for the edit dialog; the Edit button is hidden without them. */
  editForm?: BookingFormValues;
  /** Open the edit dialog right away (e.g. `?edit=1` from the bookings list). */
  initialEditing?: boolean;
  onSaveBooking: (values: BookingFormValues) => Promise<void>;
  /** Called after the user confirmed the cancellation. */
  onCancel: (id: string) => void;
  onCheckIn: (id: string) => void;
  onCheckOut: (id: string) => void;
  onAddGuest: (data: CreateGuestRequest) => Promise<void>;
  onEditGuest: (guestId: string, data: UpdateGuestRequest) => Promise<void>;
  onRegisterGuest: (guestId: string) => void;
  /** Called after the user confirmed the deletion. */
  onDeleteGuest: (guestId: string) => void;
}

function formatDate(dateString: string, language: string): string {
  return new Intl.DateTimeFormat(language, {
    weekday: 'long',
    year: 'numeric',
    month: 'long',
    day: 'numeric',
  }).format(parseIsoDate(dateString));
}

function formatCurrency(
  amount: number | undefined,
  currency: string | undefined,
  language: string
): string {
  if (amount === undefined) return '-';
  return new Intl.NumberFormat(language, {
    style: 'currency',
    currency: currency || 'EUR',
  }).format(amount);
}

function getNights(checkIn: string, checkOut: string): number {
  const start = parseIsoDate(checkIn);
  const end = parseIsoDate(checkOut);
  return Math.round((end.getTime() - start.getTime()) / (1000 * 60 * 60 * 24));
}

function GuestCard({
  guest,
  onEdit,
  onRegister,
  onDelete,
  t,
}: {
  guest: RentalGuest;
  onEdit: () => void;
  onRegister: () => void;
  onDelete: () => void;
  t: (key: string) => string;
}) {
  const { i18n } = useTranslation();
  const language = i18n.language;
  const isRegistered = guest.registrationStatus === 'registered';
  const isPending = guest.registrationStatus === 'pending';

  return (
    <div className="bg-white rounded-lg border p-4">
      <div className="flex items-start justify-between">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 bg-gray-200 rounded-full flex items-center justify-center">
            <span className="text-sm font-medium text-gray-600">
              {guest.firstName[0]}
              {guest.lastName[0]}
            </span>
          </div>
          <div>
            <div className="flex items-center gap-2">
              <h4 className="font-medium text-gray-900">
                {guest.firstName} {guest.lastName}
              </h4>
              {guest.isPrimary && (
                <span className="px-2 py-0.5 text-xs bg-blue-100 text-blue-700 rounded">
                  {t('rentals.guest.primary')}
                </span>
              )}
            </div>
            {guest.email && <p className="text-sm text-gray-500">{guest.email}</p>}
          </div>
        </div>
        <span
          className={`px-2 py-1 text-xs rounded ${
            isRegistered
              ? 'bg-green-100 text-green-700'
              : isPending
                ? 'bg-yellow-100 text-yellow-700'
                : 'bg-red-100 text-red-700'
          }`}
        >
          {t(`rentals.guest.registrationStatus.${guest.registrationStatus}`)}
        </span>
      </div>

      <div className="mt-4 grid grid-cols-2 gap-4 text-sm">
        {guest.nationality && (
          <div>
            <span className="text-gray-500">{t('rentals.guest.nationality')}</span>
            <p className="font-medium text-gray-900">
              {t(`rentals.guest.countries.${guest.nationality}`)}
            </p>
          </div>
        )}
        {guest.dateOfBirth && (
          <div>
            <span className="text-gray-500">{t('rentals.guest.dateOfBirth')}</span>
            <p className="font-medium text-gray-900">
              {parseIsoDate(guest.dateOfBirth).toLocaleDateString(language)}
            </p>
          </div>
        )}
        {guest.documentType && (
          <div>
            <span className="text-gray-500">{t('rentals.guest.documentType')}</span>
            <p className="font-medium text-gray-900">
              {t(`rentals.guest.documentTypes.${guest.documentType}`)}
            </p>
          </div>
        )}
        {guest.documentNumber && (
          <div>
            <span className="text-gray-500">{t('rentals.guest.documentNumber')}</span>
            <p className="font-medium text-gray-900 font-mono">{guest.documentNumber}</p>
          </div>
        )}
      </div>

      <div className="mt-4 pt-3 border-t flex items-center justify-end gap-2">
        {isPending && (
          <button
            type="button"
            onClick={onRegister}
            className="px-3 py-1 text-sm bg-green-600 text-white rounded-md hover:bg-green-700"
          >
            {t('rentals.guest.register')}
          </button>
        )}
        <button
          type="button"
          onClick={onEdit}
          className="px-3 py-1 text-sm text-gray-700 border border-gray-300 rounded-md hover:bg-gray-50"
        >
          {t('common.edit')}
        </button>
        <button
          type="button"
          onClick={onDelete}
          className="px-3 py-1 text-sm text-red-600 border border-red-200 rounded-md hover:bg-red-50"
        >
          {t('common.delete')}
        </button>
      </div>
    </div>
  );
}

export function BookingDetailPage({
  booking,
  isLoading,
  onBack,
  editForm,
  initialEditing = false,
  onSaveBooking,
  onCancel,
  onCheckIn,
  onCheckOut,
  onAddGuest,
  onEditGuest,
  onRegisterGuest,
  onDeleteGuest,
}: BookingDetailPageProps) {
  const { t, i18n } = useTranslation();
  const language = i18n.language;
  const [isEditing, setIsEditing] = useState(initialEditing);
  const [confirmCancel, setConfirmCancel] = useState(false);
  const [guestToDelete, setGuestToDelete] = useState<RentalGuest | null>(null);
  // null = closed, 'new' = add guest, otherwise the guest being edited.
  const [guestDialog, setGuestDialog] = useState<'new' | RentalGuest | null>(null);
  const [isSavingGuest, setIsSavingGuest] = useState(false);

  const submitGuest = async (data: CreateGuestRequest | UpdateGuestRequest) => {
    if (!guestDialog) return;
    setIsSavingGuest(true);
    try {
      if (guestDialog === 'new') await onAddGuest(data as CreateGuestRequest);
      else await onEditGuest(guestDialog.id, data as UpdateGuestRequest);
      setGuestDialog(null);
    } catch {
      // The caller surfaces the failure (toast); keep the form open with its input.
    } finally {
      setIsSavingGuest(false);
    }
  };

  if (isLoading) {
    return (
      <div className="min-h-screen bg-gray-100 flex items-center justify-center">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
      </div>
    );
  }

  const nights = getNights(booking.checkIn, booking.checkOut);
  const isPast = booking.checkOut < toIsoDate(new Date());
  const canCheckIn = booking.status === 'confirmed' && !isPast;
  const canCheckOut = booking.status === 'checked_in';
  const canCancel = ['pending', 'confirmed'].includes(booking.status);
  const canEdit = !!editForm && !isPast && booking.status !== 'cancelled';
  const pendingGuests = booking.guests.filter((g) => g.registrationStatus === 'pending');

  const statusColors: Record<string, string> = {
    pending: 'bg-yellow-100 text-yellow-800',
    confirmed: 'bg-blue-100 text-blue-800',
    checked_in: 'bg-green-100 text-green-800',
    checked_out: 'bg-gray-100 text-gray-800',
    cancelled: 'bg-red-100 text-red-800',
    no_show: 'bg-orange-100 text-orange-800',
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
            <div className="flex-1">
              <div className="flex items-center gap-3">
                <h1 className="text-2xl font-bold text-gray-900">
                  {t('rentals.bookingDetail.title')}
                </h1>
                <span className={`px-3 py-1 text-sm rounded ${statusColors[booking.status]}`}>
                  {t(`rentals.status.${booking.status}`)}
                </span>
              </div>
              <p className="mt-1 text-sm text-gray-500">
                {booking.unitName} - {booking.buildingName}
              </p>
            </div>
            <div className="flex items-center gap-2">
              {canCheckIn && (
                <button
                  type="button"
                  onClick={() => onCheckIn(booking.id)}
                  className="px-4 py-2 text-sm font-medium text-white bg-green-600 rounded-md hover:bg-green-700"
                >
                  {t('rentals.checkIn')}
                </button>
              )}
              {canCheckOut && (
                <button
                  type="button"
                  onClick={() => onCheckOut(booking.id)}
                  className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700"
                >
                  {t('rentals.checkOut')}
                </button>
              )}
              {canEdit && (
                <button
                  type="button"
                  onClick={() => setIsEditing(true)}
                  className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50"
                >
                  {t('rentals.bookingForm.editTitle')}
                </button>
              )}
              {canCancel && (
                <button
                  type="button"
                  onClick={() => setConfirmCancel(true)}
                  className="px-4 py-2 text-sm font-medium text-red-600 bg-white border border-red-200 rounded-md hover:bg-red-50"
                >
                  {t('rentals.bookings.cancelBooking')}
                </button>
              )}
            </div>
          </div>
        </div>
      </div>

      {/* Content */}
      <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {/* Warning for pending registrations */}
        {pendingGuests.length > 0 && (
          <div className="mb-6 p-4 bg-orange-50 border border-orange-200 rounded-lg">
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
                  {t('rentals.bookingDetail.pendingRegistrations', {
                    count: pendingGuests.length,
                  })}
                </p>
                <p className="text-sm text-orange-700">
                  {t('rentals.bookingDetail.pendingRegistrationsDesc')}
                </p>
              </div>
            </div>
          </div>
        )}

        <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
          {/* Main Info */}
          <div className="lg:col-span-2 space-y-6">
            {/* Booking Details */}
            <div className="bg-white rounded-lg shadow p-6">
              <h2 className="text-lg font-semibold text-gray-900 mb-4">
                {t('rentals.bookingDetail.details')}
              </h2>
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <span className="text-sm text-gray-500">
                    {t('rentals.bookingDetail.checkIn')}
                  </span>
                  <p className="font-medium text-gray-900">
                    {formatDate(booking.checkIn, language)}
                  </p>
                </div>
                <div>
                  <span className="text-sm text-gray-500">
                    {t('rentals.bookingDetail.checkOut')}
                  </span>
                  <p className="font-medium text-gray-900">
                    {formatDate(booking.checkOut, language)}
                  </p>
                </div>
                <div>
                  <span className="text-sm text-gray-500">{t('rentals.bookingDetail.nights')}</span>
                  <p className="font-medium text-gray-900">{nights}</p>
                </div>
                <div>
                  <span className="text-sm text-gray-500">{t('rentals.bookingDetail.guests')}</span>
                  <p className="font-medium text-gray-900">{booking.guestCount}</p>
                </div>
                <div>
                  <span className="text-sm text-gray-500">{t('rentals.bookingDetail.source')}</span>
                  <p className="font-medium text-gray-900">
                    {t(`rentals.source.${booking.source}`)}
                  </p>
                </div>
                {booking.platformBookingId && (
                  <div>
                    <span className="text-sm text-gray-500">
                      {t('rentals.bookingDetail.platformId')}
                    </span>
                    <p className="font-medium text-gray-900 font-mono text-sm">
                      {booking.platformBookingId}
                    </p>
                  </div>
                )}
              </div>
              {(booking.guestNotes || booking.internalNotes) && (
                <div className="mt-4 pt-4 border-t space-y-3">
                  {booking.guestNotes && (
                    <div>
                      <span className="text-sm text-gray-500">
                        {t('rentals.bookingForm.guestNotes')}
                      </span>
                      <p className="mt-1 text-gray-900 whitespace-pre-line">{booking.guestNotes}</p>
                    </div>
                  )}
                  {booking.internalNotes && (
                    <div>
                      <span className="text-sm text-gray-500">
                        {t('rentals.bookingForm.internalNotes')}
                      </span>
                      <p className="mt-1 text-gray-900 whitespace-pre-line">
                        {booking.internalNotes}
                      </p>
                    </div>
                  )}
                </div>
              )}
            </div>

            {/* Guests */}
            <div className="bg-white rounded-lg shadow p-6">
              <div className="flex items-center justify-between mb-4">
                <h2 className="text-lg font-semibold text-gray-900">
                  {t('rentals.bookingDetail.guestList')}
                </h2>
                <button
                  type="button"
                  onClick={() => setGuestDialog('new')}
                  className="text-sm text-blue-600 hover:text-blue-800"
                >
                  + {t('rentals.guest.addGuest')}
                </button>
              </div>
              {booking.guests.length === 0 ? (
                <div className="text-center py-8">
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
                      d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0z"
                    />
                  </svg>
                  <p className="mt-2 text-gray-500">{t('rentals.bookingDetail.noGuests')}</p>
                  <button
                    type="button"
                    onClick={() => setGuestDialog('new')}
                    className="mt-4 px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700"
                  >
                    {t('rentals.guest.addFirstGuest')}
                  </button>
                </div>
              ) : (
                <div className="space-y-4">
                  {booking.guests.map((guest) => (
                    <GuestCard
                      key={guest.id}
                      guest={guest}
                      onEdit={() => setGuestDialog(guest)}
                      onRegister={() => onRegisterGuest(guest.id)}
                      onDelete={() => setGuestToDelete(guest)}
                      t={t}
                    />
                  ))}
                </div>
              )}
            </div>
          </div>

          {/* Sidebar */}
          <div className="space-y-6">
            {/* Primary Guest Contact */}
            <div className="bg-white rounded-lg shadow p-6">
              <h2 className="text-lg font-semibold text-gray-900 mb-4">
                {t('rentals.bookingDetail.contact')}
              </h2>
              <div className="space-y-3">
                <div className="flex items-center gap-3">
                  <div className="w-10 h-10 bg-blue-100 rounded-full flex items-center justify-center">
                    <svg
                      className="w-5 h-5 text-blue-600"
                      fill="none"
                      viewBox="0 0 24 24"
                      stroke="currentColor"
                      aria-hidden="true"
                    >
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z"
                      />
                    </svg>
                  </div>
                  <div>
                    <p className="font-medium text-gray-900">{booking.guestName}</p>
                    <p className="text-sm text-gray-500">
                      {t('rentals.bookingDetail.primaryGuest')}
                    </p>
                  </div>
                </div>
                {booking.guestEmail && (
                  <a
                    href={`mailto:${booking.guestEmail}`}
                    className="flex items-center gap-3 text-sm text-gray-600 hover:text-blue-600"
                  >
                    <svg
                      className="w-5 h-5"
                      fill="none"
                      viewBox="0 0 24 24"
                      stroke="currentColor"
                      aria-hidden="true"
                    >
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M3 8l7.89 5.26a2 2 0 002.22 0L21 8M5 19h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"
                      />
                    </svg>
                    {booking.guestEmail}
                  </a>
                )}
                {booking.guestPhone && (
                  <a
                    href={`tel:${booking.guestPhone}`}
                    className="flex items-center gap-3 text-sm text-gray-600 hover:text-blue-600"
                  >
                    <svg
                      className="w-5 h-5"
                      fill="none"
                      viewBox="0 0 24 24"
                      stroke="currentColor"
                      aria-hidden="true"
                    >
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M3 5a2 2 0 012-2h3.28a1 1 0 01.948.684l1.498 4.493a1 1 0 01-.502 1.21l-2.257 1.13a11.042 11.042 0 005.516 5.516l1.13-2.257a1 1 0 011.21-.502l4.493 1.498a1 1 0 01.684.949V19a2 2 0 01-2 2h-1C9.716 21 3 14.284 3 6V5z"
                      />
                    </svg>
                    {booking.guestPhone}
                  </a>
                )}
              </div>
            </div>

            {/* Payment Summary */}
            <div className="bg-white rounded-lg shadow p-6">
              <h2 className="text-lg font-semibold text-gray-900 mb-4">
                {t('rentals.bookingDetail.payment')}
              </h2>
              <div className="space-y-3">
                <div className="flex items-center justify-between">
                  <span className="text-gray-600">{t('rentals.bookingDetail.pricePerNight')}</span>
                  <span className="font-medium text-gray-900">
                    {booking.totalPrice && nights > 0
                      ? formatCurrency(booking.totalPrice / nights, booking.currency, language)
                      : '-'}
                  </span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-gray-600">{t('rentals.bookingDetail.nights')}</span>
                  <span className="font-medium text-gray-900">x {nights}</span>
                </div>
                <div className="pt-3 border-t flex items-center justify-between">
                  <span className="font-medium text-gray-900">
                    {t('rentals.bookingDetail.total')}
                  </span>
                  <span className="text-xl font-bold text-gray-900">
                    {formatCurrency(booking.totalPrice, booking.currency, language)}
                  </span>
                </div>
              </div>
            </div>

            {/* Timeline */}
            <div className="bg-white rounded-lg shadow p-6">
              <h2 className="text-lg font-semibold text-gray-900 mb-4">
                {t('rentals.bookingDetail.timeline')}
              </h2>
              <div className="space-y-4">
                <div className="flex items-start gap-3">
                  <div className="w-2 h-2 mt-2 rounded-full bg-blue-500" />
                  <div>
                    <p className="text-sm font-medium text-gray-900">
                      {t('rentals.bookingDetail.created')}
                    </p>
                    <p className="text-xs text-gray-500">
                      {new Date(booking.createdAt).toLocaleString(language)}
                    </p>
                  </div>
                </div>
                {booking.updatedAt !== booking.createdAt && (
                  <div className="flex items-start gap-3">
                    <div className="w-2 h-2 mt-2 rounded-full bg-gray-400" />
                    <div>
                      <p className="text-sm font-medium text-gray-900">
                        {t('rentals.bookingDetail.updated')}
                      </p>
                      <p className="text-xs text-gray-500">
                        {new Date(booking.updatedAt).toLocaleString(language)}
                      </p>
                    </div>
                  </div>
                )}
              </div>
            </div>
          </div>
        </div>
      </div>

      <ConfirmationDialog
        isOpen={confirmCancel}
        title={t('rentals.bookings.cancelTitle')}
        message={t('rentals.bookings.cancelMessage', {
          guest: booking.guestName,
          unit: booking.unitName ?? '',
        })}
        confirmLabel={t('rentals.bookings.cancelConfirm')}
        cancelLabel={t('rentals.bookings.keepBooking')}
        variant="danger"
        onConfirm={() => {
          setConfirmCancel(false);
          onCancel(booking.id);
        }}
        onCancel={() => setConfirmCancel(false)}
      />

      <ConfirmationDialog
        isOpen={guestToDelete !== null}
        title={t('rentals.guest.deleteTitle')}
        message={t('rentals.guest.deleteMessage', {
          name: guestToDelete ? `${guestToDelete.firstName} ${guestToDelete.lastName}` : '',
        })}
        confirmLabel={t('common.delete')}
        cancelLabel={t('common.cancel')}
        variant="danger"
        onConfirm={() => {
          if (guestToDelete) onDeleteGuest(guestToDelete.id);
          setGuestToDelete(null);
        }}
        onCancel={() => setGuestToDelete(null)}
      />

      {isEditing && editForm && (
        <BookingFormDialog
          mode="edit"
          initial={editForm}
          units={[]}
          onSubmit={onSaveBooking}
          onClose={() => setIsEditing(false)}
        />
      )}

      {guestDialog && (
        <div className="fixed inset-0 z-50 overflow-y-auto">
          <div className="flex min-h-full items-center justify-center p-4">
            <div className="fixed inset-0 bg-black/25" aria-hidden="true" />
            <div
              className="relative bg-white rounded-lg shadow-xl max-w-3xl w-full p-6"
              role="dialog"
              aria-modal="true"
              aria-labelledby="guest-dialog-title"
            >
              <h3 id="guest-dialog-title" className="text-lg font-semibold text-gray-900 mb-4">
                {guestDialog === 'new' ? t('rentals.guest.addGuest') : t('rentals.guest.editGuest')}
              </h3>
              <GuestForm
                bookingId={booking.id}
                initialData={guestDialog === 'new' ? undefined : guestDialog}
                isEditing={guestDialog !== 'new'}
                isSubmitting={isSavingGuest}
                onSubmit={submitGuest}
                onCancel={() => setGuestDialog(null)}
              />
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
