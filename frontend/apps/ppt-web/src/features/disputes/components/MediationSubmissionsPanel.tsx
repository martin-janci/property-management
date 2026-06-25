/**
 * MediationSubmissionsPanel — lists party submissions for a dispute and, for
 * parties to the dispute, provides a composer to file a new submission.
 *
 * Wires the previously-unwired submissions endpoints
 * (`GET/POST /api/v1/disputes/{id}/submissions`) via useDisputeSubmissions +
 * useSubmitResponse.
 *
 * Epic 80: Dispute Resolution (Story 80.3)
 */

import { useDisputeSubmissions, useSubmitResponse } from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Spinner } from '../../../components/Spinner';

/** Submission types the backend accepts (free-form string column, curated here). */
const SUBMISSION_TYPES = ['statement', 'evidence', 'response', 'request'] as const;
type SubmissionType = (typeof SUBMISSION_TYPES)[number];

const submissionTypeLabelKeys: Record<SubmissionType, string> = {
  statement: 'disputes.mediation.submissions.typeStatement',
  evidence: 'disputes.mediation.submissions.typeEvidence',
  response: 'disputes.mediation.submissions.typeResponse',
  request: 'disputes.mediation.submissions.typeRequest',
};

export interface MediationSubmissionsPanelProps {
  disputeId: string;
  /** Whether the current user is a party (filer/respondent) and may submit. */
  canSubmit: boolean;
  onToastSuccess: (title: string, message?: string) => void;
  onToastError: (title: string, message?: string) => void;
}

export function MediationSubmissionsPanel({
  disputeId,
  canSubmit,
  onToastSuccess,
  onToastError,
}: MediationSubmissionsPanelProps) {
  const { t } = useTranslation();
  const { data: submissions = [], isLoading } = useDisputeSubmissions(disputeId);
  const submitResponse = useSubmitResponse(disputeId);

  const [submissionType, setSubmissionType] = useState<SubmissionType>('statement');
  const [content, setContent] = useState('');
  const [isVisibleToAll, setIsVisibleToAll] = useState(true);

  const formatDate = (iso: string) =>
    new Date(iso).toLocaleString(undefined, {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });

  const submissionTypeLabel = (type: string) => {
    const key = submissionTypeLabelKeys[type as SubmissionType];
    return key ? t(key) : type;
  };

  const handleSubmit = async () => {
    const trimmed = content.trim();
    if (!trimmed) return;
    try {
      await submitResponse.mutateAsync({
        submission_type: submissionType,
        content: trimmed,
        is_visible_to_all: isVisibleToAll,
      });
      onToastSuccess(t('disputes.mediation.submissions.submittedSuccess'));
      setContent('');
      setSubmissionType('statement');
      setIsVisibleToAll(true);
    } catch (err) {
      onToastError(
        t('disputes.mediation.submissions.submitError'),
        err instanceof Error ? err.message : undefined
      );
    }
  };

  return (
    <div>
      <h2 className="text-base font-semibold text-gray-900 mb-1">
        {t('disputes.mediation.submissions.title')} ({submissions.length})
      </h2>
      <p className="text-sm text-gray-500 mb-4">{t('disputes.mediation.submissions.subtitle')}</p>

      {isLoading ? (
        <div className="flex justify-center py-8">
          <Spinner size="md" />
        </div>
      ) : submissions.length === 0 ? (
        <p className="text-sm text-gray-400 py-4">{t('disputes.mediation.submissions.empty')}</p>
      ) : (
        <ul className="space-y-3">
          {submissions.map((submission) => (
            <li key={submission.id} className="p-3 border border-gray-100 rounded-lg bg-gray-50">
              <div className="flex items-center justify-between gap-2 mb-1.5">
                <span className="px-2 py-0.5 text-xs font-medium rounded bg-violet-100 text-violet-800">
                  {submissionTypeLabel(submission.submission_type)}
                </span>
                <span className="text-xs text-gray-400">{formatDate(submission.created_at)}</span>
              </div>
              <p className="text-sm text-gray-700 whitespace-pre-wrap">{submission.content}</p>
              {!submission.is_visible_to_all && (
                <p className="text-xs text-amber-600 mt-1.5">
                  {t('disputes.mediation.submissions.privateNote')}
                </p>
              )}
            </li>
          ))}
        </ul>
      )}

      {canSubmit && (
        <div className="mt-6 pt-6 border-t border-gray-100">
          <h3 className="text-sm font-semibold text-gray-900 mb-3">
            {t('disputes.mediation.submissions.composerTitle')}
          </h3>
          <div className="space-y-3">
            <div>
              <label
                htmlFor="submission-type"
                className="block text-xs font-medium text-gray-600 mb-1"
              >
                {t('disputes.mediation.submissions.typeLabel')}
              </label>
              <select
                id="submission-type"
                value={submissionType}
                onChange={(e) => setSubmissionType(e.target.value as SubmissionType)}
                className="w-full px-3 py-2 text-sm border border-gray-300 rounded-lg focus:ring-2 focus:ring-violet-500 focus:border-violet-500"
              >
                {SUBMISSION_TYPES.map((type) => (
                  <option key={type} value={type}>
                    {t(submissionTypeLabelKeys[type])}
                  </option>
                ))}
              </select>
            </div>

            <div>
              <label
                htmlFor="submission-content"
                className="block text-xs font-medium text-gray-600 mb-1"
              >
                {t('disputes.mediation.submissions.contentLabel')}
              </label>
              <textarea
                id="submission-content"
                value={content}
                onChange={(e) => setContent(e.target.value)}
                rows={4}
                placeholder={t('disputes.mediation.submissions.contentPlaceholder')}
                className="w-full px-3 py-2 text-sm border border-gray-300 rounded-lg focus:ring-2 focus:ring-violet-500 focus:border-violet-500"
              />
            </div>

            <label className="flex items-center gap-2 text-sm text-gray-700">
              <input
                type="checkbox"
                checked={isVisibleToAll}
                onChange={(e) => setIsVisibleToAll(e.target.checked)}
                className="rounded border-gray-300 text-violet-600 focus:ring-violet-500"
              />
              {t('disputes.mediation.submissions.visibleToAll')}
            </label>

            <div className="flex justify-end">
              <button
                type="button"
                onClick={handleSubmit}
                disabled={!content.trim() || submitResponse.isPending}
                className="px-4 py-2 text-sm bg-violet-600 text-white rounded-lg hover:bg-violet-700 font-medium disabled:opacity-60 disabled:cursor-not-allowed"
              >
                {submitResponse.isPending
                  ? t('disputes.mediation.submissions.submitting')
                  : t('disputes.mediation.submissions.submit')}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
