/**
 * AML Dashboard Page (Epic 67, Story 67.1).
 * Epic 90, Story 90.5: Wire up AML dashboard handlers to API.
 * Epic 90: replace the Phase-1 window.prompt/window.alert EDD + review decision
 * flow with in-app modal dialogs + toast feedback, and localize all copy.
 *
 * Dashboard for AML risk assessments and compliance monitoring.
 */

import {
  type ReviewAmlAssessmentRequest,
  useAmlAssessments,
  useAmlThresholds,
  useCountryRisks,
  useInitiateEdd,
  useReviewAmlAssessment,
} from '@ppt/api-client';
import type React from 'react';
import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useToast } from '../../../components';
import type { AmlRiskAssessment } from '../components/AmlRiskAssessmentCard';
import { AmlRiskAssessmentCard } from '../components/AmlRiskAssessmentCard';
import { InitiateEddDialog } from '../components/InitiateEddDialog';
import { ReviewAssessmentDialog } from '../components/ReviewAssessmentDialog';

interface AmlThresholdsDisplay {
  transaction_threshold_eur: number;
  transaction_threshold_cents: number;
  cumulative_threshold_eur: number;
  review_threshold_score: number;
}

interface CountryRiskDisplay {
  country_code: string;
  country_name: string;
  risk_rating: string;
  is_sanctioned: boolean;
  fatf_status?: string;
}

type AmlReviewDecision = ReviewAmlAssessmentRequest['decision'];

export const AmlDashboardPage: React.FC = () => {
  const { t } = useTranslation();
  const { showToast } = useToast();

  // Filters
  const [statusFilter, setStatusFilter] = useState<string>('');
  const [riskLevelFilter, setRiskLevelFilter] = useState<string>('');
  const [flaggedOnly, setFlaggedOnly] = useState(false);

  // Decision dialogs: which assessment (if any) each dialog is acting on.
  const [eddAssessmentId, setEddAssessmentId] = useState<string | null>(null);
  const [reviewAssessmentId, setReviewAssessmentId] = useState<string | null>(null);

  // Fetch assessments from API
  const {
    data: assessmentsData,
    isLoading: assessmentsLoading,
    error: assessmentsError,
  } = useAmlAssessments({
    status: statusFilter || undefined,
    risk_level: riskLevelFilter || undefined,
    flagged_only: flaggedOnly || undefined,
  });

  // Fetch thresholds from API
  const { data: thresholdsData } = useAmlThresholds();

  // Fetch country risks from API
  const { data: countryRisksData } = useCountryRisks();

  // Mutations
  const initiateEdd = useInitiateEdd();
  const reviewAssessment = useReviewAmlAssessment();

  // Transform API data to component types
  const assessments: AmlRiskAssessment[] = (assessmentsData?.assessments ?? []).map((a) => ({
    id: a.id,
    party_id: a.subject_id,
    party_type: a.subject_type,
    risk_score: a.risk_score,
    risk_level: a.risk_level,
    status: a.status,
    risk_factors: a.risk_factors.map((f) => ({
      factor_type: f.factor_type,
      description: f.description,
      weight: f.weight,
      mitigated: !f.triggered,
    })),
    flagged_for_review: a.flagged_for_review,
    // TODO(Phase-2): Extend API to include these fields
    // Phase 1: Default values for missing fields
    id_verified: false,
    source_of_funds_documented: false,
    pep_check_completed: false,
    sanctions_check_completed: false,
    recommendations: [],
    created_at: a.created_at,
    assessed_at: a.updated_at,
  }));

  const thresholds: AmlThresholdsDisplay | null = thresholdsData?.thresholds
    ? {
        transaction_threshold_eur: thresholdsData.thresholds.transaction_threshold_eur,
        transaction_threshold_cents: thresholdsData.thresholds.transaction_threshold_cents,
        cumulative_threshold_eur: thresholdsData.thresholds.cumulative_threshold_eur,
        review_threshold_score: thresholdsData.thresholds.review_threshold_score,
      }
    : null;

  const countryRisks: CountryRiskDisplay[] = (countryRisksData?.countries ?? []).map((c) => ({
    country_code: c.country_code,
    country_name: c.country_name,
    risk_rating: c.risk_rating,
    is_sanctioned: c.is_sanctioned,
    fatf_status: c.fatf_status,
  }));

  const handleInitiateEdd = useCallback((assessmentId: string) => {
    setEddAssessmentId(assessmentId);
  }, []);

  const handleReview = useCallback((assessmentId: string) => {
    setReviewAssessmentId(assessmentId);
  }, []);

  const submitEdd = useCallback(
    (reason: string) => {
      if (!eddAssessmentId) return;
      initiateEdd.mutate(
        {
          assessment_id: eddAssessmentId,
          reason,
          documents_requested: [],
        },
        {
          onSuccess: () => {
            setEddAssessmentId(null);
            showToast({
              type: 'success',
              title: t('aml.edd.successTitle'),
              message: t('aml.edd.successMessage'),
            });
          },
          onError: (err) => {
            console.error('Failed to initiate EDD:', err);
            showToast({
              type: 'error',
              title: t('aml.edd.errorTitle'),
              message: t('aml.edd.errorMessage'),
            });
          },
        }
      );
    },
    [eddAssessmentId, initiateEdd, showToast, t]
  );

  const submitReview = useCallback(
    (decision: AmlReviewDecision, notes: string) => {
      if (!reviewAssessmentId) return;
      reviewAssessment.mutate(
        {
          assessmentId: reviewAssessmentId,
          request: { decision, notes },
        },
        {
          onSuccess: () => {
            setReviewAssessmentId(null);
            showToast({
              type: 'success',
              title: t('aml.review.successTitle'),
              message: t('aml.review.successMessage'),
            });
          },
          onError: (err) => {
            console.error('Failed to review assessment:', err);
            showToast({
              type: 'error',
              title: t('aml.review.errorTitle'),
              message: t('aml.review.errorMessage'),
            });
          },
        }
      );
    },
    [reviewAssessmentId, reviewAssessment, showToast, t]
  );

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
      {thresholds && (
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
      )}

      {/* Country Risks */}
      <div className="aml-country-risks-section">
        <h2>{t('aml.countryRisks.title')}</h2>
        <table className="aml-country-risks-table">
          <thead>
            <tr>
              <th>{t('aml.countryRisks.country')}</th>
              <th>{t('aml.countryRisks.riskRating')}</th>
              <th>{t('aml.countryRisks.sanctioned')}</th>
              <th>{t('aml.countryRisks.fatfStatus')}</th>
            </tr>
          </thead>
          <tbody>
            {countryRisks.map((country) => (
              <tr key={country.country_code} className={`risk-${country.risk_rating}`}>
                <td>
                  {country.country_code} - {country.country_name}
                </td>
                <td>
                  <span className={`risk-badge ${country.risk_rating}`}>
                    {country.risk_rating.toUpperCase()}
                  </span>
                </td>
                <td>{country.is_sanctioned ? t('common.yes') : t('common.no')}</td>
                <td>{country.fatf_status || '-'}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* Filters */}
      <div className="aml-filters-section">
        <h2>{t('aml.filters.title')}</h2>
        <div className="aml-filters">
          <div className="aml-filter">
            <label htmlFor="statusFilter">{t('aml.filters.status')}</label>
            <select
              id="statusFilter"
              value={statusFilter}
              onChange={(e) => setStatusFilter(e.target.value)}
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
              onChange={(e) => setRiskLevelFilter(e.target.value)}
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
                onChange={(e) => setFlaggedOnly(e.target.checked)}
              />
              {t('aml.filters.flaggedOnly')}
            </label>
          </div>
        </div>
      </div>

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

      {/* Decision dialogs (replace the Phase-1 window.prompt/alert flow) */}
      <InitiateEddDialog
        isOpen={eddAssessmentId !== null}
        isSubmitting={initiateEdd.isPending}
        onSubmit={submitEdd}
        onClose={() => setEddAssessmentId(null)}
      />
      <ReviewAssessmentDialog
        isOpen={reviewAssessmentId !== null}
        isSubmitting={reviewAssessment.isPending}
        onSubmit={submitReview}
        onClose={() => setReviewAssessmentId(null)}
      />
    </div>
  );
};

AmlDashboardPage.displayName = 'AmlDashboardPage';
