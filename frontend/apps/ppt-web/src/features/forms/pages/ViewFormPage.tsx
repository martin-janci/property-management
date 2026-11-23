/**
 * ViewFormPage Component
 *
 * View form details and submissions (Epic 54, Story 54.1, 54.4).
 */

import type {
  FormSubmissionStatus,
  FormSubmissionSummary,
  FormWithDetails,
  ListFormSubmissionsParams,
} from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { FormBuilder } from '../components/FormBuilder';
import { SubmissionList } from '../components/SubmissionList';

interface ViewFormPageProps {
  form: FormWithDetails;
  submissions: FormSubmissionSummary[];
  submissionsTotal: number;
  isLoading?: boolean;
  onNavigateToEdit: (id: string) => void;
  onPublish: (id: string) => void;
  onArchive: (id: string) => void;
  onDuplicate: (id: string) => void;
  onDownloadPdf: (id: string) => void;
  onExportSubmissions: (id: string, format: 'csv' | 'xlsx' | 'pdf') => void;
  onViewSubmission: (formId: string, submissionId: string) => void;
  onApproveSubmission: (formId: string, submissionId: string) => void;
  onRejectSubmission: (formId: string, submissionId: string) => void;
  onSubmissionsFilterChange: (params: ListFormSubmissionsParams) => void;
  onBack: () => void;
}

export function ViewFormPage({
  form,
  submissions,
  submissionsTotal,
  isLoading,
  onNavigateToEdit,
  onPublish,
  onArchive,
  onDuplicate,
  onDownloadPdf,
  onExportSubmissions,
  onViewSubmission,
  onApproveSubmission,
  onRejectSubmission,
  onSubmissionsFilterChange,
  onBack,
}: ViewFormPageProps) {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<'preview' | 'submissions'>('preview');
  const [subPage, setSubPage] = useState(1);
  const [subPageSize] = useState(10);

  const isDraft = form.status === 'draft';
  const isPublished = form.status === 'published';

  const statusColors = {
    draft: 'bg-yellow-100 text-yellow-800',
    published: 'bg-green-100 text-green-800',
    archived: 'bg-gray-100 text-gray-800',
  };

  const handleSubmissionsPageChange = (newPage: number) => {
    setSubPage(newPage);
    onSubmissionsFilterChange({ page: newPage, pageSize: subPageSize });
  };

  const handleSubmissionsStatusFilter = (status?: FormSubmissionStatus) => {
    setSubPage(1);
    onSubmissionsFilterChange({ status, page: 1, pageSize: subPageSize });
  };

  return (
    <div className="max-w-6xl mx-auto px-4 py-8">
      {/* Header */}
      <div className="mb-6">
        <button
          type="button"
          onClick={onBack}
          className="text-gray-500 hover:text-gray-700 flex items-center gap-1 mb-4"
        >
          <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <title>{t('forms.actions.back')}</title>
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M10 19l-7-7m0 0l7-7m-7 7h18"
            />
          </svg>
          {t('forms.actions.backToForms')}
        </button>

        <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
          <div>
            <div className="flex items-center gap-3">
              <h1 className="text-2xl font-bold text-gray-900">{form.title}</h1>
              <span
                className={`px-2 py-0.5 text-xs font-medium rounded-full ${statusColors[form.status as keyof typeof statusColors]}`}
              >
                {t(`forms.status.${form.status}`)}
              </span>
            </div>
            {form.description && <p className="text-gray-500 mt-1">{form.description}</p>}
          </div>

          <div className="flex flex-wrap gap-2">
            {isDraft && (
              <>
                <button
                  type="button"
                  onClick={() => onNavigateToEdit(form.id)}
                  className="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700"
                >
                  {t('forms.actions.editForm')}
                </button>
                <button
                  type="button"
                  onClick={() => onPublish(form.id)}
                  className="px-4 py-2 bg-green-600 text-white rounded-md hover:bg-green-700"
                >
                  {t('forms.actions.publish')}
                </button>
              </>
            )}
            {isPublished && (
              <button
                type="button"
                onClick={() => onArchive(form.id)}
                className="px-4 py-2 border border-gray-300 text-gray-700 rounded-md hover:bg-gray-50"
              >
                {t('forms.actions.archive')}
              </button>
            )}
            <button
              type="button"
              onClick={() => onDuplicate(form.id)}
              className="px-4 py-2 border border-gray-300 text-gray-700 rounded-md hover:bg-gray-50"
            >
              {t('forms.actions.duplicate')}
            </button>
            <button
              type="button"
              onClick={() => onDownloadPdf(form.id)}
              className="px-4 py-2 border border-gray-300 text-gray-700 rounded-md hover:bg-gray-50"
            >
              {t('forms.actions.downloadPdf')}
            </button>
          </div>
        </div>
      </div>

      {/* Stats */}
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-4 mb-6">
        <div className="bg-white border border-gray-200 rounded-lg p-4">
          <p className="text-sm text-gray-500">{t('forms.view.fields')}</p>
          <p className="text-2xl font-semibold text-gray-900">{form.fields.length}</p>
        </div>
        <div className="bg-white border border-gray-200 rounded-lg p-4">
          <p className="text-sm text-gray-500">{t('forms.view.submissions')}</p>
          <p className="text-2xl font-semibold text-gray-900">{form.submissionCount}</p>
        </div>
        <div className="bg-white border border-gray-200 rounded-lg p-4">
          <p className="text-sm text-gray-500">{t('forms.view.pendingReview')}</p>
          <p className="text-2xl font-semibold text-yellow-600">{form.pendingSubmissions}</p>
        </div>
        <div className="bg-white border border-gray-200 rounded-lg p-4">
          <p className="text-sm text-gray-500">{t('forms.view.downloads')}</p>
          <p className="text-2xl font-semibold text-gray-900">{form.downloadCount}</p>
        </div>
      </div>

      {/* Tabs */}
      <div className="border-b border-gray-200 mb-6">
        <nav className="flex gap-4">
          <button
            type="button"
            onClick={() => setActiveTab('preview')}
            className={`py-4 px-1 border-b-2 font-medium text-sm ${
              activeTab === 'preview'
                ? 'border-blue-500 text-blue-600'
                : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
            }`}
          >
            {t('forms.view.formPreview')}
          </button>
          <button
            type="button"
            onClick={() => setActiveTab('submissions')}
            className={`py-4 px-1 border-b-2 font-medium text-sm ${
              activeTab === 'submissions'
                ? 'border-blue-500 text-blue-600'
                : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
            }`}
          >
            {t('forms.view.submissionsTab', { count: form.submissionCount })}
          </button>
        </nav>
      </div>

      {/* Tab Content */}
      {activeTab === 'preview' ? (
        <FormBuilder
          fields={form.fields}
          isEditing={false}
          onAddField={() => {
            // Read-only preview mode - editing disabled
          }}
          onUpdateField={() => {
            // Read-only preview mode - editing disabled
          }}
          onDeleteField={() => {
            // Read-only preview mode - editing disabled
          }}
          onReorderFields={() => {
            // Read-only preview mode - editing disabled
          }}
        />
      ) : (
        <>
          {submissions.length > 0 && (
            <div className="mb-4 flex gap-2">
              <button
                type="button"
                onClick={() => onExportSubmissions(form.id, 'csv')}
                className="px-3 py-1.5 text-sm border border-gray-300 text-gray-700 rounded-md hover:bg-gray-50"
              >
                {t('forms.actions.exportCsv')}
              </button>
              <button
                type="button"
                onClick={() => onExportSubmissions(form.id, 'xlsx')}
                className="px-3 py-1.5 text-sm border border-gray-300 text-gray-700 rounded-md hover:bg-gray-50"
              >
                {t('forms.actions.exportExcel')}
              </button>
            </div>
          )}
          <SubmissionList
            submissions={submissions}
            total={submissionsTotal}
            page={subPage}
            pageSize={subPageSize}
            isLoading={isLoading}
            onPageChange={handleSubmissionsPageChange}
            onStatusFilter={handleSubmissionsStatusFilter}
            onView={(submissionId) => onViewSubmission(form.id, submissionId)}
            onApprove={(submissionId) => onApproveSubmission(form.id, submissionId)}
            onReject={(submissionId) => onRejectSubmission(form.id, submissionId)}
          />
        </>
      )}
    </div>
  );
}
