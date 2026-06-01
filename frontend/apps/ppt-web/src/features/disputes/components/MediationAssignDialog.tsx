import { useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useFocusTrap } from '../../../hooks/useFocusTrap';

export interface MediationAssignDialogProps {
  onConfirm: (mediatorId: string) => void;
  onCancel: () => void;
  isSubmitting: boolean;
}

export function MediationAssignDialog({
  onConfirm,
  onCancel,
  isSubmitting,
}: MediationAssignDialogProps) {
  const { t } = useTranslation();
  const [mediatorId, setMediatorId] = useState('');
  const dialogRef = useRef<HTMLDivElement>(null);
  useFocusTrap(dialogRef, onCancel);

  return (
    <div
      ref={dialogRef}
      role="dialog"
      aria-modal="true"
      aria-labelledby="assign-dialog-title"
      tabIndex={-1}
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 px-4"
    >
      <div className="bg-white rounded-xl shadow-xl w-full max-w-md p-6">
        <h2 id="assign-dialog-title" className="text-lg font-semibold text-gray-900 mb-2">
          {t('disputes.mediation.assignMediatorTitle')}
        </h2>
        <p className="text-sm text-gray-500 mb-4">
          {t('disputes.mediation.assignMediatorDescription')}
        </p>

        <label
          htmlFor="assign-dialog-mediator-id"
          className="block text-sm font-medium text-gray-700 mb-1"
        >
          {t('disputes.mediation.mediatorUserId')}
        </label>
        <input
          id="assign-dialog-mediator-id"
          type="text"
          value={mediatorId}
          onChange={(e) => setMediatorId(e.target.value)}
          placeholder={t('disputes.mediation.userPickerNote')}
          className="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-violet-500 focus:border-transparent"
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
            onClick={() => onConfirm(mediatorId)}
            disabled={!mediatorId.trim() || isSubmitting}
            className="px-4 py-2 text-sm bg-violet-600 text-white rounded-lg hover:bg-violet-700 disabled:opacity-50 disabled:cursor-not-allowed font-medium"
          >
            {isSubmitting ? t('disputes.mediation.assigning') : t('disputes.mediation.assign')}
          </button>
        </div>
      </div>
    </div>
  );
}
