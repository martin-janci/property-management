/**
 * Booking Form Component (Epic 56: Facility Booking).
 *
 * Form for creating or editing a facility booking.
 */

import type { AvailableSlot, CreateBookingRequest, Facility } from '@ppt/api-client';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';

interface BookingFormProps {
  facility: Facility;
  availableSlots?: AvailableSlot[];
  selectedDate: string;
  onDateChange: (date: string) => void;
  onSubmit: (data: CreateBookingRequest) => void;
  onCancel: () => void;
  isSubmitting?: boolean;
}

/**
 * Whether a local date-time is bookable given the availability response.
 *
 * The api-server reports existing bookings as `is_available: false` slots and
 * never lists the free time, so a time is free unless it falls inside one of
 * them. If the response ever does list explicit free slots
 * (`is_available: true`), the time must also fall inside one of those.
 */
export function isTimeAvailable(localDateTime: string, slots?: AvailableSlot[]): boolean {
  if (!slots || slots.length === 0) return true;
  const at = new Date(localDateTime);
  const contains = (slot: AvailableSlot) =>
    at >= new Date(slot.start_time) && at < new Date(slot.end_time);
  if (slots.some((slot) => !slot.is_available && contains(slot))) return false;
  const freeSlots = slots.filter((slot) => slot.is_available);
  return freeSlots.length === 0 || freeSlots.some(contains);
}

export function BookingForm({
  facility,
  availableSlots,
  selectedDate,
  onDateChange,
  onSubmit,
  onCancel,
  isSubmitting,
}: BookingFormProps) {
  const { t, i18n } = useTranslation();
  const formatNumber = (value: number, fractionDigits = 0) =>
    new Intl.NumberFormat(i18n.language, {
      minimumFractionDigits: fractionDigits,
      maximumFractionDigits: Math.max(fractionDigits, 2),
    }).format(value);
  const [startTime, setStartTime] = useState('');
  const [endTime, setEndTime] = useState('');
  const [purpose, setPurpose] = useState('');
  const [notes, setNotes] = useState('');
  const [attendeesCount, setAttendeesCount] = useState<number | ''>('');
  const [errors, setErrors] = useState<Record<string, string>>({});

  // Reset times when date changes - intentional dependency on selectedDate
  // biome-ignore lint/correctness/useExhaustiveDependencies: we want to reset when date changes
  useEffect(() => {
    setStartTime('');
    setEndTime('');
  }, [selectedDate]);

  const validate = (): boolean => {
    const newErrors: Record<string, string> = {};

    if (!selectedDate) {
      newErrors.date = t('facilities.bookings.form.errors.selectDate');
    }
    if (!startTime) {
      newErrors.startTime = t('facilities.bookings.form.errors.selectStart');
    }
    if (!endTime) {
      newErrors.endTime = t('facilities.bookings.form.errors.selectEnd');
    }
    if (startTime && endTime && startTime >= endTime) {
      newErrors.endTime = t('facilities.bookings.form.errors.endAfterStart');
    }

    // Check max booking hours
    if (startTime && endTime && facility.max_booking_hours) {
      const start = new Date(`${selectedDate}T${startTime}`);
      const end = new Date(`${selectedDate}T${endTime}`);
      const hours = (end.getTime() - start.getTime()) / (1000 * 60 * 60);
      if (hours > facility.max_booking_hours) {
        newErrors.endTime = t('facilities.bookings.form.errors.maxDuration', {
          hours: facility.max_booking_hours,
        });
      }
    }

    // Check capacity
    if (attendeesCount && facility.capacity && Number(attendeesCount) > facility.capacity) {
      newErrors.attendeesCount = t('facilities.bookings.form.errors.maxCapacity', {
        count: facility.capacity,
      });
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!validate()) return;

    // Use the selected local date and time directly to avoid unintended timezone conversion
    const startDateTime = `${selectedDate}T${startTime}:00`;
    const endDateTime = `${selectedDate}T${endTime}:00`;

    onSubmit({
      start_time: startDateTime,
      end_time: endDateTime,
      purpose: purpose || undefined,
      notes: notes || undefined,
      attendees_count: attendeesCount ? Number(attendeesCount) : undefined,
    });
  };

  // Generate time options based on facility availability
  const timeOptions: string[] = [];
  const availableFrom = facility.available_from || '00:00';
  const availableTo = facility.available_to || '23:59';

  for (let hour = 0; hour < 24; hour++) {
    for (const minute of ['00', '30']) {
      const time = `${hour.toString().padStart(2, '0')}:${minute}`;
      if (time >= availableFrom && time <= availableTo) {
        timeOptions.push(time);
      }
    }
  }

  // Filter to available slots if provided
  const isSlotAvailable = (time: string): boolean =>
    isTimeAvailable(`${selectedDate}T${time}:00`, availableSlots);

  return (
    <form onSubmit={handleSubmit} className="space-y-6">
      {/* Facility Info */}
      <div className="bg-blue-50 rounded-lg p-4">
        <h3 className="font-semibold text-gray-900">{facility.name}</h3>
        <div className="mt-2 text-sm text-gray-600 space-y-1">
          {facility.location && (
            <p>{t('facilities.bookings.form.location', { location: facility.location })}</p>
          )}
          {facility.capacity && (
            <p>{t('facilities.bookings.form.capacity', { count: facility.capacity })}</p>
          )}
          {facility.hourly_fee && (
            <p>{t('facilities.bookings.form.fee', { fee: facility.hourly_fee })}</p>
          )}
          {facility.max_booking_hours && (
            <p>
              {t('facilities.bookings.form.maxDuration', { hours: facility.max_booking_hours })}
            </p>
          )}
          {facility.requires_approval && (
            <p className="text-yellow-700">{t('facilities.bookings.form.requiresApproval')}</p>
          )}
        </div>
      </div>

      {/* Date Selection */}
      <div>
        <label htmlFor="date" className="block text-sm font-medium text-gray-700">
          {t('facilities.bookings.form.date')}
        </label>
        <input
          type="date"
          id="date"
          value={selectedDate}
          onChange={(e) => onDateChange(e.target.value)}
          min={new Date().toISOString().split('T')[0]}
          max={
            facility.max_advance_days
              ? new Date(Date.now() + facility.max_advance_days * 24 * 60 * 60 * 1000)
                  .toISOString()
                  .split('T')[0]
              : undefined
          }
          className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
        />
        {errors.date && <p className="mt-1 text-sm text-red-600">{errors.date}</p>}
      </div>

      {/* Time Selection */}
      <div className="grid grid-cols-2 gap-4">
        <div>
          <label htmlFor="startTime" className="block text-sm font-medium text-gray-700">
            {t('facilities.bookings.form.startTime')}
          </label>
          <select
            id="startTime"
            value={startTime}
            onChange={(e) => setStartTime(e.target.value)}
            className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
          >
            <option value="">{t('facilities.bookings.form.selectTime')}</option>
            {timeOptions.map((time) => (
              <option key={time} value={time} disabled={!isSlotAvailable(time)}>
                {time} {!isSlotAvailable(time) ? t('facilities.bookings.form.unavailable') : ''}
              </option>
            ))}
          </select>
          {errors.startTime && <p className="mt-1 text-sm text-red-600">{errors.startTime}</p>}
        </div>

        <div>
          <label htmlFor="endTime" className="block text-sm font-medium text-gray-700">
            {t('facilities.bookings.form.endTime')}
          </label>
          <select
            id="endTime"
            value={endTime}
            onChange={(e) => setEndTime(e.target.value)}
            className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
          >
            <option value="">{t('facilities.bookings.form.selectTime')}</option>
            {timeOptions
              .filter((time) => time > startTime)
              .map((time) => (
                <option key={time} value={time}>
                  {time}
                </option>
              ))}
          </select>
          {errors.endTime && <p className="mt-1 text-sm text-red-600">{errors.endTime}</p>}
        </div>
      </div>

      {/* Purpose */}
      <div>
        <label htmlFor="purpose" className="block text-sm font-medium text-gray-700">
          {t('facilities.bookings.form.purpose')}
        </label>
        <input
          type="text"
          id="purpose"
          value={purpose}
          onChange={(e) => setPurpose(e.target.value)}
          placeholder={t('facilities.bookings.form.purposePlaceholder')}
          className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
        />
      </div>

      {/* Attendees Count */}
      {facility.capacity && (
        <div>
          <label htmlFor="attendeesCount" className="block text-sm font-medium text-gray-700">
            {t('facilities.bookings.form.attendeesCount')}
          </label>
          <input
            type="number"
            id="attendeesCount"
            value={attendeesCount}
            onChange={(e) => setAttendeesCount(e.target.value ? Number(e.target.value) : '')}
            min={1}
            max={facility.capacity}
            placeholder={t('facilities.bookings.form.maxPlaceholder', { count: facility.capacity })}
            className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
          {errors.attendeesCount && (
            <p className="mt-1 text-sm text-red-600">{errors.attendeesCount}</p>
          )}
        </div>
      )}

      {/* Notes */}
      <div>
        <label htmlFor="notes" className="block text-sm font-medium text-gray-700">
          {t('facilities.bookings.form.notes')}
        </label>
        <textarea
          id="notes"
          value={notes}
          onChange={(e) => setNotes(e.target.value)}
          rows={3}
          placeholder={t('facilities.bookings.form.notesPlaceholder')}
          className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
        />
      </div>

      {/* Fee Calculation */}
      {facility.hourly_fee && startTime && endTime && (
        <div className="bg-gray-50 rounded-lg p-4">
          <div className="flex justify-between text-sm">
            <span>{t('facilities.bookings.form.durationLabel')}</span>
            <span>
              {(() => {
                const start = new Date(`2000-01-01T${startTime}`);
                const end = new Date(`2000-01-01T${endTime}`);
                const hours = (end.getTime() - start.getTime()) / (1000 * 60 * 60);
                return t('facilities.bookings.form.hoursValue', { hours: formatNumber(hours) });
              })()}
            </span>
          </div>
          <div className="flex justify-between font-semibold mt-2">
            <span>{t('facilities.bookings.form.estimatedTotal')}</span>
            <span>
              {(() => {
                const start = new Date(`2000-01-01T${startTime}`);
                const end = new Date(`2000-01-01T${endTime}`);
                const hours = (end.getTime() - start.getTime()) / (1000 * 60 * 60);
                const fee = Number.parseFloat(facility.hourly_fee || '0');
                return formatNumber(Number.isNaN(fee) ? 0 : fee * hours, 2);
              })()}
            </span>
          </div>
          {facility.deposit_amount && (
            <div className="flex justify-between text-sm text-gray-600 mt-1">
              <span>{t('facilities.bookings.form.depositRequired')}</span>
              <span>{facility.deposit_amount}</span>
            </div>
          )}
        </div>
      )}

      {/* Actions */}
      <div className="flex gap-4 pt-4">
        <button
          type="button"
          onClick={onCancel}
          className="flex-1 px-4 py-2 border border-gray-300 rounded-md hover:bg-gray-50 transition-colors"
        >
          {t('facilities.bookings.form.cancel')}
        </button>
        <button
          type="submit"
          disabled={isSubmitting}
          className="flex-1 px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        >
          {isSubmitting
            ? t('facilities.bookings.form.submitting')
            : facility.requires_approval
              ? t('facilities.bookings.form.submitForApproval')
              : t('facilities.bookings.form.bookNow')}
        </button>
      </div>
    </form>
  );
}
