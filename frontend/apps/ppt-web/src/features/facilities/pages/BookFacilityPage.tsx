/**
 * Book Facility Page (Epic 56: Facility Booking).
 *
 * Allows users to book a specific facility.
 */

import type { AvailableSlot, CreateBookingRequest, Facility } from '@ppt/api-client';
import { checkAvailability, createBooking, getFacility } from '@ppt/api-client';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate, useParams } from 'react-router-dom';
import { BookingForm } from '../components';

export function BookFacilityPage() {
  const { t } = useTranslation();
  const { buildingId, facilityId } = useParams<{ buildingId: string; facilityId: string }>();
  const navigate = useNavigate();

  const [facility, setFacility] = useState<Facility | null>(null);
  const [availableSlots, setAvailableSlots] = useState<AvailableSlot[]>([]);
  const [selectedDate, setSelectedDate] = useState(new Date().toISOString().split('T')[0]);
  const [isLoading, setIsLoading] = useState(true);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Fetch facility details
  useEffect(() => {
    if (!buildingId || !facilityId) return;

    const fetchFacility = async () => {
      try {
        const data = await getFacility(buildingId, facilityId);
        setFacility(data);
      } catch (err) {
        setError('Failed to load facility details');
        console.error('Failed to fetch facility:', err);
      } finally {
        setIsLoading(false);
      }
    };

    fetchFacility();
  }, [buildingId, facilityId]);

  // Fetch availability when date changes
  useEffect(() => {
    if (!buildingId || !facilityId || !selectedDate) return;

    const fetchAvailability = async () => {
      try {
        const slots = await checkAvailability(buildingId, facilityId, { date: selectedDate });
        setAvailableSlots(slots);
      } catch (err) {
        console.error('Failed to fetch availability:', err);
      }
    };

    fetchAvailability();
  }, [buildingId, facilityId, selectedDate]);

  const handleSubmit = async (data: CreateBookingRequest) => {
    if (!buildingId || !facilityId) return;

    setIsSubmitting(true);
    setError(null);

    try {
      await createBooking(buildingId, facilityId, data);
      navigate(`/buildings/${buildingId}/facilities/${facilityId}?booked=true`);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create booking');
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleCancel = () => {
    navigate(-1);
  };

  if (isLoading) {
    return (
      <div className="max-w-2xl mx-auto px-4 py-8">
        <div className="flex items-center justify-center py-12">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
        </div>
      </div>
    );
  }

  if (!facility) {
    return (
      <div className="max-w-2xl mx-auto px-4 py-8">
        <div className="text-center py-12">
          <p className="text-red-600">{error || 'Facility not found'}</p>
          <button
            type="button"
            onClick={() => navigate(-1)}
            className="mt-4 text-blue-600 hover:text-blue-800"
          >
            Go back
          </button>
        </div>
      </div>
    );
  }

  if (!facility.is_bookable) {
    return (
      <div className="max-w-2xl mx-auto px-4 py-8">
        <div className="text-center py-12">
          <p className="text-yellow-600">This facility is not available for booking</p>
          <button
            type="button"
            onClick={() => navigate(-1)}
            className="mt-4 text-blue-600 hover:text-blue-800"
          >
            Go back
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="max-w-2xl mx-auto px-4 py-8">
      <h1 className="text-2xl font-bold text-gray-900 mb-6">
        {t('facilities.bookings.bookFacility')}
      </h1>

      {error && (
        <div className="mb-6 p-4 bg-red-50 border border-red-200 rounded-md text-red-700">
          {error}
        </div>
      )}

      <BookingForm
        facility={facility}
        availableSlots={availableSlots}
        selectedDate={selectedDate}
        onDateChange={setSelectedDate}
        onSubmit={handleSubmit}
        onCancel={handleCancel}
        isSubmitting={isSubmitting}
      />
    </div>
  );
}
