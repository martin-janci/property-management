/**
 * EditFaultPage - page for editing an existing fault (before triage).
 * Epic 4: Fault Reporting & Resolution (UC-03.1)
 */

import { useTranslation } from 'react-i18next';
import { FaultForm, type FaultFormData } from '../components/FaultForm';

interface EditFaultPageProps {
  faultId: string;
  initialData: Partial<FaultFormData>;
  buildings: Array<{ id: string; name: string }>;
  units: Array<{ id: string; designation: string }>;
  isSubmitting?: boolean;
  onSubmit: (data: FaultFormData) => void;
  onCancel: () => void;
}

export function EditFaultPage({
  faultId: _faultId,
  initialData,
  buildings,
  units,
  isSubmitting,
  onSubmit,
  onCancel,
}: EditFaultPageProps) {
  const { t } = useTranslation();

  return (
    <div className="max-w-2xl mx-auto px-4 py-8">
      <div className="mb-6">
        <button
          type="button"
          onClick={onCancel}
          className="text-sm text-blue-600 hover:text-blue-800 flex items-center gap-1 mb-4"
        >
          ← {t('faults.edit.backToFault')}
        </button>
        <h1 className="text-2xl font-bold text-gray-900">{t('faults.edit.title')}</h1>
        <p className="text-gray-600 mt-2">{t('faults.edit.subtitle')}</p>
      </div>

      <div className="bg-white rounded-lg shadow p-6">
        <FaultForm
          initialData={initialData}
          buildings={buildings}
          units={units}
          isSubmitting={isSubmitting}
          onSubmit={onSubmit}
          onCancel={onCancel}
        />
      </div>
    </div>
  );
}
