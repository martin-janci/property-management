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
import { useToast } from '../../../components';
import { AmlRiskAssessmentCard } from '../components/AmlRiskAssessmentCard';
import type { AmlRiskAssessment } from '../components/AmlRiskAssessmentCard';

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

export const AmlDashboardPage: React.FC = () => {
  const { showToast } = useToast();

  // Filters
  const [statusFilter, setStatusFilter] = useState<string>('');
  const [riskLevelFilter, setRiskLevelFilter] = useState<string>('');
  const [flaggedOnly, setFlaggedOnly] = useState(false);

  // Modal state for EDD initiation
  const [eddModal, setEddModal] = useState<{
    assessmentId: string;
    reason: string;
  } | null>(null);

  // Modal state for assessment review
  const [reviewModal, setReviewModal] = useState<{
    assessmentId: string;
    decision: 'approve' | 'reject' | 'escalate';
    notes: string;
  } | null>(null);

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
    // Default values for API fields not yet available
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
    setEddModal({ assessmentId, reason: '' });
  }, []);

  const submitEdd = useCallback(() => {
    if (!eddModal || !eddModal.reason.trim()) return;
    initiateEdd.mutate(
      {
        assessment_id: eddModal.assessmentId,
        reason: eddModal.reason,
        documents_requested: [],
      },
      {
        onSuccess: () => {
          showToast({ type: 'success', title: 'Enhanced Due Diligence initiated' });
          setEddModal(null);
        },
        onError: (err) => {
          showToast({ type: 'error', title: 'Failed to initiate EDD', message: err.message });
        },
      }
    );
  }, [eddModal, initiateEdd, showToast]);

  const handleReview = useCallback((assessmentId: string) => {
    setReviewModal({ assessmentId, decision: 'approve', notes: '' });
  }, []);

  const submitReview = useCallback(() => {
    if (!reviewModal || !reviewModal.notes.trim()) return;
    reviewAssessment.mutate(
      {
        assessmentId: reviewModal.assessmentId,
        request: {
          decision: reviewModal.decision,
          notes: reviewModal.notes,
        },
      },
      {
        onSuccess: () => {
          showToast({ type: 'success', title: 'Assessment reviewed successfully' });
          setReviewModal(null);
        },
        onError: (err) => {
          showToast({ type: 'error', title: 'Failed to review assessment', message: err.message });
        },
      }
    );
  }, [reviewModal, reviewAssessment, showToast]);

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

      {/* EDD Initiation Modal */}
      {eddModal && (
        <div
          className="aml-modal-overlay"
          role="dialog"
          aria-label="Initiate Enhanced Due Diligence"
        >
          <div className="aml-modal">
            <h3>Initiate Enhanced Due Diligence</h3>
            <div className="aml-modal-field">
              <label htmlFor="eddReason">Reason</label>
              <textarea
                id="eddReason"
                value={eddModal.reason}
                onChange={(e) => setEddModal({ ...eddModal, reason: e.target.value })}
                placeholder="Enter reason for initiating EDD..."
                rows={3}
                required
              />
            </div>
            <div className="aml-modal-actions">
              <button type="button" onClick={() => setEddModal(null)}>
                Cancel
              </button>
              <button
                type="button"
                onClick={submitEdd}
                disabled={!eddModal.reason.trim() || initiateEdd.isPending}
              >
                {initiateEdd.isPending ? 'Initiating...' : 'Initiate EDD'}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Assessment Review Modal */}
      {reviewModal && (
        <div className="aml-modal-overlay" role="dialog" aria-label="Review assessment">
          <div className="aml-modal">
            <h3>Review Assessment</h3>
            <div className="aml-modal-field">
              <label htmlFor="reviewDecision">Decision</label>
              <select
                id="reviewDecision"
                value={reviewModal.decision}
                onChange={(e) =>
                  setReviewModal({
                    ...reviewModal,
                    decision: e.target.value as 'approve' | 'reject' | 'escalate',
                  })
                }
              >
                <option value="approve">Approve</option>
                <option value="reject">Reject</option>
                <option value="escalate">Escalate</option>
              </select>
            </div>
            <div className="aml-modal-field">
              <label htmlFor="reviewNotes">Notes</label>
              <textarea
                id="reviewNotes"
                value={reviewModal.notes}
                onChange={(e) => setReviewModal({ ...reviewModal, notes: e.target.value })}
                placeholder="Enter review notes..."
                rows={3}
                required
              />
            </div>
            <div className="aml-modal-actions">
              <button type="button" onClick={() => setReviewModal(null)}>
                Cancel
              </button>
              <button
                type="button"
                onClick={submitReview}
                disabled={!reviewModal.notes.trim() || reviewAssessment.isPending}
              >
                {reviewAssessment.isPending ? 'Submitting...' : 'Submit Review'}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* EDD Initiation Modal */}
      {eddModal && (
        <div className="aml-modal-overlay">
          <div className="aml-modal">
            <h3>Initiate Enhanced Due Diligence</h3>
            <label htmlFor="edd-reason">
              Reason
              <textarea
                id="edd-reason"
                value={eddModal.reason}
                onChange={(e) => setEddModal({ ...eddModal, reason: e.target.value })}
                placeholder="Enter reason for initiating EDD..."
                rows={3}
              />
            </label>
            <div className="aml-modal-actions">
              <button type="button" onClick={() => setEddModal(null)}>
                Cancel
              </button>
              <button
                type="button"
                onClick={submitEdd}
                disabled={!eddModal.reason.trim() || initiateEdd.isPending}
              >
                {initiateEdd.isPending ? 'Submitting...' : 'Initiate EDD'}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Assessment Review Modal */}
      {reviewModal && (
        <div className="aml-modal-overlay">
          <div className="aml-modal">
            <h3>Review Assessment</h3>
            <label htmlFor="review-decision">
              Decision
              <select
                id="review-decision"
                value={reviewModal.decision}
                onChange={(e) =>
                  setReviewModal({
                    ...reviewModal,
                    decision: e.target.value as 'approve' | 'reject' | 'escalate',
                  })
                }
              >
                <option value="approve">Approve</option>
                <option value="reject">Reject</option>
                <option value="escalate">Escalate</option>
              </select>
            </label>
            <label htmlFor="review-notes">
              Notes
              <textarea
                id="review-notes"
                value={reviewModal.notes}
                onChange={(e) => setReviewModal({ ...reviewModal, notes: e.target.value })}
                placeholder="Enter review notes..."
                rows={3}
              />
            </label>
            <div className="aml-modal-actions">
              <button type="button" onClick={() => setReviewModal(null)}>
                Cancel
              </button>
              <button
                type="button"
                onClick={submitReview}
                disabled={!reviewModal.notes.trim() || reviewAssessment.isPending}
              >
                {reviewAssessment.isPending ? 'Submitting...' : 'Submit Review'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

AmlDashboardPage.displayName = 'AmlDashboardPage';
