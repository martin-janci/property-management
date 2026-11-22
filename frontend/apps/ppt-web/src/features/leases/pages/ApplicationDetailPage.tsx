/**
 * ApplicationDetailPage - Review application with screening.
 * Epic 19: Lease Management & Tenant Screening
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { getActiveLocale } from '../../../lib/locale';
import type {
  ApplicationStatus,
  LeaseApplication,
  ReviewApplicationData,
  ScreeningStatus,
  ScreeningType,
  TenantScreening,
} from '../types';

interface ApplicationDetailPageProps {
  application: LeaseApplication;
  screenings: TenantScreening[];
  unitInfo: {
    number: string;
    buildingName: string;
  };
  isLoading?: boolean;
  isSubmitting?: boolean;
  onBack: () => void;
  onInitiateScreening: (types: ScreeningType[]) => void;
  onReview: (data: ReviewApplicationData) => void;
  onCreateLease: (applicationId: string) => void;
}

const statusColors: Record<ApplicationStatus, string> = {
  draft: 'bg-gray-100 text-gray-800',
  submitted: 'bg-blue-100 text-blue-800',
  under_review: 'bg-yellow-100 text-yellow-800',
  screening: 'bg-purple-100 text-purple-800',
  approved: 'bg-green-100 text-green-800',
  rejected: 'bg-red-100 text-red-800',
  withdrawn: 'bg-gray-100 text-gray-800',
};

const screeningStatusColors: Record<ScreeningStatus, string> = {
  pending: 'bg-gray-100 text-gray-800',
  in_progress: 'bg-blue-100 text-blue-800',
  completed: 'bg-green-100 text-green-800',
  failed: 'bg-red-100 text-red-800',
};

const SCREENING_TYPES: ScreeningType[] = [
  'credit_check',
  'background_check',
  'employment_verification',
  'rental_history',
  'income_verification',
];

export function ApplicationDetailPage({
  application,
  screenings,
  unitInfo,
  isLoading,
  isSubmitting,
  onBack,
  onInitiateScreening,
  onReview,
  onCreateLease,
}: ApplicationDetailPageProps) {
  const { t } = useTranslation();
  const [showReviewForm, setShowReviewForm] = useState(false);
  const [reviewNotes, setReviewNotes] = useState('');
  const [selectedScreeningTypes, setSelectedScreeningTypes] = useState<ScreeningType[]>([]);

  const formatDate = (dateString?: string) => {
    if (!dateString) return '-';
    return new Date(dateString).toLocaleDateString(getActiveLocale());
  };

  const formatCurrency = (amount?: number) => {
    if (!amount) return '-';
    return new Intl.NumberFormat(getActiveLocale(), {
      style: 'currency',
      currency: 'EUR',
    }).format(amount);
  };

  const canReview =
    application.status === 'submitted' ||
    application.status === 'under_review' ||
    application.status === 'screening';

  const canInitiateScreening =
    application.status === 'submitted' || application.status === 'under_review';

  const allScreeningsCompleted =
    screenings.length > 0 && screenings.every((s) => s.status === 'completed');

  const handleToggleScreeningType = (type: ScreeningType) => {
    setSelectedScreeningTypes((prev) =>
      prev.includes(type) ? prev.filter((t) => t !== type) : [...prev, type]
    );
  };

  const handleInitiateScreening = () => {
    if (selectedScreeningTypes.length > 0) {
      onInitiateScreening(selectedScreeningTypes);
      setSelectedScreeningTypes([]);
    }
  };

  const handleApprove = () => {
    onReview({ decision: 'approve', notes: reviewNotes || undefined });
    setShowReviewForm(false);
  };

  const handleReject = () => {
    onReview({ decision: 'reject', notes: reviewNotes || undefined });
    setShowReviewForm(false);
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-12">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
      </div>
    );
  }

  return (
    <div className="max-w-4xl mx-auto px-4 py-8">
      {/* Back Button */}
      <button
        type="button"
        onClick={onBack}
        className="flex items-center gap-2 text-gray-600 hover:text-gray-800 mb-6"
      >
        <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
        </svg>
        {t('leases.applications.backToApplications')}
      </button>

      {/* Header */}
      <div className="bg-white rounded-lg shadow p-6 mb-6">
        <div className="flex items-start justify-between">
          <div>
            <div className="flex items-center gap-3">
              <h1 className="text-2xl font-bold text-gray-900">{application.applicantName}</h1>
              <span
                className={`px-3 py-1 text-sm font-medium rounded-full ${statusColors[application.status]}`}
              >
                {t(`leases.applications.status.${application.status}`)}
              </span>
            </div>
            <p className="text-gray-600 mt-1">{application.applicantEmail}</p>
            {application.applicantPhone && (
              <p className="text-sm text-gray-500">{application.applicantPhone}</p>
            )}
            <p className="text-sm text-gray-500 mt-2">
              {t('leases.applications.applyingFor')}: {unitInfo.buildingName} - {unitInfo.number}
            </p>
          </div>
          <div className="flex gap-2">
            {application.status === 'approved' && (
              <button
                type="button"
                onClick={() => onCreateLease(application.id)}
                className="px-4 py-2 text-sm font-medium text-white bg-green-600 hover:bg-green-700 rounded-md"
              >
                {t('leases.applications.createLease')}
              </button>
            )}
            {canReview && !showReviewForm && (
              <button
                type="button"
                onClick={() => setShowReviewForm(true)}
                className="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded-md"
              >
                {t('leases.applications.review')}
              </button>
            )}
          </div>
        </div>
      </div>

      {/* Review Form */}
      {showReviewForm && (
        <div className="bg-white rounded-lg shadow p-6 mb-6 border-l-4 border-blue-500">
          <h2 className="text-lg font-semibold text-gray-900 mb-4">
            {t('leases.applications.reviewApplication')}
          </h2>
          <div className="mb-4">
            <label htmlFor="reviewNotes" className="block text-sm font-medium text-gray-700 mb-1">
              {t('leases.applications.reviewNotes')}
            </label>
            <textarea
              id="reviewNotes"
              value={reviewNotes}
              onChange={(e) => setReviewNotes(e.target.value)}
              rows={3}
              placeholder={t('leases.applications.reviewNotesPlaceholder')}
              className="w-full border border-gray-300 rounded-md px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
          </div>
          <div className="flex gap-3">
            <button
              type="button"
              onClick={handleApprove}
              disabled={isSubmitting}
              className="px-4 py-2 text-sm font-medium text-white bg-green-600 hover:bg-green-700 rounded-md disabled:opacity-50"
            >
              {t('leases.applications.approve')}
            </button>
            <button
              type="button"
              onClick={handleReject}
              disabled={isSubmitting}
              className="px-4 py-2 text-sm font-medium text-white bg-red-600 hover:bg-red-700 rounded-md disabled:opacity-50"
            >
              {t('leases.applications.reject')}
            </button>
            <button
              type="button"
              onClick={() => setShowReviewForm(false)}
              className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50"
            >
              {t('common.cancel')}
            </button>
          </div>
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Application Details */}
        <div className="bg-white rounded-lg shadow p-6">
          <h2 className="text-lg font-semibold text-gray-900 mb-4">
            {t('leases.applications.details')}
          </h2>
          <div className="space-y-4">
            <div>
              <p className="text-sm text-gray-500">{t('leases.applications.currentAddress')}</p>
              <p className="font-medium">{application.currentAddress || '-'}</p>
            </div>
            <div>
              <p className="text-sm text-gray-500">{t('leases.applications.employer')}</p>
              <p className="font-medium">{application.employerName || '-'}</p>
              {application.employerPhone && (
                <p className="text-sm text-gray-500">{application.employerPhone}</p>
              )}
            </div>
            <div>
              <p className="text-sm text-gray-500">{t('leases.applications.annualIncome')}</p>
              <p className="font-medium">{formatCurrency(application.annualIncome)}</p>
            </div>
            <div>
              <p className="text-sm text-gray-500">{t('leases.applications.moveInDate')}</p>
              <p className="font-medium">{formatDate(application.moveInDate)}</p>
            </div>
            <div>
              <p className="text-sm text-gray-500">{t('leases.applications.occupants')}</p>
              <p className="font-medium">{application.numberOfOccupants || '-'}</p>
            </div>
            <div>
              <p className="text-sm text-gray-500">{t('leases.applications.pets')}</p>
              <p className="font-medium">
                {application.hasPets ? application.petDetails || t('common.yes') : t('common.no')}
              </p>
            </div>
            {application.references && (
              <div>
                <p className="text-sm text-gray-500">{t('leases.applications.references')}</p>
                <p className="font-medium whitespace-pre-wrap">{application.references}</p>
              </div>
            )}
            {application.notes && (
              <div>
                <p className="text-sm text-gray-500">{t('leases.applications.applicantNotes')}</p>
                <p className="font-medium whitespace-pre-wrap">{application.notes}</p>
              </div>
            )}
          </div>
        </div>

        {/* Screening Section */}
        <div className="bg-white rounded-lg shadow p-6">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-lg font-semibold text-gray-900">
              {t('leases.applications.screening')}
            </h2>
            {allScreeningsCompleted && (
              <span className="px-2 py-1 text-xs font-medium bg-green-100 text-green-800 rounded">
                {t('leases.applications.screeningComplete')}
              </span>
            )}
          </div>

          {/* Initiate Screening */}
          {canInitiateScreening && (
            <div className="mb-6 p-4 bg-gray-50 rounded-lg">
              <p className="text-sm font-medium text-gray-700 mb-3">
                {t('leases.applications.initiateScreening')}
              </p>
              <div className="space-y-2 mb-4">
                {SCREENING_TYPES.map((type) => {
                  const alreadyInitiated = screenings.some((s) => s.screeningType === type);
                  return (
                    <label
                      key={type}
                      className={`flex items-center gap-2 ${alreadyInitiated ? 'opacity-50' : ''}`}
                    >
                      <input
                        type="checkbox"
                        checked={selectedScreeningTypes.includes(type)}
                        onChange={() => handleToggleScreeningType(type)}
                        disabled={alreadyInitiated}
                        className="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
                      />
                      <span className="text-sm text-gray-700">
                        {t(`leases.applications.screeningTypes.${type}`)}
                      </span>
                      {alreadyInitiated && (
                        <span className="text-xs text-gray-500">
                          ({t('leases.applications.alreadyInitiated')})
                        </span>
                      )}
                    </label>
                  );
                })}
              </div>
              <button
                type="button"
                onClick={handleInitiateScreening}
                disabled={selectedScreeningTypes.length === 0 || isSubmitting}
                className="px-4 py-2 text-sm font-medium text-white bg-purple-600 hover:bg-purple-700 rounded-md disabled:opacity-50"
              >
                {t('leases.applications.startScreening')}
              </button>
            </div>
          )}

          {/* Screening Results */}
          {screenings.length > 0 ? (
            <div className="space-y-3">
              {screenings.map((screening) => (
                <div key={screening.id} className="border rounded-lg p-4">
                  <div className="flex items-start justify-between">
                    <div>
                      <p className="font-medium text-gray-900">
                        {t(`leases.applications.screeningTypes.${screening.screeningType}`)}
                      </p>
                      <p className="text-sm text-gray-500 mt-1">
                        {t('leases.applications.requestedOn')}: {formatDate(screening.requestedAt)}
                      </p>
                      {screening.completedAt && (
                        <p className="text-sm text-gray-500">
                          {t('leases.applications.completedOn')}:{' '}
                          {formatDate(screening.completedAt)}
                        </p>
                      )}
                    </div>
                    <span
                      className={`px-2 py-1 text-xs font-medium rounded ${screeningStatusColors[screening.status]}`}
                    >
                      {t(`leases.applications.screeningStatus.${screening.status}`)}
                    </span>
                  </div>
                  {screening.result && (
                    <div className="mt-3 pt-3 border-t">
                      <div className="flex items-center gap-2">
                        <span
                          className={`px-2 py-1 text-xs font-medium rounded ${
                            screening.result === 'pass'
                              ? 'bg-green-100 text-green-800'
                              : screening.result === 'fail'
                                ? 'bg-red-100 text-red-800'
                                : 'bg-yellow-100 text-yellow-800'
                          }`}
                        >
                          {t(`leases.applications.screeningResult.${screening.result}`)}
                        </span>
                        {screening.score !== undefined && (
                          <span className="text-sm text-gray-600">
                            {t('leases.applications.score')}: {screening.score}
                          </span>
                        )}
                      </div>
                      {screening.details && (
                        <p className="text-sm text-gray-600 mt-2">{screening.details}</p>
                      )}
                    </div>
                  )}
                </div>
              ))}
            </div>
          ) : (
            <p className="text-gray-500 text-center py-4">
              {t('leases.applications.noScreenings')}
            </p>
          )}
        </div>
      </div>

      {/* Review History */}
      {application.reviewedAt && (
        <div className="mt-6 bg-white rounded-lg shadow p-6">
          <h2 className="text-lg font-semibold text-gray-900 mb-4">
            {t('leases.applications.reviewHistory')}
          </h2>
          <div className="border-l-4 border-gray-200 pl-4">
            <p className="text-sm text-gray-500">
              {t('leases.applications.reviewedOn', { date: formatDate(application.reviewedAt) })}
            </p>
            {application.reviewNotes && (
              <p className="text-gray-700 mt-2">{application.reviewNotes}</p>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
