/**
 * Filter controls for the action queue.
 * Allows filtering by type, priority, and search.
 *
 * @module features/dashboard/components/ActionFilters
 */

import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { ActionPriority, ActionQueueFilters, ActionType } from '../hooks/useActionQueue';
import './ActionFilters.css';

interface ActionFiltersProps {
  filters: ActionQueueFilters;
  onFilterChange: (filters: ActionQueueFilters) => void;
  stats: {
    total: number;
    urgent: number;
    high: number;
    medium: number;
    low: number;
  };
  userRole: 'manager' | 'resident';
}

const managerActionTypes: ActionType[] = [
  'fault_pending',
  'fault_escalated',
  'approval_pending',
  'vote_active',
  'message_unread',
];

const residentActionTypes: ActionType[] = [
  'vote_active',
  'meter_due',
  'person_months_due',
  'announcement_unread',
];

const priorities: ActionPriority[] = ['urgent', 'high', 'medium', 'low'];

export function ActionFilters({ filters, onFilterChange, stats, userRole }: ActionFiltersProps) {
  const { t } = useTranslation();
  const [searchValue, setSearchValue] = useState(filters.search ?? '');

  const actionTypes = userRole === 'manager' ? managerActionTypes : residentActionTypes;

  const handleTypeToggle = useCallback(
    (type: ActionType) => {
      const currentTypes = filters.types ?? [];
      const newTypes = currentTypes.includes(type)
        ? currentTypes.filter((t) => t !== type)
        : [...currentTypes, type];

      onFilterChange({
        ...filters,
        types: newTypes.length > 0 ? newTypes : undefined,
      });
    },
    [filters, onFilterChange]
  );

  const handlePriorityToggle = useCallback(
    (priority: ActionPriority) => {
      const currentPriorities = filters.priorities ?? [];
      const newPriorities = currentPriorities.includes(priority)
        ? currentPriorities.filter((p) => p !== priority)
        : [...currentPriorities, priority];

      onFilterChange({
        ...filters,
        priorities: newPriorities.length > 0 ? newPriorities : undefined,
      });
    },
    [filters, onFilterChange]
  );

  const handleSearchSubmit = useCallback(() => {
    onFilterChange({
      ...filters,
      search: searchValue.trim() || undefined,
    });
  }, [filters, searchValue, onFilterChange]);

  const handleClearFilters = useCallback(() => {
    setSearchValue('');
    onFilterChange({});
  }, [onFilterChange]);

  const hasActiveFilters =
    (filters.types?.length ?? 0) > 0 || (filters.priorities?.length ?? 0) > 0 || !!filters.search;

  return (
    <div className="action-filters">
      <div>
        <label htmlFor="action-search" className="action-filters__label">
          {t('dashboard.searchActions')}
        </label>
        <div className="action-filters__search-row">
          <input
            id="action-search"
            type="text"
            value={searchValue}
            onChange={(e) => setSearchValue(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') handleSearchSubmit();
            }}
            placeholder={t('dashboard.searchPlaceholder')}
            className="action-filters__search-input"
          />
          <button type="button" onClick={handleSearchSubmit} className="action-filters__search-btn">
            {t('common.search')}
          </button>
        </div>
      </div>

      <div>
        <span className="action-filters__label">{t('dashboard.filterByPriority')}</span>
        <div className="action-filters__chips">
          {priorities.map((priority) => {
            const isActive = filters.priorities?.includes(priority);
            const count = stats[priority];
            return (
              <button
                key={priority}
                type="button"
                onClick={() => handlePriorityToggle(priority)}
                className={`action-filters__chip action-filters__chip--${priority}${isActive ? ` action-filters__chip--active-${priority}` : ''}`}
                aria-pressed={isActive}
              >
                {t(`dashboard.priority.${priority}`)}
                {count > 0 && <span className="action-filters__chip-count">{count}</span>}
              </button>
            );
          })}
        </div>
      </div>

      <div>
        <span className="action-filters__label">{t('dashboard.filterByType')}</span>
        <div className="action-filters__chips">
          {actionTypes.map((type) => {
            const isActive = filters.types?.includes(type);
            return (
              <button
                key={type}
                type="button"
                onClick={() => handleTypeToggle(type)}
                className={`action-filters__chip${isActive ? ' action-filters__chip--type-active' : ' action-filters__chip--type'}`}
                aria-pressed={isActive}
              >
                {t(`dashboard.actionType.${type}`)}
              </button>
            );
          })}
        </div>
      </div>

      {hasActiveFilters && (
        <div className="action-filters__clear-row">
          <button type="button" onClick={handleClearFilters} className="action-filters__clear-btn">
            {t('dashboard.clearFilters')}
          </button>
        </div>
      )}
    </div>
  );
}
