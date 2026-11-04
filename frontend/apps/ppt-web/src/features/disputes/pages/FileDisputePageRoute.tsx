import { type DisputeCategory, useCreateDispute, useMyUnits } from '@ppt/api-client';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { AuthRequiredGate, useToast } from '../../../components';
import { Spinner } from '../../../components/Spinner';
import { useAuth } from '../../../contexts';
import {
  type DisputeTypeValue,
  FileDisputePage,
  type FileDisputeSubmitPayload,
} from './FileDisputePage';

/** Form dispute type → backend `dispute_category`. Only `lease` differs. */
const TYPE_TO_CATEGORY: Record<DisputeTypeValue, DisputeCategory> = {
  noise: 'noise',
  damage: 'damage',
  payment: 'payment',
  lease: 'lease_terms',
  maintenance: 'maintenance',
  other: 'other',
};

/**
 * Route wrapper for file dispute page (Epic 80, Story 80.2).
 *
 *  1. Loads the caller's units (`GET /api/v1/users/me/units`) for the unit
 *     selector; a caller without units (e.g. a manager) files without one.
 *  2. `POST /api/v1/disputes` — the organization and filer come from the JWT.
 *  3. Navigates to the new dispute's detail page.
 *
 * FileDisputePage is a pure presentational component; all API side-effects live here.
 */
export function FileDisputePageRoute() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { user } = useAuth();
  const { showToast } = useToast();

  const organizationId = user?.organizationId ?? '';
  const createDispute = useCreateDispute(organizationId);
  const { data: myUnits, isLoading: unitsLoading } = useMyUnits();

  if (!user?.organizationId) {
    return <AuthRequiredGate />;
  }

  // The form fixes its validation schema (unit required or not) on mount, so
  // wait for the unit list before rendering it.
  if (unitsLoading) {
    return (
      <div className="flex justify-center py-12">
        <Spinner size="lg" color="blue" />
      </div>
    );
  }

  const units = (myUnits ?? [])
    .filter((u) => u.association.isActive)
    .map((u) => ({
      id: u.unit.id,
      buildingId: u.unit.buildingId,
      label: `${u.building.name || u.building.street} – ${u.unit.designation}`,
    }));

  const handleSubmit = async ({ values }: FileDisputeSubmitPayload) => {
    try {
      const unit = units.find((u) => u.id === values.unitId);
      const created = await createDispute.mutateAsync({
        category: TYPE_TO_CATEGORY[values.type],
        title: values.subject,
        description: values.description,
        unitId: unit?.id,
        buildingId: unit?.buildingId,
        respondentIds: values.respondentId ? [values.respondentId] : [],
      });

      showToast({
        type: 'success',
        title: t('disputes.filedSuccessfully', 'Dispute filed successfully'),
        message: t('disputes.submittedMsg', 'Your dispute has been submitted for review.'),
      });
      navigate(`/disputes/${created.id}`);
    } catch (error) {
      showToast({
        type: 'error',
        title: t('disputes.failedToFile', 'Failed to file dispute'),
        message: error instanceof Error ? error.message : t('auth.unexpectedError'),
      });
      // Re-throw so FileDisputePage keeps the auto-saved draft (AC-4): a failed
      // filing must not wipe the user's in-progress work.
      throw error;
    }
  };

  return (
    <FileDisputePage units={units} onSubmit={handleSubmit} isSubmitting={createDispute.isPending} />
  );
}
