/**
 * SubmissionList Component
 *
 * Displays form submissions with filtering and review actions (Epic 54, Story 54.4).
 */

import type { FormSubmissionStatus, FormSubmissionSummary } from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { getActiveLocale } from '../../../lib/locale';

interface SubmissionListProps {
  submissions: FormSubmissionSummary[];
  total: number;
  page: number;
  pageSize: number;
  isLoading?: boolean;
  onPageChange: (page: number) => void;
  onStatusFilter: (status?: FormSubmissionStatus) => void;
  onView: (submissionId: string) => void;
  onApprove: (submissionId: string) => void;
  onReject: (submissionId: string) => void;
}

const statusColors: Record<FormSubmissionStatus, string> = {
  pending: 'bg-yellow-100 text-yellow-800',
  reviewed: 'bg-blue-100 text-blue-800',
  approved: 'bg-green-100 text-green-800',
  rejected: 'bg-red-100 text-red-800',
};

export function SubmissionList({
  submissions,
  total,
  page,
  pageSize,
  isLoading,
  onPageChange,
  onStatusFilter,
  onView,
  onApprove,
  onReject,
}: SubmissionListProps) {
  const { t } = useTranslation();
  const [statusFilter, setStatusFilter] = useState<FormSubmissionStatus | ''>('');
  const totalPages = Math.ceil(total / pageSize);

  const handleStatusChange = (value: string) => {
    setStatusFilter(value as FormSubmissionStatus | '');
    onStatusFilter(value ? (value as FormSubmissionStatus) : undefined);
  };

  return (
    <div>
      {/* Header */}
      <div className="mb-6 flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
        <h2 className="text-xl font-semibold text-gray-900">{t('forms.submissions.title')}</h2>
        <div className="flex gap-4">
          <select
            value={statusFilter}
            onChange={(e) => handleStatusChange(e.target.value)}
            className="px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
          >
            <option value="">{t('forms.filters.allStatuses')}</option>
            <option value="pending">{t('forms.submissions.status.pending')}</option>
            <option value="reviewed">{t('forms.submissions.status.reviewed')}</option>
            <option value="approved">{t('forms.submissions.status.approved')}</option>
            <option value="rejected">{t('forms.submissions.status.rejected')}</option>
          </select>
        </div>
      </div>

      {/* Table */}
      {isLoading ? (
        <div className="flex items-center justify-center py-12">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
        </div>
      ) : submissions.length === 0 ? (
        <div className="text-center py-12 text-gray-500 border border-gray-200 rounded-lg">
          <svg
            className="mx-auto h-12 w-12 text-gray-400 mb-4"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <title>{t('forms.submissions.noSubmissionsIcon')}</title>
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
            />
          </svg>
          <p className="text-lg font-medium mb-2">{t('forms.submissions.emptyTitle')}</p>
          <p>{t('forms.submissions.emptyDescription')}</p>
        </div>
      ) : (
        <div className="overflow-x-auto">
          <table className="min-w-full divide-y divide-gray-200">
            <thead className="bg-gray-50">
              <tr>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  {t('forms.submissions.columns.submittedBy')}
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  {t('forms.submissions.columns.form')}
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  {t('forms.submissions.columns.status')}
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  {t('forms.submissions.columns.submittedAt')}
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  {t('forms.submissions.columns.reviewedAt')}
                </th>
                <th className="px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">
                  {t('forms.submissions.columns.actions')}
                </th>
              </tr>
            </thead>
            <tbody className="bg-white divide-y divide-gray-200">
              {submissions.map((submission) => {
                const status = submission.status as FormSubmissionStatus;
                const isPending = status === 'pending';

                return (
                  <tr key={submission.id} className="hover:bg-gray-50">
                    <td className="px-6 py-4 whitespace-nowrap">
                      <div className="text-sm font-medium text-gray-900">{submission.userName}</div>
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap">
                      <div className="text-sm text-gray-500">{submission.formTitle}</div>
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap">
                      <span
                        className={`px-2 py-1 text-xs font-medium rounded-full ${statusColors[status]}`}
                      >
                        {t(`forms.submissions.status.${status}`)}
                      </span>
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                      {new Date(submission.submittedAt).toLocaleString(getActiveLocale())}
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                      {submission.reviewedAt
                        ? new Date(submission.reviewedAt).toLocaleString(getActiveLocale())
                        : '-'}
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-right text-sm font-medium">
                      <div className="flex justify-end gap-2">
                        <button
                          type="button"
                          onClick={() => onView(submission.id)}
                          className="text-blue-600 hover:text-blue-900"
                        >
                          {t('forms.actions.view')}
                        </button>
                        {isPending && (
                          <>
                            <button
                              type="button"
                              onClick={() => onApprove(submission.id)}
                              className="text-green-600 hover:text-green-900"
                            >
                              {t('forms.submissions.approve')}
                            </button>
                            <button
                              type="button"
                              onClick={() => onReject(submission.id)}
                              className="text-red-600 hover:text-red-900"
                            >
                              {t('forms.submissions.reject')}
                            </button>
                          </>
                        )}
                      </div>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}

      {/* Pagination */}
      {totalPages > 1 && (
        <div className="mt-6 flex items-center justify-between">
          <p className="text-sm text-gray-500">
            {t('forms.pagination.showing', {
              from: (page - 1) * pageSize + 1,
              to: Math.min(page * pageSize, total),
              total,
            })}
          </p>
          <div className="flex gap-2">
            <button
              type="button"
              onClick={() => onPageChange(page - 1)}
              disabled={page === 1}
              className="px-3 py-1 border rounded-md disabled:opacity-50 disabled:cursor-not-allowed hover:bg-gray-50"
            >
              {t('common.previous')}
            </button>
            <span className="px-3 py-1">{t('forms.pagination.pageOf', { page, totalPages })}</span>
            <button
              type="button"
              onClick={() => onPageChange(page + 1)}
              disabled={page === totalPages}
              className="px-3 py-1 border rounded-md disabled:opacity-50 disabled:cursor-not-allowed hover:bg-gray-50"
            >
              {t('common.next')}
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
