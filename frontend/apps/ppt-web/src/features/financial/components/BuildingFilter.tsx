/**
 * BuildingFilter component for filtering financial data by building.
 */

import { useTranslation } from 'react-i18next';

interface Building {
  id: string;
  name: string;
}

interface BuildingFilterProps {
  buildings: Building[];
  selectedBuildingId?: string;
  onChange: (buildingId?: string) => void;
  isLoading?: boolean;
}

export function BuildingFilter({
  buildings,
  selectedBuildingId,
  onChange,
  isLoading,
}: BuildingFilterProps) {
  const { t } = useTranslation();

  if (isLoading) {
    return (
      <div className="w-64">
        <div className="animate-pulse h-10 bg-gray-200 rounded" />
      </div>
    );
  }

  return (
    <div className="flex items-center gap-2">
      <label htmlFor="building-filter" className="text-sm text-gray-600">
        {t('financial.buildingFilter.label')}
      </label>
      <select
        id="building-filter"
        value={selectedBuildingId || ''}
        onChange={(e) => onChange(e.target.value || undefined)}
        className="block w-64 px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm"
      >
        <option value="">{t('financial.buildingFilter.allBuildings')}</option>
        {buildings.map((building) => (
          <option key={building.id} value={building.id}>
            {building.name}
          </option>
        ))}
      </select>
    </div>
  );
}
