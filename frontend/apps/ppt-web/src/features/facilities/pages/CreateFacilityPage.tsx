/**
 * Create Facility Page (Epic 56: Facility Booking).
 *
 * Form for creating a new facility.
 */

import type { CreateFacilityRequest, UpdateFacilityRequest } from '@ppt/api-client';
import { createFacility } from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate, useParams } from 'react-router-dom';
import { FacilityForm } from '../components';

export function CreateFacilityPage() {
  const { t } = useTranslation();
  const { buildingId } = useParams<{ buildingId: string }>();
  const navigate = useNavigate();

  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (data: CreateFacilityRequest | UpdateFacilityRequest) => {
    if (!buildingId) return;

    setIsSubmitting(true);
    setError(null);

    try {
      // For create, we need to cast to CreateFacilityRequest
      await createFacility(buildingId, data as CreateFacilityRequest);
      navigate(`/buildings/${buildingId}/facilities`);
    } catch (err) {
      setError(err instanceof Error ? err.message : t('facilities.errors.createFailed'));
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleCancel = () => {
    navigate(-1);
  };

  return (
    <div className="max-w-3xl mx-auto px-4 py-8">
      <h1 className="text-2xl font-bold text-gray-900 mb-6">{t('facilities.createTitle')}</h1>

      {error && (
        <div className="mb-6 p-4 bg-red-50 border border-red-200 rounded-md text-red-700">
          {error}
        </div>
      )}

      <FacilityForm onSubmit={handleSubmit} onCancel={handleCancel} isSubmitting={isSubmitting} />
    </div>
  );
}
