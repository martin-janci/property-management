/**
 * ApplicationCard component - displays a tenant application summary.
 * Epic 19: Lease Management & Tenant Screening
 */

import { useTranslation } from 'react-i18next';
import { getActiveLocale } from '../../../lib/locale';
import type { ApplicationStatus, ApplicationSummary } from '../types';

interface ApplicationCardProps {
  application: ApplicationSummary;
  onView?: (id: string) => void;
  onReview?: (id: string) => void;
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

export function ApplicationCard({ application, onView, onReview }: ApplicationCardProps) {
  const { t } = useTranslation();

  const canReview =
    application.status === 'submitted' ||
    application.status === 'under_review' ||
    application.status === 'screening';

  const formatDate = (dateString?: string) => {
    if (!dateString) return '-';
    return new Date(dateString).toLocaleDateString(getActiveLocale());
  };

  return (
    <div className="bg-white rounded-lg shadow p-4 hover:shadow-md transition-shadow">
      <div className="flex items-start justify-between">
        <div className="flex-1">
          <div className="flex items-center gap-2">
            {application.status === 'submitted' && (
              <span className="text-blue-500" title={t('leases.applications.newApplication')}>
                <svg
                  className="w-4 h-4"
                  fill="currentColor"
                  viewBox="0 0 20 20"
                  aria-label={t('leases.applications.newApplication')}
                >
                  <title>{t('leases.applications.newApplication')}</title>
                  <path d="M10 2a6 6 0 00-6 6v3.586l-.707.707A1 1 0 004 14h12a1 1 0 00.707-1.707L16 11.586V8a6 6 0 00-6-6zM10 18a3 3 0 01-3-3h6a3 3 0 01-3 3z" />
                </svg>
              </span>
            )}
            <h3 className="text-lg font-semibold text-gray-900">{application.applicantName}</h3>
          </div>
          <p className="text-sm text-gray-600 mt-1">{application.applicantEmail}</p>
          <p className="text-xs text-gray-500 mt-1">
            {application.buildingName} - {application.unitNumber}
          </p>
          <div className="mt-2 flex items-center gap-2 flex-wrap">
            <span
              className={`px-2 py-1 text-xs font-medium rounded ${statusColors[application.status]}`}
            >
              {t(`leases.applications.status.${application.status}`)}
            </span>
          </div>
          <div className="mt-2 text-xs text-gray-500">
            {application.submittedAt ? (
              <span>
                {t('leases.applications.submittedOn')} {formatDate(application.submittedAt)}
              </span>
            ) : (
              <span>
                {t('leases.applications.createdOn')} {formatDate(application.createdAt)}
              </span>
            )}
          </div>
        </div>
      </div>

      <div className="mt-4 flex items-center gap-2 border-t pt-3">
        <button
          type="button"
          onClick={() => onView?.(application.id)}
          className="text-sm text-blue-600 hover:text-blue-800"
        >
          {t('common.view')}
        </button>
        {canReview && (
          <button
            type="button"
            onClick={() => onReview?.(application.id)}
            className="text-sm text-green-600 hover:text-green-800"
          >
            {t('leases.applications.review')}
          </button>
        )}
      </div>
    </div>
  );
}
