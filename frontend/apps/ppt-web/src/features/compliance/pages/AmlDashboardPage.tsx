/**
 * AML Dashboard Page (Epic 67, Story 67.1).
 * Epic 90, Story 90.5: Wire up AML dashboard handlers to API.
 * Epic 90: replace the Phase-1 window.prompt/window.alert EDD + review decision
 * flow with in-app modal dialogs + toast feedback, and localize all copy.
 *
 * Dashboard for AML risk assessments and compliance monitoring.
 *
 * The page is a thin orchestrator: data + decision-flow logic lives in the
 * `useAmlDashboard` hook, and the thresholds / country-risks / filters sections
 * are presentational sub-components under ../components.
 */

import type React from 'react';
import { useTranslation } from 'react-i18next';
import {
  AmlCountryRisksTable,
  AmlFiltersPanel,
  AmlRiskAssessmentCard,
  AmlThresholdsSection,
  InitiateEddDialog,
  ReviewAssessmentDialog,
} from '../components';
import { useAmlDashboard } from '../hooks/useAmlDashboard';

export const AmlDashboardPage: React.FC = () => {
  const { t } = useTranslation();
  const {
    statusFilter,
    setStatusFilter,
    riskLevelFilter,
    setRiskLevelFilter,
    flaggedOnly,
    setFlaggedOnly,
    assessments,
    thresholds,
    countryRisks,
    assessmentsLoading,
    assessmentsError,
    eddAssessmentId,
    reviewAssessmentId,
    eddPending,
    reviewPending,
    handleInitiateEdd,
    handleReview,
    closeEdd,
    closeReview,
    submitEdd,
    submitReview,
  } = useAmlDashboard();

  // Loading state
  if (assessmentsLoading) {
    return (
      <div className="aml-dashboard-page">
        <div className="aml-dashboard-header">
          <h1>{t('aml.dashboard.title')}</h1>
          <p>{t('aml.dashboard.subtitle')}</p>
        </div>
        <div className="aml-loading">{t('aml.dashboard.loading')}</div>
      </div>
    );
  }

  // Error state
  if (assessmentsError) {
    return (
      <div className="aml-dashboard-page">
        <div className="aml-dashboard-header">
          <h1>{t('aml.dashboard.title')}</h1>
          <p>{t('aml.dashboard.subtitle')}</p>
        </div>
        <div className="aml-dashboard-error" role="alert">
          {t('aml.dashboard.loadError', { message: assessmentsError.message })}
          <button
            type="button"
            onClick={() => window.location.reload()}
            className="aml-retry-button"
          >
            {t('aml.dashboard.retry')}
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="aml-dashboard-page">
      <div className="aml-dashboard-header">
        <h1>{t('aml.dashboard.title')}</h1>
        <p>{t('aml.dashboard.subtitle')}</p>
      </div>

      {/* Thresholds Info */}
      <AmlThresholdsSection thresholds={thresholds} />

      {/* Country Risks */}
      <AmlCountryRisksTable countryRisks={countryRisks} />

      {/* Filters */}
      <AmlFiltersPanel
        statusFilter={statusFilter}
        onStatusFilterChange={setStatusFilter}
        riskLevelFilter={riskLevelFilter}
        onRiskLevelFilterChange={setRiskLevelFilter}
        flaggedOnly={flaggedOnly}
        onFlaggedOnlyChange={setFlaggedOnly}
      />

      {/* Assessments List */}
      {assessments.length > 0 ? (
        <div className="aml-assessments-list">
          {assessments.map((assessment) => (
            <AmlRiskAssessmentCard
              key={assessment.id}
              assessment={assessment}
              onInitiateEdd={handleInitiateEdd}
              onReview={handleReview}
              showActions={true}
            />
          ))}
        </div>
      ) : (
        <div className="aml-empty-state">
          <p>{t('aml.emptyState.none')}</p>
          <p>{t('aml.emptyState.hint')}</p>
        </div>
      )}

      {/* Decision dialogs (replace the Phase-1 window.prompt/alert flow).
       *
       * The `key` binds each dialog's identity to the assessment it acts on so
       * React remounts a fresh instance per assessment (and on close → reopen).
       * Without it the dialogs stay mounted — they only render `null` while
       * closed — and their internal reason/notes/decision state would leak from
       * one assessment into the next one opened (#2832). The `-closed` sentinels
       * force a remount across the closed state so a same-assessment reopen also
       * starts blank. */}
      <InitiateEddDialog
        key={eddAssessmentId ?? 'edd-closed'}
        isOpen={eddAssessmentId !== null}
        isSubmitting={eddPending}
        onSubmit={submitEdd}
        onClose={closeEdd}
      />
      <ReviewAssessmentDialog
        key={reviewAssessmentId ?? 'review-closed'}
        isOpen={reviewAssessmentId !== null}
        isSubmitting={reviewPending}
        onSubmit={submitReview}
        onClose={closeReview}
      />
    </div>
  );
};

AmlDashboardPage.displayName = 'AmlDashboardPage';
