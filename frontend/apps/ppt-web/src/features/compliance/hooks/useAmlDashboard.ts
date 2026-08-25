/**
 * useAmlDashboard — data + decision-flow logic for the AML dashboard.
 *
 * Epic 67, Story 67.1 / Epic 90, Story 90.5.
 * Extracted from AmlDashboardPage so the page is a thin orchestrator: this hook
 * owns the API queries, the API→display transforms, the EDD/review mutations,
 * and the modal-dialog state (which assessment each dialog acts on). Behavior is
 * unchanged from the original inline implementation.
 */

import {
  type ReviewAmlAssessmentRequest,
  useAmlAssessments,
  useAmlThresholds,
  useCountryRisks,
  useInitiateEdd,
  useReviewAmlAssessment,
} from '@ppt/api-client';
import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useToast } from '../../../components';
import type { AmlCountryRisksTableProps } from '../components/AmlCountryRisksTable';
import type { AmlRiskAssessment } from '../components/AmlRiskAssessmentCard';
import type { AmlThresholdsDisplay } from '../components/AmlThresholdsSection';

type CountryRiskDisplay = AmlCountryRisksTableProps['countryRisks'][number];

export type AmlReviewDecision = ReviewAmlAssessmentRequest['decision'];

export interface UseAmlDashboardResult {
  // Filters
  statusFilter: string;
  setStatusFilter: (value: string) => void;
  riskLevelFilter: string;
  setRiskLevelFilter: (value: string) => void;
  flaggedOnly: boolean;
  setFlaggedOnly: (value: boolean) => void;

  // Data
  assessments: AmlRiskAssessment[];
  thresholds: AmlThresholdsDisplay | null;
  countryRisks: CountryRiskDisplay[];
  assessmentsLoading: boolean;
  assessmentsError: Error | null;

  // Dialog state
  eddAssessmentId: string | null;
  reviewAssessmentId: string | null;
  eddPending: boolean;
  reviewPending: boolean;

  // Handlers
  handleInitiateEdd: (assessmentId: string) => void;
  handleReview: (assessmentId: string) => void;
  closeEdd: () => void;
  closeReview: () => void;
  submitEdd: (reason: string) => void;
  submitReview: (decision: AmlReviewDecision, notes: string) => void;
}

export function useAmlDashboard(): UseAmlDashboardResult {
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

  const closeEdd = useCallback(() => {
    setEddAssessmentId(null);
  }, []);

  const closeReview = useCallback(() => {
    setReviewAssessmentId(null);
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

  return {
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
    eddPending: initiateEdd.isPending,
    reviewPending: reviewAssessment.isPending,
    handleInitiateEdd,
    handleReview,
    closeEdd,
    closeReview,
    submitEdd,
    submitReview,
  };
}
