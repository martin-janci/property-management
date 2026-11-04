/**
 * Building picker for the voting pages (Epic 5). Votes are scoped to one
 * building; this lets managers of several buildings (and residents of more
 * than one) pick which building they are viewing or creating a vote for.
 */
import { useId } from 'react';
import { useTranslation } from 'react-i18next';
import type { VotingBuildingOption } from '../hooks';

interface VotingBuildingSelectProps {
  options: VotingBuildingOption[];
  value: string;
  onChange: (buildingId: string) => void;
  disabled?: boolean;
}

export function VotingBuildingSelect({
  options,
  value,
  onChange,
  disabled,
}: VotingBuildingSelectProps) {
  const { t } = useTranslation();
  const id = useId();

  if (options.length === 0) return null;

  return (
    <div className="flex flex-wrap items-center gap-2">
      <label htmlFor={id} className="text-sm font-medium text-gray-700">
        {t('voting.buildingSelect.label')}
      </label>
      <select
        id={id}
        value={value}
        disabled={disabled || options.length === 1}
        onChange={(e) => onChange(e.target.value)}
        className="block w-full max-w-xs rounded-md border border-gray-300 px-3 py-2 text-sm shadow-sm focus:border-blue-500 focus:outline-none focus:ring-blue-500 disabled:bg-gray-50 disabled:text-gray-600"
      >
        {options.map((o) => (
          <option key={o.id} value={o.id}>
            {o.isMine ? t('voting.buildingSelect.mine', { building: o.label }) : o.label}
          </option>
        ))}
      </select>
    </div>
  );
}
