import type { JSX } from 'react';
import { useTranslation } from 'react-i18next';
import type { Equipment } from '../types';

const STATUS_CLASSES: Record<string, string> = {
  operational: 'bg-green-100 text-green-700',
  needs_maintenance: 'bg-yellow-100 text-yellow-700',
  under_repair: 'bg-orange-100 text-orange-700',
  decommissioned: 'bg-gray-100 text-gray-600',
};

interface EquipmentListProps {
  equipment: Equipment[];
  isLoading: boolean;
  selectedId: string | null;
  onSelect: (id: string) => void;
}

export function EquipmentList({ equipment, isLoading, selectedId, onSelect }: EquipmentListProps): JSX.Element {
  const { t } = useTranslation();

  return (
    <div className="rounded-lg border bg-white shadow-sm">
      <div className="border-b px-4 py-3">
        <h2 className="text-base font-semibold text-gray-900">
          {t('predictive.equipmentTitle', { defaultValue: 'Equipment' })}
        </h2>
      </div>
      {isLoading ? (
        <div className="space-y-2 p-3">
          {[1, 2, 3, 4].map((i) => (
            <div key={i} className="h-10 animate-pulse rounded bg-gray-100" />
          ))}
        </div>
      ) : equipment.length === 0 ? (
        <p className="p-4 text-center text-sm text-gray-500">
          {t('predictive.noEquipment', { defaultValue: 'No equipment found.' })}
        </p>
      ) : (
        <ul className="divide-y divide-gray-100">
          {equipment.map((e) => (
            <li key={e.id}>
              <button
                type="button"
                onClick={() => onSelect(e.id)}
                className={`w-full px-4 py-3 text-left hover:bg-gray-50 ${selectedId === e.id ? 'bg-indigo-50' : ''}`}
              >
                <div className="flex items-center justify-between gap-2">
                  <span className="truncate text-sm font-medium text-gray-900">{e.name}</span>
                  <span
                    className={`shrink-0 rounded-full px-2 py-0.5 text-xs font-medium ${STATUS_CLASSES[e.status] ?? 'bg-gray-100 text-gray-600'}`}
                  >
                    {e.status.replace(/_/g, ' ')}
                  </span>
                </div>
                <p className="mt-0.5 text-xs text-gray-500">{e.category}</p>
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
