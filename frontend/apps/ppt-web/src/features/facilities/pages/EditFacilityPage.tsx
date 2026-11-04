/**
 * Edit Facility Page (Epic 56: Facility Booking).
 *
 * Form for editing an existing facility.
 */

import type { Facility, UpdateFacilityRequest } from '@ppt/api-client';
import { getFacility, updateFacility } from '@ppt/api-client';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate, useParams } from 'react-router-dom';
import { FacilityForm } from '../components';

export function EditFacilityPage() {
  const { t } = useTranslation();
  const { buildingId, facilityId } = useParams<{ buildingId: string; facilityId: string }>();
  const navigate = useNavigate();

  const [facility, setFacility] = useState<Facility | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!buildingId || !facilityId) return;

    const fetchFacility = async () => {
      try {
        const data = await getFacility(buildingId, facilityId);
        setFacility(data);
      } catch (err) {
        setError(t('facilities.errors.loadFailed'));
        console.error('Failed to fetch facility:', err);
      } finally {
        setIsLoading(false);
      }
    };

    fetchFacility();
  }, [buildingId, facilityId, t]);

  const handleSubmit = async (data: UpdateFacilityRequest) => {
    if (!buildingId || !facilityId) return;

    setIsSubmitting(true);
    setError(null);

    try {
      await updateFacility(buildingId, facilityId, data);
      navigate(`/buildings/${buildingId}/facilities`);
    } catch (err) {
      setError(err instanceof Error ? err.message : t('facilities.errors.updateFailed'));
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleCancel = () => {
    navigate(-1);
  };

  if (isLoading) {
    return (
      <div className="max-w-3xl mx-auto px-4 py-8">
        <div className="flex items-center justify-center py-12">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
        </div>
      </div>
    );
  }

  if (!facility) {
    return (
      <div className="max-w-3xl mx-auto px-4 py-8">
        <div className="text-center py-12">
          <p className="text-red-600">{error || t('facilities.notFound')}</p>
          <button
            type="button"
            onClick={() => navigate(-1)}
            className="mt-4 text-blue-600 hover:text-blue-800"
          >
            {t('facilities.goBack')}
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="max-w-3xl mx-auto px-4 py-8">
      <h1 className="text-2xl font-bold text-gray-900 mb-6">{t('facilities.editTitle')}</h1>

      {error && (
        <div className="mb-6 p-4 bg-red-50 border border-red-200 rounded-md text-red-700">
          {error}
        </div>
      )}

      <FacilityForm
        facility={facility}
        onSubmit={handleSubmit}
        onCancel={handleCancel}
        isSubmitting={isSubmitting}
      />
    </div>
  );
}
