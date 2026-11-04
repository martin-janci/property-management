/**
 * ViolationDetailPage - View and manage a single violation.
 * UC-34: Lease Violations Tracking
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { ViolationSeverity, ViolationStatus, ViolationWithDetails } from '../types';

interface ViolationDetailPageProps {
  violation: ViolationWithDetails;
  isLoading?: boolean;
  isManager?: boolean;
  isTenant?: boolean;
  onBack: () => void;
  onResolve: (id: string, notes: string) => void;
  onDispute: (id: string, reason: string) => void;
  onUploadEvidence: (id: string, files: FileList) => void;
  /** True while evidence files are uploading — disables the upload control. */
  isUploadingEvidence?: boolean;
}

type TabType = 'details' | 'evidence' | 'timeline';

const statusColors: Record<ViolationStatus, string> = {
  open: 'bg-red-100 text-red-800',
  resolved: 'bg-green-100 text-green-800',
  disputed: 'bg-yellow-100 text-yellow-800',
  escalated: 'bg-purple-100 text-purple-800',
  dismissed: 'bg-gray-100 text-gray-800',
};

const severityColors: Record<ViolationSeverity, string> = {
  minor: 'bg-blue-100 text-blue-800',
  moderate: 'bg-orange-100 text-orange-800',
  severe: 'bg-red-100 text-red-800',
};

export function ViolationDetailPage({
  violation,
  isLoading,
  isManager,
  isTenant,
  onBack,
  onResolve,
  onDispute,
  onUploadEvidence,
  isUploadingEvidence,
}: ViolationDetailPageProps) {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<TabType>('details');
  const [showResolveForm, setShowResolveForm] = useState(false);
  const [showDisputeForm, setShowDisputeForm] = useState(false);
  const [resolutionNotes, setResolutionNotes] = useState('');
  const [disputeReason, setDisputeReason] = useState('');

  const formatDate = (dateString?: string) => {
    if (!dateString) return '-';
    return new Date(dateString).toLocaleDateString();
  };

  const formatDateTime = (dateString?: string) => {
    if (!dateString) return '-';
    return new Date(dateString).toLocaleString();
  };

  const formatFileSize = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  const canResolve = isManager && (violation.status === 'open' || violation.status === 'disputed');
  const canDispute = isTenant && violation.status === 'open';

  const handleResolveSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (resolutionNotes.trim()) {
      onResolve(violation.id, resolutionNotes);
      setShowResolveForm(false);
      setResolutionNotes('');
    }
  };

  const handleDisputeSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (disputeReason.trim()) {
      onDispute(violation.id, disputeReason);
      setShowDisputeForm(false);
      setDisputeReason('');
    }
  };

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (e.target.files && e.target.files.length > 0) {
      onUploadEvidence(violation.id, e.target.files);
      e.target.value = '';
    }
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-12">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
      </div>
    );
  }

  const tabs: { key: TabType; label: string; count?: number }[] = [
    { key: 'details', label: t('leases.violations.tabs.details') },
    {
      key: 'evidence',
      label: t('leases.violations.tabs.evidence'),
      count: violation.evidence.length,
    },
    {
      key: 'timeline',
      label: t('leases.violations.tabs.timeline'),
      count: violation.timeline.length,
    },
  ];

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
        {t('leases.violations.backToViolations')}
      </button>

      {/* Header */}
      <div className="bg-white rounded-lg shadow p-6 mb-6">
        <div className="flex items-start justify-between">
          <div>
            <div className="flex items-center gap-3">
              <h1 className="text-2xl font-bold text-gray-900">
                {t(`leases.violations.type.${violation.violationType}`)}
              </h1>
              <span
                className={`px-3 py-1 text-sm font-medium rounded-full ${statusColors[violation.status]}`}
              >
                {t(`leases.violations.status.${violation.status}`)}
              </span>
              <span
                className={`px-3 py-1 text-sm font-medium rounded-full ${severityColors[violation.severity]}`}
              >
                {t(`leases.violations.severity.${violation.severity}`)}
              </span>
            </div>
            <p className="text-gray-600 mt-1">
              {violation.unit.buildingName} - {violation.unit.number}
            </p>
            <p className="text-sm text-gray-500">{violation.tenant.name}</p>
            {violation.tenant.email && (
              <p className="text-sm text-gray-500">{violation.tenant.email}</p>
            )}
          </div>
          <div className="flex gap-2">
            {canResolve && (
              <button
                type="button"
                onClick={() => setShowResolveForm(true)}
                className="px-4 py-2 text-sm font-medium text-white bg-green-600 hover:bg-green-700 rounded-md"
              >
                {t('leases.violations.resolve')}
              </button>
            )}
            {canDispute && (
              <button
                type="button"
                onClick={() => setShowDisputeForm(true)}
                className="px-4 py-2 text-sm font-medium text-white bg-yellow-600 hover:bg-yellow-700 rounded-md"
              >
                {t('leases.violations.dispute')}
              </button>
            )}
          </div>
        </div>

        {/* Key Details Grid */}
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mt-6 pt-6 border-t">
          <div>
            <p className="text-sm text-gray-500">{t('leases.violations.violationDate')}</p>
            <p className="font-medium">{formatDate(violation.violationDate)}</p>
          </div>
          <div>
            <p className="text-sm text-gray-500">{t('leases.violations.reportedAt')}</p>
            <p className="font-medium">{formatDate(violation.reportedAt)}</p>
          </div>
          <div>
            <p className="text-sm text-gray-500">{t('leases.violations.reportedBy')}</p>
            <p className="font-medium">{violation.reportedByUser.name}</p>
          </div>
          <div>
            <p className="text-sm text-gray-500">{t('leases.violations.leaseLabel')}</p>
            <p className="font-medium">
              {formatDate(violation.lease.startDate)} - {formatDate(violation.lease.endDate)}
            </p>
          </div>
        </div>
      </div>

      {/* Resolution Form Modal */}
      {showResolveForm && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg shadow-lg p-6 max-w-md w-full mx-4">
            <h2 className="text-lg font-medium text-gray-900 mb-4">
              {t('leases.violations.resolveViolation')}
            </h2>
            <form onSubmit={handleResolveSubmit}>
              <div className="mb-4">
                <label
                  htmlFor="resolution-notes"
                  className="block text-sm font-medium text-gray-700 mb-1"
                >
                  {t('leases.violations.resolutionNotes')}
                </label>
                <textarea
                  id="resolution-notes"
                  value={resolutionNotes}
                  onChange={(e) => setResolutionNotes(e.target.value)}
                  rows={4}
                  required
                  className="block w-full border border-gray-300 rounded-md shadow-sm py-2 px-3 focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm"
                  placeholder={t('leases.violations.resolutionNotesPlaceholder')}
                />
              </div>
              <div className="flex justify-end gap-2">
                <button
                  type="button"
                  onClick={() => setShowResolveForm(false)}
                  className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50"
                >
                  {t('common.cancel')}
                </button>
                <button
                  type="submit"
                  className="px-4 py-2 text-sm font-medium text-white bg-green-600 hover:bg-green-700 rounded-md"
                >
                  {t('leases.violations.confirmResolve')}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* Dispute Form Modal */}
      {showDisputeForm && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg shadow-lg p-6 max-w-md w-full mx-4">
            <h2 className="text-lg font-medium text-gray-900 mb-4">
              {t('leases.violations.disputeViolation')}
            </h2>
            <form onSubmit={handleDisputeSubmit}>
              <div className="mb-4">
                <label
                  htmlFor="dispute-reason"
                  className="block text-sm font-medium text-gray-700 mb-1"
                >
                  {t('leases.violations.disputeReason')}
                </label>
                <textarea
                  id="dispute-reason"
                  value={disputeReason}
                  onChange={(e) => setDisputeReason(e.target.value)}
                  rows={4}
                  required
                  className="block w-full border border-gray-300 rounded-md shadow-sm py-2 px-3 focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm"
                  placeholder={t('leases.violations.disputeReasonPlaceholder')}
                />
              </div>
              <div className="flex justify-end gap-2">
                <button
                  type="button"
                  onClick={() => setShowDisputeForm(false)}
                  className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50"
                >
                  {t('common.cancel')}
                </button>
                <button
                  type="submit"
                  className="px-4 py-2 text-sm font-medium text-white bg-yellow-600 hover:bg-yellow-700 rounded-md"
                >
                  {t('leases.violations.confirmDispute')}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* Tabs */}
      <div className="bg-white rounded-lg shadow">
        <div className="border-b">
          <nav className="flex -mb-px">
            {tabs.map((tab) => (
              <button
                key={tab.key}
                type="button"
                onClick={() => setActiveTab(tab.key)}
                className={`px-6 py-3 text-sm font-medium border-b-2 ${
                  activeTab === tab.key
                    ? 'border-blue-500 text-blue-600'
                    : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
                }`}
              >
                {tab.label}
                {tab.count !== undefined && tab.count > 0 && (
                  <span className="ml-2 px-2 py-0.5 text-xs bg-gray-100 rounded-full">
                    {tab.count}
                  </span>
                )}
              </button>
            ))}
          </nav>
        </div>

        <div className="p-6">
          {/* Details Tab */}
          {activeTab === 'details' && (
            <div className="space-y-6">
              <div>
                <h3 className="text-lg font-medium text-gray-900 mb-3">
                  {t('leases.violations.descriptionLabel')}
                </h3>
                <div className="bg-gray-50 rounded-lg p-4">
                  <p className="text-gray-700 whitespace-pre-wrap">{violation.description}</p>
                </div>
              </div>

              {violation.status === 'resolved' && violation.resolutionNotes && (
                <div className="bg-green-50 rounded-lg p-4">
                  <h3 className="text-lg font-medium text-green-800 mb-2">
                    {t('leases.violations.resolution')}
                  </h3>
                  <p className="text-sm text-green-700">{violation.resolutionNotes}</p>
                  {violation.resolvedAt && (
                    <p className="text-xs text-green-600 mt-2">
                      {t('leases.violations.resolvedAt')}: {formatDateTime(violation.resolvedAt)}
                    </p>
                  )}
                </div>
              )}

              {violation.status === 'disputed' && violation.disputeReason && (
                <div className="bg-yellow-50 rounded-lg p-4">
                  <h3 className="text-lg font-medium text-yellow-800 mb-2">
                    {t('leases.violations.disputeInfo')}
                  </h3>
                  <p className="text-sm text-yellow-700">{violation.disputeReason}</p>
                  {violation.disputedAt && (
                    <p className="text-xs text-yellow-600 mt-2">
                      {t('leases.violations.disputedAt')}: {formatDateTime(violation.disputedAt)}
                    </p>
                  )}
                </div>
              )}
            </div>
          )}

          {/* Evidence Tab */}
          {activeTab === 'evidence' && (
            <div>
              <div className="flex items-center justify-between mb-4">
                <h3 className="text-lg font-medium text-gray-900">
                  {t('leases.violations.evidenceDocumentation')}
                </h3>
                <label
                  className={`px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded-md ${
                    isUploadingEvidence ? 'opacity-50 cursor-wait' : 'cursor-pointer'
                  }`}
                  aria-busy={isUploadingEvidence || undefined}
                >
                  <span className="flex items-center gap-2">
                    <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"
                      />
                    </svg>
                    {isUploadingEvidence
                      ? t('leases.violations.uploadingEvidence')
                      : t('leases.violations.uploadEvidence')}
                  </span>
                  <input
                    type="file"
                    disabled={isUploadingEvidence}
                    multiple
                    accept="image/*,.pdf,.doc,.docx"
                    onChange={handleFileChange}
                    className="hidden"
                  />
                </label>
              </div>

              {violation.evidence.length === 0 ? (
                <div className="text-center py-8">
                  <svg
                    className="mx-auto h-12 w-12 text-gray-400"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z"
                    />
                  </svg>
                  <p className="mt-2 text-sm text-gray-500">{t('leases.violations.noEvidence')}</p>
                </div>
              ) : (
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                  {violation.evidence.map((evidence) => (
                    <div key={evidence.id} className="border rounded-lg p-4">
                      <div className="flex items-start justify-between">
                        <div className="flex items-center gap-3">
                          <div className="p-2 bg-gray-100 rounded">
                            <svg
                              className="w-6 h-6 text-gray-500"
                              fill="none"
                              stroke="currentColor"
                              viewBox="0 0 24 24"
                            >
                              <path
                                strokeLinecap="round"
                                strokeLinejoin="round"
                                strokeWidth={2}
                                d="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z"
                              />
                            </svg>
                          </div>
                          <div>
                            <p className="font-medium text-gray-900">{evidence.fileName}</p>
                            <p className="text-xs text-gray-500">
                              {formatFileSize(evidence.fileSize)} |{' '}
                              {formatDateTime(evidence.uploadedAt)}
                            </p>
                          </div>
                        </div>
                        {evidence.fileUrl && (
                          <a
                            href={evidence.fileUrl}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="text-blue-600 hover:text-blue-800"
                          >
                            <svg
                              className="w-5 h-5"
                              fill="none"
                              stroke="currentColor"
                              viewBox="0 0 24 24"
                            >
                              <path
                                strokeLinecap="round"
                                strokeLinejoin="round"
                                strokeWidth={2}
                                d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"
                              />
                            </svg>
                          </a>
                        )}
                      </div>
                      {evidence.description && (
                        <p className="text-sm text-gray-600 mt-2">{evidence.description}</p>
                      )}
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}

          {/* Timeline Tab */}
          {activeTab === 'timeline' && (
            <div>
              <h3 className="text-lg font-medium text-gray-900 mb-4">
                {t('leases.violations.statusHistory')}
              </h3>
              {violation.timeline.length === 0 ? (
                <p className="text-gray-500 text-center py-8">
                  {t('leases.violations.noTimeline')}
                </p>
              ) : (
                <div className="flow-root">
                  <ul className="-mb-8">
                    {violation.timeline.map((event, eventIdx) => (
                      <li key={event.id}>
                        <div className="relative pb-8">
                          {eventIdx !== violation.timeline.length - 1 && (
                            <span
                              className="absolute top-4 left-4 -ml-px h-full w-0.5 bg-gray-200"
                              aria-hidden="true"
                            />
                          )}
                          <div className="relative flex space-x-3">
                            <div>
                              <span className="h-8 w-8 rounded-full bg-blue-500 flex items-center justify-center ring-8 ring-white">
                                <svg
                                  className="w-4 h-4 text-white"
                                  fill="currentColor"
                                  viewBox="0 0 20 20"
                                >
                                  <path
                                    fillRule="evenodd"
                                    d="M10 18a8 8 0 100-16 8 8 0 000 16zm1-12a1 1 0 10-2 0v4a1 1 0 00.293.707l2.828 2.829a1 1 0 101.415-1.415L11 9.586V6z"
                                    clipRule="evenodd"
                                  />
                                </svg>
                              </span>
                            </div>
                            <div className="flex min-w-0 flex-1 justify-between space-x-4 pt-1.5">
                              <div>
                                <p className="text-sm text-gray-900">
                                  {event.description}
                                  <span className="text-gray-500">
                                    {' '}
                                    {t('leases.violations.by')}{' '}
                                  </span>
                                  <span className="font-medium">{event.createdByName}</span>
                                </p>
                              </div>
                              <div className="whitespace-nowrap text-right text-sm text-gray-500">
                                {formatDateTime(event.createdAt)}
                              </div>
                            </div>
                          </div>
                        </div>
                      </li>
                    ))}
                  </ul>
                </div>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
