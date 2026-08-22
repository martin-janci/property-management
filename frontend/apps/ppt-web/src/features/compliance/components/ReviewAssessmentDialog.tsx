/**
 * ReviewAssessmentDialog component - modal dialog for recording a compliance
 * decision on an AML risk assessment (Epic 67 / Epic 90).
 *
 * Replaces the Phase-1 `window.prompt` flow with an in-app, localized form.
 * Decision semantics are preserved:
 *  - the decision is constrained to the `approve | reject | escalate` union via a
 *    `<select>` (a free-text typo can no longer reach the API), and
 *  - non-empty review notes are required before the mutation is triggered
 *    (matching the old `if (!notes) return;` guard).
 */

import type { ReviewAmlAssessmentRequest } from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';

type AmlReviewDecision = ReviewAmlAssessmentRequest['decision'];

const AML_REVIEW_DECISIONS: readonly AmlReviewDecision[] = ['approve', 'reject', 'escalate'];

interface ReviewAssessmentDialogProps {
  isOpen: boolean;
  isSubmitting?: boolean;
  onSubmit: (decision: AmlReviewDecision, notes: string) => void;
  onClose: () => void;
}

export function ReviewAssessmentDialog({
  isOpen,
  isSubmitting,
  onSubmit,
  onClose,
}: ReviewAssessmentDialogProps) {
  const { t } = useTranslation();
  const [decision, setDecision] = useState<AmlReviewDecision>('approve');
  const [notes, setNotes] = useState('');
  const [showError, setShowError] = useState(false);

  if (!isOpen) return null;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const trimmed = notes.trim();
    if (!trimmed) {
      setShowError(true);
      return;
    }
    onSubmit(decision, trimmed);
  };

  return (
    <div className="fixed inset-0 z-50 overflow-y-auto">
      {/* Backdrop */}
      <button
        type="button"
        className="fixed inset-0 bg-black bg-opacity-50 transition-opacity cursor-default"
        onClick={onClose}
        onKeyDown={(e) => e.key === 'Escape' && onClose()}
        aria-label={t('aria.closeDialog')}
      />

      {/* Dialog */}
      <div className="flex min-h-full items-center justify-center p-4">
        <div
          className="relative w-full max-w-md bg-white rounded-lg shadow-xl"
          role="dialog"
          aria-modal="true"
          aria-labelledby="review-dialog-title"
        >
          {/* Header */}
          <div className="px-6 py-4 border-b">
            <h2 id="review-dialog-title" className="text-lg font-semibold text-gray-900">
              {t('aml.review.title')}
            </h2>
            <p className="text-sm text-gray-500 mt-1">{t('aml.review.description')}</p>
          </div>

          {/* Form */}
          <form onSubmit={handleSubmit} className="px-6 py-4 space-y-4">
            {/* Decision */}
            <div>
              <label htmlFor="review-decision" className="block text-sm font-medium text-gray-700">
                {t('aml.review.decisionLabel')} *
              </label>
              <select
                id="review-decision"
                value={decision}
                onChange={(e) => setDecision(e.target.value as AmlReviewDecision)}
                className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
              >
                {AML_REVIEW_DECISIONS.map((value) => (
                  <option key={value} value={value}>
                    {t(`aml.review.decision.${value}`)}
                  </option>
                ))}
              </select>
            </div>

            {/* Notes */}
            <div>
              <label htmlFor="review-notes" className="block text-sm font-medium text-gray-700">
                {t('aml.review.notesLabel')} *
              </label>
              <textarea
                id="review-notes"
                value={notes}
                onChange={(e) => {
                  setNotes(e.target.value);
                  if (showError) setShowError(false);
                }}
                rows={4}
                placeholder={t('aml.review.notesPlaceholder')}
                className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
                aria-invalid={showError}
                aria-describedby={showError ? 'review-notes-error' : undefined}
              />
              {showError && (
                <p id="review-notes-error" className="mt-1 text-sm text-red-600" role="alert">
                  {t('aml.review.notesRequired')}
                </p>
              )}
            </div>
          </form>

          {/* Actions */}
          <div className="px-6 py-4 border-t flex justify-end gap-3">
            <button
              type="button"
              onClick={onClose}
              disabled={isSubmitting}
              className="px-4 py-2 text-gray-700 border border-gray-300 rounded-lg hover:bg-gray-50 disabled:opacity-50"
            >
              {t('common.cancel')}
            </button>
            <button
              type="button"
              onClick={handleSubmit}
              disabled={isSubmitting}
              className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 flex items-center gap-2"
            >
              {isSubmitting && (
                <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-white" />
              )}
              {isSubmitting ? t('aml.review.submitting') : t('aml.review.submit')}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

ReviewAssessmentDialog.displayName = 'ReviewAssessmentDialog';
