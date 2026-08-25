/**
 * AML Filters Panel (Epic 67, Story 67.1).
 *
 * Presentational filter controls (status / risk level / flagged-only) for the
 * AML assessments list. State lives in the page/hook; this is a controlled view.
 * Extracted from AmlDashboardPage to keep the page a thin orchestrator.
 */

import type React from 'react';
import { useTranslation } from 'react-i18next';

export interface AmlFiltersPanelProps {
  statusFilter: string;
  onStatusFilterChange: (value: string) => void;
  riskLevelFilter: string;
  onRiskLevelFilterChange: (value: string) => void;
  flaggedOnly: boolean;
  onFlaggedOnlyChange: (value: boolean) => void;
}

export const AmlFiltersPanel: React.FC<AmlFiltersPanelProps> = ({
  statusFilter,
  onStatusFilterChange,
  riskLevelFilter,
  onRiskLevelFilterChange,
  flaggedOnly,
  onFlaggedOnlyChange,
}) => {
  const { t } = useTranslation();

  return (
    <div className="aml-filters-section">
      <h2>{t('aml.filters.title')}</h2>
      <div className="aml-filters">
        <div className="aml-filter">
          <label htmlFor="statusFilter">{t('aml.filters.status')}</label>
          <select
            id="statusFilter"
            value={statusFilter}
            onChange={(e) => onStatusFilterChange(e.target.value)}
          >
            <option value="">{t('aml.filters.allStatuses')}</option>
            <option value="pending">{t('aml.status.pending')}</option>
            <option value="in_progress">{t('aml.status.in_progress')}</option>
            <option value="completed">{t('aml.status.completed')}</option>
            <option value="requires_review">{t('aml.status.requires_review')}</option>
            <option value="approved">{t('aml.status.approved')}</option>
            <option value="rejected">{t('aml.status.rejected')}</option>
          </select>
        </div>
        <div className="aml-filter">
          <label htmlFor="riskLevelFilter">{t('aml.filters.riskLevel')}</label>
          <select
            id="riskLevelFilter"
            value={riskLevelFilter}
            onChange={(e) => onRiskLevelFilterChange(e.target.value)}
          >
            <option value="">{t('aml.filters.allLevels')}</option>
            <option value="low">{t('aml.riskLevel.low')}</option>
            <option value="medium">{t('aml.riskLevel.medium')}</option>
            <option value="high">{t('aml.riskLevel.high')}</option>
            <option value="critical">{t('aml.riskLevel.critical')}</option>
          </select>
        </div>
        <div className="aml-filter checkbox">
          <label htmlFor="flaggedOnly">
            <input
              type="checkbox"
              id="flaggedOnly"
              checked={flaggedOnly}
              onChange={(e) => onFlaggedOnlyChange(e.target.checked)}
            />
            {t('aml.filters.flaggedOnly')}
          </label>
        </div>
      </div>
    </div>
  );
};

AmlFiltersPanel.displayName = 'AmlFiltersPanel';
