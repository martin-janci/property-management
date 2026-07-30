/**
 * AML Dashboard Page (Epic 67, Story 67.1).
 * Epic 90, Story 90.5: Wire up AML dashboard handlers to API.
 *
 * Dashboard for AML risk assessments and compliance monitoring.
 */

import {
  useAmlAssessments,
  useAmlThresholds,
  useCountryRisks,
  useInitiateEdd,
  useReviewAmlAssessment,
} from '@ppt/api-client';
import type React from 'react';
import { useCallback, useState } from 'react';
import type { AmlRiskAssessment } from '../components/AmlRiskAssessmentCard';
import { AmlRiskAssessmentCard } from '../components/AmlRiskAssessmentCard';

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

type AmlReviewDecision = 'approve' | 'reject' | 'escalate';

const AML_REVIEW_DECISIONS: readonly AmlReviewDecision[] = ['approve', 'reject', 'escalate'];

const isAmlReviewDecision = (value: string): value is AmlReviewDecision =>
  (AML_REVIEW_DECISIONS as readonly string[]).includes(value);

export const AmlDashboardPage: React.FC = () => {
  // Filters
  const [statusFilter, setStatusFilter] = useState<string>('');
  const [riskLevelFilter, setRiskLevelFilter] = useState<string>('');
  const [flaggedOnly, setFlaggedOnly] = useState(false);

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

  const handleInitiateEdd = useCallback(
    (assessmentId: string) => {
      // TODO(Phase-2): Replace window.prompt with proper modal form with document selection
      // Phase 1: Basic prompt for collecting reason
      const reason = window.prompt('Enter reason for initiating Enhanced Due Diligence:');
      if (!reason) return;

      initiateEdd.mutate(
        {
          assessment_id: assessmentId,
          reason,
          documents_requested: [],
        },
        {
          onSuccess: () => {
            alert('Enhanced Due Diligence initiated successfully.');
          },
          onError: (err) => {
            console.error('Failed to initiate EDD:', err);
            alert('Failed to initiate EDD. Please try again.');
          },
        }
      );
    },
    [initiateEdd]
  );

  const handleReview = useCallback(
    (assessmentId: string) => {
      // TODO(Phase-2): Replace window.prompt with proper modal form with validation
      // Phase 1: Basic prompts for collecting decision and notes
      const decisionInput = window.prompt('Enter decision (approve, reject, escalate):', 'approve');
      if (!decisionInput) return;

      // Validate the free-text prompt against the allowed decision union before
      // submitting — a typo (e.g. "aprove") must not reach the API as a decision.
      const decision = decisionInput.trim().toLowerCase();
      if (!isAmlReviewDecision(decision)) {
        alert(
          `Invalid decision "${decisionInput}". Please enter one of: approve, reject, escalate.`
        );
        return;
      }

      const notes = window.prompt('Enter review notes:');
      if (!notes) return;

      reviewAssessment.mutate(
        {
          assessmentId,
          request: {
            decision,
            notes,
          },
        },
        {
          onSuccess: () => {
            alert('Assessment reviewed successfully.');
          },
          onError: (err) => {
            console.error('Failed to review assessment:', err);
            alert('Failed to review assessment. Please try again.');
          },
        }
      );
    },
    [reviewAssessment]
  );

  // Loading state
  if (assessmentsLoading) {
    return (
      <div className="aml-dashboard-page">
        <div className="aml-dashboard-header">
          <h1>AML Compliance Dashboard</h1>
          <p>Monitor anti-money laundering risk assessments and compliance status.</p>
        </div>
        <div className="aml-loading">Loading AML data...</div>
      </div>
    );
  }

  // Error state
  if (assessmentsError) {
    return (
      <div className="aml-dashboard-page">
        <div className="aml-dashboard-header">
          <h1>AML Compliance Dashboard</h1>
          <p>Monitor anti-money laundering risk assessments and compliance status.</p>
        </div>
        <div className="aml-dashboard-error" role="alert">
          Failed to load AML data: {assessmentsError.message}
          <button
            type="button"
            onClick={() => window.location.reload()}
            className="aml-retry-button"
          >
            Retry
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="aml-dashboard-page">
      <div className="aml-dashboard-header">
        <h1>AML Compliance Dashboard</h1>
        <p>Monitor anti-money laundering risk assessments and compliance status.</p>
      </div>

      {/* Thresholds Info */}
      {thresholds && (
        <div className="aml-thresholds-section">
          <h2>AML Thresholds</h2>
          <div className="aml-thresholds-grid">
            <div className="aml-threshold-card">
              <div className="aml-threshold-value">
                {thresholds.transaction_threshold_eur.toLocaleString()} EUR
              </div>
              <div className="aml-threshold-label">Transaction Threshold</div>
            </div>
            <div className="aml-threshold-card">
              <div className="aml-threshold-value">
                {thresholds.cumulative_threshold_eur.toLocaleString()} EUR
              </div>
              <div className="aml-threshold-label">Cumulative Threshold</div>
            </div>
            <div className="aml-threshold-card">
              <div className="aml-threshold-value">{thresholds.review_threshold_score}</div>
              <div className="aml-threshold-label">Review Score Threshold</div>
            </div>
          </div>
        </div>
      )}

      {/* Country Risks */}
      <div className="aml-country-risks-section">
        <h2>Country Risk Database</h2>
        <table className="aml-country-risks-table">
          <thead>
            <tr>
              <th>Country</th>
              <th>Risk Rating</th>
              <th>Sanctioned</th>
              <th>FATF Status</th>
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
                <td>{country.is_sanctioned ? 'Yes' : 'No'}</td>
                <td>{country.fatf_status || '-'}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* Filters */}
      <div className="aml-filters-section">
        <h2>Risk Assessments</h2>
        <div className="aml-filters">
          <div className="aml-filter">
            <label htmlFor="statusFilter">Status</label>
            <select
              id="statusFilter"
              value={statusFilter}
              onChange={(e) => setStatusFilter(e.target.value)}
            >
              <option value="">All Statuses</option>
              <option value="pending">Pending</option>
              <option value="in_progress">In Progress</option>
              <option value="completed">Completed</option>
              <option value="requires_review">Requires Review</option>
              <option value="approved">Approved</option>
              <option value="rejected">Rejected</option>
            </select>
          </div>
          <div className="aml-filter">
            <label htmlFor="riskLevelFilter">Risk Level</label>
            <select
              id="riskLevelFilter"
              value={riskLevelFilter}
              onChange={(e) => setRiskLevelFilter(e.target.value)}
            >
              <option value="">All Levels</option>
              <option value="low">Low</option>
              <option value="medium">Medium</option>
              <option value="high">High</option>
              <option value="critical">Critical</option>
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
              Flagged for Review Only
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
          <p>No AML risk assessments found matching the criteria.</p>
          <p>Assessments are automatically created when transactions exceed the threshold.</p>
        </div>
      )}
    </div>
  );
};

AmlDashboardPage.displayName = 'AmlDashboardPage';
