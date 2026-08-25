/**
 * DecideAppealDialog component - modal dialog for recording an appeal decision
 * on a content moderation case (Epic 67 / Epic 90).
 *
 * Replaces the Phase-1 `window.prompt` flow with an in-app, localized form.
 * Decision semantics are preserved:
 *  - the decision is constrained to the `DecideAppealRequest['decision']` union
 *    (`uphold | overturn`) via a `<select>`, so a free-text typo (e.g. `Uphold`,
 *    `revert`) can no longer be cast into the API payload, and
 *  - a non-empty rationale is required before the mutation is triggered
 *    (matching the old `if (!rationale) return;` guard).
 */

import type { DecideAppealRequest } from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';

type AppealDecision = DecideAppealRequest['decision'];

const APPEAL_DECISIONS: readonly AppealDecision[] = ['uphold', 'overturn'];

interface DecideAppealDialogProps {
  isOpen: boolean;
  isSubmitting?: boolean;
  onSubmit: (decision: AppealDecision, rationale: string) => void;
  onClose: () => void;
}

export function DecideAppealDialog({
  isOpen,
  isSubmitting,
  onSubmit,
  onClose,
}: DecideAppealDialogProps) {
  const { t } = useTranslation();
  const [decision, setDecision] = useState<AppealDecision>('uphold');
  const [rationale, setRationale] = useState('');
  const [showError, setShowError] = useState(false);

  if (!isOpen) return null;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const trimmed = rationale.trim();
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
          aria-labelledby="decide-appeal-dialog-title"
        >
          {/* Header */}
          <div className="px-6 py-4 border-b">
            <h2 id="decide-appeal-dialog-title" className="text-lg font-semibold text-gray-900">
              {t('moderation.dialogs.decideAppeal.title')}
            </h2>
            <p className="text-sm text-gray-500 mt-1">
              {t('moderation.dialogs.decideAppeal.description')}
            </p>
          </div>

          {/* Form */}
          <form onSubmit={handleSubmit} className="px-6 py-4 space-y-4">
            {/* Decision */}
            <div>
              <label
                htmlFor="decide-appeal-decision"
                className="block text-sm font-medium text-gray-700"
              >
                {t('moderation.dialogs.decideAppeal.decisionLabel')} *
              </label>
              <select
                id="decide-appeal-decision"
                value={decision}
                onChange={(e) => setDecision(e.target.value as AppealDecision)}
                className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
              >
                {APPEAL_DECISIONS.map((value) => (
                  <option key={value} value={value}>
                    {t(`moderation.dialogs.decideAppeal.decision.${value}`)}
                  </option>
                ))}
              </select>
            </div>

            {/* Rationale */}
            <div>
              <label
                htmlFor="decide-appeal-rationale"
                className="block text-sm font-medium text-gray-700"
              >
                {t('moderation.dialogs.decideAppeal.rationaleLabel')} *
              </label>
              <textarea
                id="decide-appeal-rationale"
                value={rationale}
                onChange={(e) => {
                  setRationale(e.target.value);
                  if (showError) setShowError(false);
                }}
                rows={4}
                placeholder={t('moderation.dialogs.decideAppeal.rationalePlaceholder')}
                className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
                aria-invalid={showError}
                aria-describedby={showError ? 'decide-appeal-rationale-error' : undefined}
              />
              {showError && (
                <p
                  id="decide-appeal-rationale-error"
                  className="mt-1 text-sm text-red-600"
                  role="alert"
                >
                  {t('moderation.dialogs.decideAppeal.rationaleRequired')}
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
              {isSubmitting
                ? t('moderation.dialogs.decideAppeal.submitting')
                : t('moderation.dialogs.decideAppeal.submit')}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

DecideAppealDialog.displayName = 'DecideAppealDialog';
