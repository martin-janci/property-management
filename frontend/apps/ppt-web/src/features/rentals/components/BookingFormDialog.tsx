/**
 * BookingFormDialog - create a manual booking or edit an existing one.
 * Epic 18: Short-Term Rental Integration (Story 18.2)
 *
 * Create → `POST /rentals/bookings` (unit + source chosen here); edit →
 * `PUT /rentals/bookings/{id}` (unit and source are fixed). Dates are the
 * half-open stay: check-in day … check-out day.
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { type BookingFormError, validateBookingForm } from '../mappers';
import type { BookingFormValues, BookingSource, RentalUnitOption } from '../types';

export interface BookingFormDialogProps {
  mode: 'create' | 'edit';
  initial: BookingFormValues;
  units: RentalUnitOption[];
  onSubmit: (values: BookingFormValues) => Promise<void>;
  onClose: () => void;
}

const SOURCES: BookingSource[] = ['direct', 'airbnb', 'booking'];

const inputClass =
  'mt-1 block w-full px-3 py-2 border rounded-md text-sm focus:ring-blue-500 focus:border-blue-500';

export function BookingFormDialog({
  mode,
  initial,
  units,
  onSubmit,
  onClose,
}: BookingFormDialogProps) {
  const { t } = useTranslation();
  const [values, setValues] = useState<BookingFormValues>(initial);
  const [touched, setTouched] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [serverError, setServerError] = useState<string | null>(null);

  const errors = validateBookingForm(values);
  const shown = (e: BookingFormError) => touched && errors.includes(e);
  const set = <K extends keyof BookingFormValues>(key: K, value: BookingFormValues[K]) =>
    setValues((v) => ({ ...v, [key]: value }));

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setTouched(true);
    if (errors.length > 0) return;
    setIsSaving(true);
    setServerError(null);
    try {
      await onSubmit(values);
      onClose();
    } catch (error) {
      // 409 = the unit already has a booking or block for these dates.
      const status = (error as { status?: number } | null)?.status;
      setServerError(
        status === 409 ? t('rentals.bookingForm.unavailable') : t('rentals.bookingForm.saveFailed')
      );
    } finally {
      setIsSaving(false);
    }
  };

  const fieldClass = (bad: boolean) =>
    `${inputClass} ${bad ? 'border-red-400' : 'border-gray-300'}`;

  return (
    <div className="fixed inset-0 z-50 overflow-y-auto">
      <div className="flex min-h-full items-center justify-center p-4">
        <div className="fixed inset-0 bg-black/25" aria-hidden="true" onClick={onClose} />
        <form
          onSubmit={handleSubmit}
          noValidate
          className="relative bg-white rounded-lg shadow-xl max-w-2xl w-full p-6"
          role="dialog"
          aria-modal="true"
          aria-labelledby="booking-form-title"
          onKeyDown={(e) => {
            if (e.key === 'Escape' && !isSaving) onClose();
          }}
        >
          <h3 id="booking-form-title" className="text-lg font-semibold text-gray-900">
            {mode === 'create'
              ? t('rentals.bookingForm.createTitle')
              : t('rentals.bookingForm.editTitle')}
          </h3>

          <div className="mt-6 grid grid-cols-1 sm:grid-cols-2 gap-4">
            {mode === 'create' && (
              <>
                <div>
                  <label htmlFor="bf-unit" className="block text-sm font-medium text-gray-700">
                    {t('rentals.bookingForm.unit')} *
                  </label>
                  <select
                    id="bf-unit"
                    value={values.unitId}
                    onChange={(e) => set('unitId', e.target.value)}
                    className={fieldClass(shown('unit'))}
                  >
                    <option value="">{t('common.select')}</option>
                    {units.map((u) => (
                      <option key={u.id} value={u.id}>
                        {u.name} - {u.buildingName}
                      </option>
                    ))}
                  </select>
                  {shown('unit') && (
                    <p className="mt-1 text-xs text-red-600">
                      {t('rentals.bookingForm.errors.unit')}
                    </p>
                  )}
                </div>
                <div>
                  <label htmlFor="bf-source" className="block text-sm font-medium text-gray-700">
                    {t('rentals.bookingForm.source')}
                  </label>
                  <select
                    id="bf-source"
                    value={values.source}
                    onChange={(e) => set('source', e.target.value as BookingSource)}
                    className={fieldClass(false)}
                  >
                    {SOURCES.map((s) => (
                      <option key={s} value={s}>
                        {t(`rentals.source.${s}`)}
                      </option>
                    ))}
                  </select>
                </div>
              </>
            )}

            <div className="sm:col-span-2">
              <label htmlFor="bf-guest-name" className="block text-sm font-medium text-gray-700">
                {t('rentals.bookingForm.guestName')} *
              </label>
              <input
                id="bf-guest-name"
                type="text"
                value={values.guestName}
                onChange={(e) => set('guestName', e.target.value)}
                className={fieldClass(shown('guestName'))}
              />
              {shown('guestName') && (
                <p className="mt-1 text-xs text-red-600">
                  {t('rentals.bookingForm.errors.guestName')}
                </p>
              )}
            </div>

            <div>
              <label htmlFor="bf-email" className="block text-sm font-medium text-gray-700">
                {t('rentals.bookingForm.guestEmail')}
              </label>
              <input
                id="bf-email"
                type="email"
                value={values.guestEmail}
                onChange={(e) => set('guestEmail', e.target.value)}
                className={fieldClass(false)}
              />
            </div>
            <div>
              <label htmlFor="bf-phone" className="block text-sm font-medium text-gray-700">
                {t('rentals.bookingForm.guestPhone')}
              </label>
              <input
                id="bf-phone"
                type="tel"
                value={values.guestPhone}
                onChange={(e) => set('guestPhone', e.target.value)}
                className={fieldClass(false)}
              />
            </div>

            <div>
              <label htmlFor="bf-check-in" className="block text-sm font-medium text-gray-700">
                {t('rentals.bookingDetail.checkIn')} *
              </label>
              <input
                id="bf-check-in"
                type="date"
                value={values.checkIn}
                onChange={(e) => set('checkIn', e.target.value)}
                className={fieldClass(shown('dates') || shown('dateOrder'))}
              />
            </div>
            <div>
              <label htmlFor="bf-check-out" className="block text-sm font-medium text-gray-700">
                {t('rentals.bookingDetail.checkOut')} *
              </label>
              <input
                id="bf-check-out"
                type="date"
                value={values.checkOut}
                min={values.checkIn || undefined}
                onChange={(e) => set('checkOut', e.target.value)}
                className={fieldClass(shown('dates') || shown('dateOrder'))}
              />
            </div>
            {(shown('dates') || shown('dateOrder')) && (
              <p className="sm:col-span-2 -mt-2 text-xs text-red-600">
                {shown('dates')
                  ? t('rentals.bookingForm.errors.dates')
                  : t('rentals.bookingForm.errors.dateOrder')}
              </p>
            )}

            <div>
              <label htmlFor="bf-guest-count" className="block text-sm font-medium text-gray-700">
                {t('rentals.bookingForm.guestCount')}
              </label>
              <input
                id="bf-guest-count"
                type="number"
                min={1}
                step={1}
                value={Number.isFinite(values.guestCount) ? values.guestCount : ''}
                onChange={(e) => set('guestCount', Number.parseInt(e.target.value, 10))}
                className={fieldClass(shown('guestCount'))}
              />
              {shown('guestCount') && (
                <p className="mt-1 text-xs text-red-600">
                  {t('rentals.bookingForm.errors.guestCount')}
                </p>
              )}
            </div>
            <div>
              <label htmlFor="bf-price" className="block text-sm font-medium text-gray-700">
                {t('rentals.bookingForm.totalPrice')}
              </label>
              <div className="mt-1 flex gap-2">
                <input
                  id="bf-price"
                  type="text"
                  inputMode="decimal"
                  value={values.totalPrice}
                  onChange={(e) => set('totalPrice', e.target.value)}
                  className={`${fieldClass(shown('price'))} mt-0 flex-1`}
                />
                <select
                  aria-label={t('rentals.bookingForm.currency')}
                  value={values.currency}
                  onChange={(e) => set('currency', e.target.value)}
                  className="px-2 py-2 border border-gray-300 rounded-md text-sm"
                >
                  {['EUR', 'CZK', 'PLN', 'HUF', 'USD'].map((c) => (
                    <option key={c} value={c}>
                      {c}
                    </option>
                  ))}
                </select>
              </div>
              {shown('price') && (
                <p className="mt-1 text-xs text-red-600">{t('rentals.bookingForm.errors.price')}</p>
              )}
            </div>

            <div className="sm:col-span-2">
              <label htmlFor="bf-guest-notes" className="block text-sm font-medium text-gray-700">
                {t('rentals.bookingForm.guestNotes')}
              </label>
              <textarea
                id="bf-guest-notes"
                rows={2}
                value={values.guestNotes}
                onChange={(e) => set('guestNotes', e.target.value)}
                className={fieldClass(false)}
              />
            </div>
            <div className="sm:col-span-2">
              <label
                htmlFor="bf-internal-notes"
                className="block text-sm font-medium text-gray-700"
              >
                {t('rentals.bookingForm.internalNotes')}
              </label>
              <textarea
                id="bf-internal-notes"
                rows={2}
                value={values.internalNotes}
                onChange={(e) => set('internalNotes', e.target.value)}
                className={fieldClass(false)}
              />
              <p className="mt-1 text-xs text-gray-500">
                {t('rentals.bookingForm.internalNotesHelp')}
              </p>
            </div>
          </div>

          {serverError && (
            <p className="mt-4 p-3 bg-red-50 rounded-md text-sm text-red-700" role="alert">
              {serverError}
            </p>
          )}

          <div className="mt-6 flex items-center justify-end gap-3">
            <button
              type="button"
              onClick={onClose}
              disabled={isSaving}
              className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50"
            >
              {t('common.cancel')}
            </button>
            <button
              type="submit"
              disabled={isSaving}
              className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 disabled:opacity-50"
            >
              {isSaving
                ? t('common.saving')
                : mode === 'create'
                  ? t('rentals.bookingForm.create')
                  : t('common.save')}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
