/**
 * ClaimDetailPage - View insurance claim details.
 * Insurance Management Feature
 */

import { useTranslation } from 'react-i18next';
import { getActiveLocale } from '../../../lib/locale';
import type { ClaimStatus, ClaimType, InsuranceClaim } from '../types';

interface ClaimDocument {
  id: string;
  filename: string;
  contentType: string;
  sizeBytes: number;
  uploadedAt: string;
}

interface ClaimDetailPageProps {
  claim: InsuranceClaim;
  documents: ClaimDocument[];
  isLoading?: boolean;
  onBack: () => void;
  onViewPolicy: () => void;
  onUploadDocument: (file: File) => void;
  onDownloadDocument: (id: string) => void;
  onDeleteDocument: (id: string) => void;
  onUpdateStatus?: (status: ClaimStatus, notes?: string) => void;
}

const statusColors: Record<ClaimStatus, string> = {
  submitted: 'bg-blue-100 text-blue-800',
  under_review: 'bg-yellow-100 text-yellow-800',
  approved: 'bg-green-100 text-green-800',
  denied: 'bg-red-100 text-red-800',
  settled: 'bg-purple-100 text-purple-800',
  closed: 'bg-gray-100 text-gray-800',
};

export function ClaimDetailPage({
  claim,
  documents,
  isLoading,
  onBack,
  onViewPolicy,
  onUploadDocument,
  onDownloadDocument,
  onDeleteDocument,
  onUpdateStatus,
}: ClaimDetailPageProps) {
  const { t } = useTranslation();

  const statusLabels: Record<ClaimStatus, string> = {
    submitted: t('insurance.claimStatusSubmitted'),
    under_review: t('insurance.claimStatusUnderReview'),
    approved: t('insurance.claimStatusApproved'),
    denied: t('insurance.claimStatusDenied'),
    settled: t('insurance.claimStatusSettled'),
    closed: t('insurance.claimStatusClosed'),
  };

  const typeLabels: Record<ClaimType, string> = {
    property_damage: t('insurance.claimTypePropertyDamage'),
    liability: t('insurance.claimTypeLiability'),
    theft: t('insurance.claimTypeTheft'),
    water_damage: t('insurance.claimTypeWaterDamage'),
    fire_damage: t('insurance.claimTypeFireDamage'),
    natural_disaster: t('insurance.claimTypeNaturalDisaster'),
    personal_injury: t('insurance.claimTypePersonalInjury'),
    other: t('insurance.claimTypeOther'),
  };

  const formatCurrency = (amount: number, currency: string) => {
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: currency,
    }).format(amount);
  };

  const formatFileSize = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  const handleFileUpload = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) {
      onUploadDocument(file);
    }
  };

  const daysSinceIncident = Math.floor(
    (new Date().getTime() - new Date(claim.incidentDate).getTime()) / (1000 * 60 * 60 * 24)
  );

  if (isLoading) {
    return (
      <div className="flex justify-center py-12">
        <div className="animate-spin rounded-full h-10 w-10 border-b-2 border-blue-600" />
      </div>
    );
  }

  return (
    <div className="max-w-4xl mx-auto px-4 py-8">
      {/* Header */}
      <div className="mb-6">
        <button
          type="button"
          onClick={onBack}
          className="text-sm text-blue-600 hover:text-blue-800 flex items-center gap-1 mb-4"
        >
          {t('insurance.backToClaims')}
        </button>
        <div className="flex items-start justify-between">
          <div>
            <div className="flex items-center gap-2 mb-2">
              <span
                className={`px-2 py-1 text-sm font-medium rounded ${statusColors[claim.status]}`}
              >
                {statusLabels[claim.status]}
              </span>
              <span className="px-2 py-1 text-sm font-medium rounded bg-gray-100 text-gray-700">
                {typeLabels[claim.claimType]}
              </span>
            </div>
            <h1 className="text-2xl font-bold text-gray-900">{claim.title}</h1>
            {claim.claimNumber && (
              <p className="text-gray-600">
                {t('insurance.claimNumber')}: {claim.claimNumber}
              </p>
            )}
          </div>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Main Content */}
        <div className="lg:col-span-2 space-y-6">
          {/* Description */}
          <div className="bg-white rounded-lg shadow p-6">
            <h2 className="text-lg font-semibold text-gray-900 mb-4">
              {t('insurance.claimDescription')}
            </h2>
            <p className="text-gray-700 whitespace-pre-wrap">{claim.description}</p>
          </div>

          {/* Financial Details */}
          <div className="bg-white rounded-lg shadow p-6">
            <h2 className="text-lg font-semibold text-gray-900 mb-4">
              {t('insurance.financialDetails')}
            </h2>
            <div className="grid grid-cols-2 gap-4">
              {claim.claimAmount && (
                <div>
                  <p className="text-sm text-gray-500">{t('insurance.claimAmount')}</p>
                  <p className="text-lg font-semibold">
                    {formatCurrency(claim.claimAmount, claim.currency)}
                  </p>
                </div>
              )}
              {claim.approvedAmount && (
                <div>
                  <p className="text-sm text-gray-500">{t('insurance.approvedAmount')}</p>
                  <p className="text-lg font-semibold text-green-600">
                    {formatCurrency(claim.approvedAmount, claim.currency)}
                  </p>
                </div>
              )}
            </div>
          </div>

          {/* Documents */}
          <div className="bg-white rounded-lg shadow p-6">
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-lg font-semibold text-gray-900">
                {t('insurance.documents')} ({documents.length})
              </h2>
              <label className="px-3 py-1.5 text-sm border border-gray-300 rounded-lg hover:bg-gray-50 cursor-pointer">
                {t('insurance.uploadDocument')}
                <input type="file" className="hidden" onChange={handleFileUpload} />
              </label>
            </div>
            {documents.length === 0 ? (
              <p className="text-gray-500">{t('insurance.noDocuments')}</p>
            ) : (
              <ul className="divide-y">
                {documents.map((doc) => (
                  <li key={doc.id} className="py-3 flex items-center justify-between">
                    <div>
                      <p className="font-medium">{doc.filename}</p>
                      <p className="text-sm text-gray-500">
                        {formatFileSize(doc.sizeBytes)} -{' '}
                        {new Date(doc.uploadedAt).toLocaleDateString(getActiveLocale())}
                      </p>
                    </div>
                    <div className="flex gap-2">
                      <button
                        type="button"
                        onClick={() => onDownloadDocument(doc.id)}
                        className="text-sm text-blue-600 hover:text-blue-800"
                      >
                        {t('documents.download')}
                      </button>
                      <button
                        type="button"
                        onClick={() => onDeleteDocument(doc.id)}
                        className="text-sm text-red-600 hover:text-red-800"
                      >
                        {t('common.delete')}
                      </button>
                    </div>
                  </li>
                ))}
              </ul>
            )}
          </div>

          {/* Notes */}
          {claim.notes && (
            <div className="bg-white rounded-lg shadow p-6">
              <h2 className="text-lg font-semibold text-gray-900 mb-4">{t('insurance.notes')}</h2>
              <p className="text-gray-700 whitespace-pre-wrap">{claim.notes}</p>
            </div>
          )}
        </div>

        {/* Sidebar */}
        <div className="space-y-6">
          {/* Claim Details */}
          <div className="bg-white rounded-lg shadow p-6">
            <h3 className="text-lg font-semibold text-gray-900 mb-4">
              {t('insurance.claimDetails')}
            </h3>
            <dl className="space-y-3 text-sm">
              <div>
                <dt className="text-gray-500">{t('insurance.incidentDate')}</dt>
                <dd className="font-medium">
                  {new Date(claim.incidentDate).toLocaleDateString(getActiveLocale())}
                  <span className="text-gray-400 text-xs ml-1">
                    ({daysSinceIncident} {t('insurance.daysAgo')})
                  </span>
                </dd>
              </div>
              <div>
                <dt className="text-gray-500">{t('insurance.reportedDate')}</dt>
                <dd className="font-medium">
                  {new Date(claim.reportedDate).toLocaleDateString(getActiveLocale())}
                </dd>
              </div>
              <div>
                <dt className="text-gray-500">{t('insurance.filedBy')}</dt>
                <dd className="font-medium">{claim.filedByName || claim.filedBy}</dd>
              </div>
              {claim.buildingName && (
                <div>
                  <dt className="text-gray-500">{t('insurance.location')}</dt>
                  <dd className="font-medium">{claim.buildingName}</dd>
                  {claim.unitDesignation && (
                    <dd className="text-gray-500">{claim.unitDesignation}</dd>
                  )}
                </div>
              )}
              <div>
                <dt className="text-gray-500">{t('insurance.createdAt')}</dt>
                <dd className="font-medium">
                  {new Date(claim.createdAt).toLocaleDateString(getActiveLocale())}
                </dd>
              </div>
            </dl>
          </div>

          {/* Policy Information */}
          <div className="bg-white rounded-lg shadow p-6">
            <h3 className="text-lg font-semibold text-gray-900 mb-4">
              {t('insurance.policyInfo')}
            </h3>
            <dl className="space-y-3 text-sm">
              {claim.policyProvider && (
                <div>
                  <dt className="text-gray-500">{t('insurance.provider')}</dt>
                  <dd className="font-medium">{claim.policyProvider}</dd>
                </div>
              )}
              {claim.policyNumber && (
                <div>
                  <dt className="text-gray-500">{t('insurance.policyNumber')}</dt>
                  <dd className="font-medium">{claim.policyNumber}</dd>
                </div>
              )}
            </dl>
            <button
              type="button"
              onClick={onViewPolicy}
              className="mt-4 text-sm text-blue-600 hover:text-blue-800"
            >
              {t('insurance.viewPolicy')}
            </button>
          </div>

          {/* Adjuster Information */}
          {claim.assignedAdjuster && (
            <div className="bg-white rounded-lg shadow p-6">
              <h3 className="text-lg font-semibold text-gray-900 mb-4">
                {t('insurance.adjusterInfo')}
              </h3>
              <dl className="space-y-3 text-sm">
                <div>
                  <dt className="text-gray-500">{t('insurance.adjuster')}</dt>
                  <dd className="font-medium">{claim.assignedAdjuster}</dd>
                </div>
                {claim.adjusterContact && (
                  <div>
                    <dt className="text-gray-500">{t('insurance.contact')}</dt>
                    <dd className="font-medium">{claim.adjusterContact}</dd>
                  </div>
                )}
              </dl>
            </div>
          )}

          {/* Status Actions */}
          {onUpdateStatus && claim.status !== 'closed' && (
            <div className="bg-white rounded-lg shadow p-6">
              <h3 className="text-lg font-semibold text-gray-900 mb-4">{t('insurance.actions')}</h3>
              <div className="space-y-2">
                {claim.status === 'submitted' && (
                  <button
                    type="button"
                    onClick={() => onUpdateStatus('under_review')}
                    className="w-full px-3 py-2 text-sm bg-yellow-600 text-white rounded-lg hover:bg-yellow-700"
                  >
                    {t('insurance.markUnderReview')}
                  </button>
                )}
                {claim.status === 'under_review' && (
                  <>
                    <button
                      type="button"
                      onClick={() => onUpdateStatus('approved')}
                      className="w-full px-3 py-2 text-sm bg-green-600 text-white rounded-lg hover:bg-green-700"
                    >
                      {t('insurance.approveClaim')}
                    </button>
                    <button
                      type="button"
                      onClick={() => onUpdateStatus('denied')}
                      className="w-full px-3 py-2 text-sm bg-red-600 text-white rounded-lg hover:bg-red-700"
                    >
                      {t('insurance.denyClaim')}
                    </button>
                  </>
                )}
                {claim.status === 'approved' && (
                  <button
                    type="button"
                    onClick={() => onUpdateStatus('settled')}
                    className="w-full px-3 py-2 text-sm bg-purple-600 text-white rounded-lg hover:bg-purple-700"
                  >
                    {t('insurance.markSettled')}
                  </button>
                )}
                {['approved', 'denied', 'settled'].includes(claim.status) && (
                  <button
                    type="button"
                    onClick={() => onUpdateStatus('closed')}
                    className="w-full px-3 py-2 text-sm bg-gray-600 text-white rounded-lg hover:bg-gray-700"
                  >
                    {t('insurance.closeClaim')}
                  </button>
                )}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
