/**
 * TakeModerationActionDialog component - modal dialog for recording a moderation
 * action on a content case (Epic 67 / Epic 90).
 *
 * Replaces the Phase-1 `window.prompt` flow with an in-app, localized form.
 * Action semantics are preserved:
 *  - the action type is constrained to the `TakeModerationActionRequest['action_type']`
 *    union (`approve | warn | restrict | remove`) via a `<select>`, so a free-text
 *    typo (e.g. `aprove`) can no longer be cast into the API payload, and
 *  - a non-empty rationale is required before the mutation is triggered
 *    (matching the old `if (!rationale) return;` guard).
 */

import type { TakeModerationActionRequest } from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';

type ModerationActionType = TakeModerationActionRequest['action_type'];

const MODERATION_ACTION_TYPES: readonly ModerationActionType[] = [
  'approve',
  'warn',
  'restrict',
  'remove',
];

interface TakeModerationActionDialogProps {
  isOpen: boolean;
  isSubmitting?: boolean;
  onSubmit: (actionType: ModerationActionType, rationale: string, notifyOwner: boolean) => void;
  onClose: () => void;
}

export function TakeModerationActionDialog({
  isOpen,
  isSubmitting,
  onSubmit,
  onClose,
}: TakeModerationActionDialogProps) {
  const { t } = useTranslation();
  const [actionType, setActionType] = useState<ModerationActionType>('approve');
  const [rationale, setRationale] = useState('');
  const [notifyOwner, setNotifyOwner] = useState(true);
  const [showError, setShowError] = useState(false);

  if (!isOpen) return null;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const trimmed = rationale.trim();
    if (!trimmed) {
      setShowError(true);
      return;
    }
    onSubmit(actionType, trimmed, notifyOwner);
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
          aria-labelledby="take-action-dialog-title"
        >
          {/* Header */}
          <div className="px-6 py-4 border-b">
            <h2 id="take-action-dialog-title" className="text-lg font-semibold text-gray-900">
              {t('moderation.dialogs.takeAction.title')}
            </h2>
            <p className="text-sm text-gray-500 mt-1">
              {t('moderation.dialogs.takeAction.description')}
            </p>
          </div>

          {/* Form */}
          <form onSubmit={handleSubmit} className="px-6 py-4 space-y-4">
            {/* Action type */}
            <div>
              <label htmlFor="take-action-type" className="block text-sm font-medium text-gray-700">
                {t('moderation.dialogs.takeAction.actionLabel')} *
              </label>
              <select
                id="take-action-type"
                value={actionType}
                onChange={(e) => setActionType(e.target.value as ModerationActionType)}
                className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
              >
                {MODERATION_ACTION_TYPES.map((value) => (
                  <option key={value} value={value}>
                    {t(`moderation.actionType.${value}`)}
                  </option>
                ))}
              </select>
            </div>

            {/* Rationale */}
            <div>
              <label
                htmlFor="take-action-rationale"
                className="block text-sm font-medium text-gray-700"
              >
                {t('moderation.dialogs.takeAction.rationaleLabel')} *
              </label>
              <textarea
                id="take-action-rationale"
                value={rationale}
                onChange={(e) => {
                  setRationale(e.target.value);
                  if (showError) setShowError(false);
                }}
                rows={4}
                placeholder={t('moderation.dialogs.takeAction.rationalePlaceholder')}
                className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
                aria-invalid={showError}
                aria-describedby={showError ? 'take-action-rationale-error' : undefined}
              />
              {showError && (
                <p
                  id="take-action-rationale-error"
                  className="mt-1 text-sm text-red-600"
                  role="alert"
                >
                  {t('moderation.dialogs.takeAction.rationaleRequired')}
                </p>
              )}
            </div>

            {/* Notify owner */}
            <div className="flex items-center gap-2">
              <input
                type="checkbox"
                id="take-action-notify-owner"
                checked={notifyOwner}
                onChange={(e) => setNotifyOwner(e.target.checked)}
                className="rounded border-gray-300"
              />
              <label htmlFor="take-action-notify-owner" className="text-sm text-gray-700">
                {t('moderation.dialogs.takeAction.notifyOwnerLabel')}
              </label>
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
                ? t('moderation.dialogs.takeAction.submitting')
                : t('moderation.dialogs.takeAction.submit')}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

TakeModerationActionDialog.displayName = 'TakeModerationActionDialog';
