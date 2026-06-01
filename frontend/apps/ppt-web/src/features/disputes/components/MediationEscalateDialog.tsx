import { useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useFocusTrap } from '../../../hooks/useFocusTrap';

export interface MediationEscalateDialogProps {
  onConfirm: (reason: string) => void;
  onCancel: () => void;
  isSubmitting: boolean;
}

export function MediationEscalateDialog({
  onConfirm,
  onCancel,
  isSubmitting,
}: MediationEscalateDialogProps) {
  const { t } = useTranslation();
  const [reason, setReason] = useState('');
  const dialogRef = useRef<HTMLDivElement>(null);
  useFocusTrap(dialogRef, onCancel);

  return (
    <div
      ref={dialogRef}
      role="dialog"
      aria-modal="true"
      aria-labelledby="escalate-dialog-title"
      tabIndex={-1}
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 px-4"
    >
      <div className="bg-white rounded-xl shadow-xl w-full max-w-md p-6">
        <h2 id="escalate-dialog-title" className="text-lg font-semibold text-gray-900 mb-2">
          {t('disputes.mediation.escalateTitle')}
        </h2>
        <p className="text-sm text-gray-500 mb-4">{t('disputes.mediation.escalateDescription')}</p>

        <label
          htmlFor="escalate-dialog-reason"
          className="block text-sm font-medium text-gray-700 mb-1"
        >
          {t('disputes.mediation.reasonForEscalation')}
        </label>
        <textarea
          id="escalate-dialog-reason"
          value={reason}
          onChange={(e) => setReason(e.target.value)}
          rows={4}
          placeholder={t('disputes.mediation.reasonPlaceholder')}
          className="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-red-500 focus:border-transparent resize-none"
        />

        <div className="flex justify-end gap-3 mt-4">
          <button
            type="button"
            onClick={onCancel}
            disabled={isSubmitting}
            className="px-4 py-2 text-sm border border-gray-300 rounded-lg hover:bg-gray-50 disabled:opacity-50"
          >
            {t('disputes.mediation.cancel')}
          </button>
          <button
            type="button"
            onClick={() => onConfirm(reason)}
            disabled={!reason.trim() || isSubmitting}
            className="px-4 py-2 text-sm bg-red-600 text-white rounded-lg hover:bg-red-700 disabled:opacity-50 disabled:cursor-not-allowed font-medium"
          >
            {isSubmitting
              ? t('disputes.mediation.escalatingBtn')
              : t('disputes.mediation.escalateBtn')}
          </button>
        </div>
      </div>
    </div>
  );
}
