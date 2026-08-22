/**
 * InitiateEddDialog component - modal dialog for initiating Enhanced Due
 * Diligence on an AML risk assessment (Epic 67 / Epic 90).
 *
 * Replaces the Phase-1 `window.prompt` flow with an in-app, localized form.
 * Decision semantics are preserved: a non-empty reason is required before the
 * mutation is triggered (an empty/blank reason is a no-op, matching the old
 * `if (!reason) return;` guard).
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';

interface InitiateEddDialogProps {
  isOpen: boolean;
  isSubmitting?: boolean;
  onSubmit: (reason: string) => void;
  onClose: () => void;
}

export function InitiateEddDialog({
  isOpen,
  isSubmitting,
  onSubmit,
  onClose,
}: InitiateEddDialogProps) {
  const { t } = useTranslation();
  const [reason, setReason] = useState('');
  const [showError, setShowError] = useState(false);

  if (!isOpen) return null;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const trimmed = reason.trim();
    if (!trimmed) {
      setShowError(true);
      return;
    }
    onSubmit(trimmed);
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
          aria-labelledby="edd-dialog-title"
        >
          {/* Header */}
          <div className="px-6 py-4 border-b">
            <h2 id="edd-dialog-title" className="text-lg font-semibold text-gray-900">
              {t('aml.edd.title')}
            </h2>
            <p className="text-sm text-gray-500 mt-1">{t('aml.edd.description')}</p>
          </div>

          {/* Form */}
          <form onSubmit={handleSubmit} className="px-6 py-4 space-y-4">
            <div>
              <label htmlFor="edd-reason" className="block text-sm font-medium text-gray-700">
                {t('aml.edd.reasonLabel')} *
              </label>
              <textarea
                id="edd-reason"
                value={reason}
                onChange={(e) => {
                  setReason(e.target.value);
                  if (showError) setShowError(false);
                }}
                rows={4}
                placeholder={t('aml.edd.reasonPlaceholder')}
                className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
                aria-invalid={showError}
                aria-describedby={showError ? 'edd-reason-error' : undefined}
              />
              {showError && (
                <p id="edd-reason-error" className="mt-1 text-sm text-red-600" role="alert">
                  {t('aml.edd.reasonRequired')}
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
              {isSubmitting ? t('aml.edd.submitting') : t('aml.edd.submit')}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

InitiateEddDialog.displayName = 'InitiateEddDialog';
