import {
  type DisputeType as ApiDisputeType,
  uploadEvidence as apiUploadEvidence,
  useCreateDispute,
} from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { AuthRequiredGate, useToast } from '../../../components';
import { useAuth } from '../../../contexts';
import { FileDisputePage, type FileDisputeSubmitPayload } from './FileDisputePage';

/**
 * Route wrapper for file dispute page (Epic 80, Story 80.2).
 *
 * Step sequence:
 *  1. POST /api/v1/disputes/organizations/:orgId  → creates the dispute (returns id)
 *  2. For each valid PendingEvidence file: POST /api/v1/disputes/:id/evidence
 *  3. Navigate to /disputes/:id (detail page) on success.
 *
 * FileDisputePage is a pure presentational component; all API side-effects live here.
 */
export function FileDisputePageRoute() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { user } = useAuth();
  const { showToast } = useToast();
  const [isSubmitting, setIsSubmitting] = useState(false);

  const organizationId = user?.organizationId ?? '';

  const createDispute = useCreateDispute(organizationId);

  if (!user?.organizationId) {
    return <AuthRequiredGate />;
  }

  const handleSubmit = async (payload: FileDisputeSubmitPayload) => {
    setIsSubmitting(true);
    try {
      // Step 1 — create the dispute
      const created = await createDispute.mutateAsync({
        type: payload.values.type as ApiDisputeType,
        subject: payload.values.subject,
        description: payload.values.description,
        unitId: payload.values.unitId,
        respondentId: payload.values.respondentId || undefined,
      });

      // Step 2 — upload evidence files sequentially; skip errored entries.
      // We call the raw API function (not the hook) because the hook must be
      // keyed to a disputeId at render time and we only know the id after step 1.
      const validFiles = payload.evidence.filter((e) => e.status !== 'error');
      const failedEvidence: typeof validFiles = [];
      for (const item of validFiles) {
        try {
          await apiUploadEvidence(created.id, item.file, item.description || item.file.name);
        } catch {
          failedEvidence.push(item);
        }
      }
      const evidenceErrors = failedEvidence.length;

      // Step 3 — toast + navigate to the new dispute detail
      if (evidenceErrors > 0) {
        showToast({
          type: 'warning',
          title: t('disputes.filedWithEvidenceErrors', 'Dispute filed (some files failed)'),
          message: t('disputes.evidenceUploadErrorsMsg', { count: evidenceErrors }),
        });
      } else {
        showToast({
          type: 'success',
          title: t('disputes.filedSuccessfully', 'Dispute filed successfully'),
          message: t('disputes.submittedMsg', 'Your dispute has been submitted for review.'),
        });
      }
      // TODO(#627): consume failedEvidence on DisputeDetailPage to surface a retry
      // prompt once evidence-retry UI lands. For now we just thread it through
      // router state so the detail page can pick it up when ready.
      navigate(`/disputes/${created.id}`, {
        state: evidenceErrors > 0 ? { failedEvidence } : undefined,
      });
    } catch (error) {
      showToast({
        type: 'error',
        title: t('disputes.failedToFile', 'Failed to file dispute'),
        message: error instanceof Error ? error.message : t('auth.unexpectedError'),
      });
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <FileDisputePage
      onSubmit={handleSubmit}
      isSubmitting={isSubmitting || createDispute.isPending}
    />
  );
}
