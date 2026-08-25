/**
 * AML Thresholds Section (Epic 67, Story 67.1).
 *
 * Presentational summary of the configured AML monitoring thresholds.
 * Extracted from AmlDashboardPage to keep the page a thin orchestrator.
 */

import type React from 'react';
import { useTranslation } from 'react-i18next';

export interface AmlThresholdsDisplay {
  transaction_threshold_eur: number;
  transaction_threshold_cents: number;
  cumulative_threshold_eur: number;
  review_threshold_score: number;
}

export interface AmlThresholdsSectionProps {
  thresholds: AmlThresholdsDisplay | null;
}

export const AmlThresholdsSection: React.FC<AmlThresholdsSectionProps> = ({ thresholds }) => {
  const { t } = useTranslation();

  if (!thresholds) {
    return null;
  }

  return (
    <div className="aml-thresholds-section">
      <h2>{t('aml.thresholds.title')}</h2>
      <div className="aml-thresholds-grid">
        <div className="aml-threshold-card">
          <div className="aml-threshold-value">
            {thresholds.transaction_threshold_eur.toLocaleString()} EUR
          </div>
          <div className="aml-threshold-label">{t('aml.thresholds.transaction')}</div>
        </div>
        <div className="aml-threshold-card">
          <div className="aml-threshold-value">
            {thresholds.cumulative_threshold_eur.toLocaleString()} EUR
          </div>
          <div className="aml-threshold-label">{t('aml.thresholds.cumulative')}</div>
        </div>
        <div className="aml-threshold-card">
          <div className="aml-threshold-value">{thresholds.review_threshold_score}</div>
          <div className="aml-threshold-label">{t('aml.thresholds.reviewScore')}</div>
        </div>
      </div>
    </div>
  );
};

AmlThresholdsSection.displayName = 'AmlThresholdsSection';
