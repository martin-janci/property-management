/**
 * Import Page (Epic 66, Stories 66.2, 66.4).
 * Epic 90, Story 90.3: Wire up template download, import, retry handlers to API.
 *
 * Main page for bulk data import workflow.
 */

import {
  useDownloadTemplate,
  useImportJobs,
  useImportTemplates,
  useRetryImport,
  useStartImport,
} from '@ppt/api-client';
import { useCallback, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useToast } from '../../../components';
import { FileUploader } from '../components/FileUploader';
import { type ImportJobHistoryItem, ImportJobList } from '../components/ImportJobList';
import { ImportJobProgress, type ImportJobStatusData } from '../components/ImportJobProgress';
import { ImportPreview, type ImportPreviewData } from '../components/ImportPreview';
import { ImportTemplateList, type ImportTemplateSummary } from '../components/ImportTemplateList';

type ImportStep = 'select_template' | 'upload' | 'preview' | 'importing' | 'complete';

export function ImportPage() {
  const navigate = useNavigate();
  const { showToast } = useToast();
  const [step, setStep] = useState<ImportStep>('select_template');
  const [selectedTemplate, setSelectedTemplate] = useState<ImportTemplateSummary | null>(null);
  const [currentJobId, setCurrentJobId] = useState<string | null>(null);
  const [previewData, setPreviewData] = useState<ImportPreviewData | null>(null);
  const [showHistory, setShowHistory] = useState(false);
  const [importError, setImportError] = useState<string | null>(null);

  // Fetch templates from API
  const { data: templatesData, isLoading: templatesLoading } = useImportTemplates();

  // Fetch job history from API
  const { data: jobsData, isLoading: jobsLoading, refetch: refetchJobs } = useImportJobs();

  // Mutations
  const downloadTemplate = useDownloadTemplate();
  const startImport = useStartImport();
  const retryImport = useRetryImport();

  // Get data from API responses - cast to component types
  const templates = (templatesData?.templates ?? []) as ImportTemplateSummary[];
  const jobs = (jobsData?.jobs ?? []) as ImportJobHistoryItem[];

  // Handle template selection
  const handleSelectTemplate = useCallback((template: ImportTemplateSummary) => {
    setSelectedTemplate(template);
    setStep('upload');
    setImportError(null);
  }, []);

  // Handle file upload completion
  const handleUploadComplete = useCallback((jobId: string, preview?: ImportPreviewData) => {
    setCurrentJobId(jobId);
    if (preview) {
      setPreviewData(preview);
    }
    setStep('preview');
  }, []);

  // Handle import approval
  const handleApproveImport = useCallback(
    (acknowledgeWarnings: boolean) => {
      if (!currentJobId) return;
      startImport.mutate(
        { jobId: currentJobId, acknowledgeWarnings },
        {
          onSuccess: () => setStep('importing'),
          onError: (err) => setImportError(err.message || 'Failed to start import'),
        }
      );
    },
    [currentJobId, startImport]
  );

  // Handle import completion
  const handleImportComplete = useCallback(
    (_status: ImportJobStatusData) => {
      setStep('complete');
      refetchJobs();
    },
    [refetchJobs]
  );

  // Handle starting a new import
  const handleStartNew = useCallback(() => {
    setStep('select_template');
    setSelectedTemplate(null);
    setCurrentJobId(null);
    setPreviewData(null);
    setImportError(null);
  }, []);

  // Handle cancel
  const handleCancel = useCallback(() => {
    if (step === 'upload') {
      setStep('select_template');
      setSelectedTemplate(null);
    } else if (step === 'preview') {
      setStep('upload');
      setPreviewData(null);
    }
    setImportError(null);
  }, [step]);

  // Handle template download
  const handleDownloadTemplate = useCallback(
    (template: ImportTemplateSummary, format: 'csv' | 'xlsx') => {
      downloadTemplate.mutate(
        { id: template.id, format },
        {
          onError: (err) => alert(`Failed to download template: ${err.message}`),
        }
      );
    },
    [downloadTemplate]
  );

  // Handle view job details
  const handleViewJob = useCallback(
    (job: ImportJobHistoryItem) => {
      navigate(`/import/jobs/${job.id}`);
    },
    [navigate]
  );

  // Handle retry job
  const handleRetryJob = useCallback(
    (job: ImportJobHistoryItem) => {
      retryImport.mutate(
        { jobId: job.id },
        {
          onSuccess: () => {
            refetchJobs();
            showToast({ type: 'success', title: 'Import retry started' });
          },
          onError: (err) =>
            showToast({ type: 'error', title: 'Failed to retry import', message: err.message }),
        }
      );
    },
    [retryImport, refetchJobs, showToast]
  );

  // Handle view job errors
  const handleViewErrors = useCallback(
    (job: ImportJobHistoryItem) => {
      navigate(`/import/jobs/${job.id}/errors`);
    },
    [navigate]
  );

  // Loading state for templates
  if (templatesLoading && step === 'select_template') {
    return (
      <div className="space-y-6">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-xl font-semibold text-gray-900">Import Data</h1>
            <p className="mt-1 text-sm text-gray-500">
              Upload spreadsheets to import data into your organization.
            </p>
          </div>
        </div>
        <div className="rounded-lg border border-gray-200 bg-white p-6 text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mx-auto" />
          <p className="mt-4 text-gray-500">Loading templates...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-semibold text-gray-900">Import Data</h1>
          <p className="mt-1 text-sm text-gray-500">
            Upload spreadsheets to import data into your organization.
          </p>
        </div>
        <button
          type="button"
          onClick={() => setShowHistory(!showHistory)}
          className="text-sm font-medium text-blue-600 hover:text-blue-700"
        >
          {showHistory ? 'Hide History' : 'View History'}
        </button>
      </div>

      {/* Error Display */}
      {importError && (
        <div className="rounded-lg bg-red-50 border border-red-200 p-4">
          <p className="text-sm text-red-700">{importError}</p>
          <button
            type="button"
            onClick={() => setImportError(null)}
            className="mt-2 text-sm text-red-600 underline"
          >
            Dismiss
          </button>
        </div>
      )}

      {/* Progress Steps */}
      <div className="flex items-center gap-2">
        {[
          { id: 'select_template', label: '1. Select Template' },
          { id: 'upload', label: '2. Upload File' },
          { id: 'preview', label: '3. Review' },
          { id: 'importing', label: '4. Import' },
        ].map((s, index) => (
          <div key={s.id} className="flex items-center">
            <div
              className={`flex items-center gap-2 rounded-full px-3 py-1 text-sm ${
                step === s.id
                  ? 'bg-blue-100 text-blue-800'
                  : ['select_template', 'upload', 'preview', 'importing'].indexOf(step) >
                      ['select_template', 'upload', 'preview', 'importing'].indexOf(
                        s.id as ImportStep
                      )
                    ? 'bg-green-100 text-green-800'
                    : 'bg-gray-100 text-gray-600'
              }`}
            >
              <span>{s.label}</span>
            </div>
            {index < 3 && (
              <svg
                className="mx-2 h-4 w-4 text-gray-400"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M9 5l7 7-7 7"
                />
              </svg>
            )}
          </div>
        ))}
      </div>

      {/* Main Content */}
      <div className="rounded-lg border border-gray-200 bg-white p-6">
        {/* Step 1: Select Template */}
        {step === 'select_template' && (
          <ImportTemplateList
            templates={templates}
            onSelect={handleSelectTemplate}
            onEdit={(_template) => {
              // Not available in import flow - use Templates page to edit
            }}
            onDownload={handleDownloadTemplate}
            onDuplicate={(_template) => {
              // Not available in import flow - use Templates page to duplicate
            }}
            onDelete={(_template) => {
              // Not available in import flow - use Templates page to delete
            }}
            onCreate={() => {
              // Not available in import flow - use Templates page to create
            }}
          />
        )}

        {/* Step 2: Upload File */}
        {step === 'upload' && selectedTemplate && (
          <FileUploader
            templateId={selectedTemplate.id}
            templateName={selectedTemplate.name}
            onUploadComplete={handleUploadComplete}
            onCancel={handleCancel}
          />
        )}

        {/* Step 3: Preview/Validation */}
        {step === 'preview' && previewData && (
          <ImportPreview
            preview={previewData}
            onApprove={handleApproveImport}
            onCancel={handleCancel}
          />
        )}

        {/* Step 4: Importing */}
        {step === 'importing' && currentJobId && (
          <ImportJobProgress
            jobId={currentJobId}
            onComplete={handleImportComplete}
            onCancel={() => setStep('preview')}
          />
        )}

        {/* Step 5: Complete */}
        {step === 'complete' && (
          <div className="text-center py-12">
            <div className="mx-auto h-16 w-16 rounded-full bg-green-100 p-4">
              <svg
                className="h-8 w-8 text-green-600"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M5 13l4 4L19 7"
                />
              </svg>
            </div>
            <h2 className="mt-4 text-lg font-medium text-gray-900">Import Complete</h2>
            <p className="mt-2 text-sm text-gray-500">Your data has been successfully imported.</p>
            <button
              type="button"
              onClick={handleStartNew}
              className="mt-6 inline-flex items-center rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700"
            >
              Start New Import
            </button>
          </div>
        )}
      </div>

      {/* Import History */}
      {showHistory && (
        <div className="rounded-lg border border-gray-200 bg-white p-6">
          {jobsLoading ? (
            <div className="text-center py-8">
              <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600 mx-auto" />
              <p className="mt-2 text-sm text-gray-500">Loading history...</p>
            </div>
          ) : (
            <ImportJobList
              jobs={jobs}
              onViewJob={handleViewJob}
              onRetryJob={handleRetryJob}
              onViewErrors={handleViewErrors}
            />
          )}
        </div>
      )}
    </div>
  );
}
