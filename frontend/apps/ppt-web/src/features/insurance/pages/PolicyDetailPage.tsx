/**
 * PolicyDetailPage - View insurance policy details with documents.
 * Insurance Management Feature
 */

import { useTranslation } from 'react-i18next';
import { getActiveLocale } from '../../../lib/locale';
import type {
  InsuranceClaim,
  InsurancePolicy,
  InsuranceReminder,
  PolicyStatus,
  PolicyType,
} from '../types';

interface PolicyDocument {
  id: string;
  filename: string;
  contentType: string;
  sizeBytes: number;
  uploadedAt: string;
}

interface PolicyDetailPageProps {
  policy: InsurancePolicy;
  claims: InsuranceClaim[];
  reminders: InsuranceReminder[];
  documents: PolicyDocument[];
  isLoading?: boolean;
  onBack: () => void;
  onEdit: () => void;
  onFileClaim: () => void;
  onViewClaim: (id: string) => void;
  onAddReminder: (date: string, message: string) => void;
  onCompleteReminder: (id: string) => void;
  onUploadDocument: (file: File) => void;
  onDownloadDocument: (id: string) => void;
  onDeleteDocument: (id: string) => void;
}

const statusColors: Record<PolicyStatus, string> = {
  active: 'bg-green-100 text-green-800',
  expired: 'bg-red-100 text-red-800',
  cancelled: 'bg-gray-100 text-gray-800',
  pending: 'bg-yellow-100 text-yellow-800',
};

export function PolicyDetailPage({
  policy,
  claims,
  reminders,
  documents,
  isLoading,
  onBack,
  onEdit,
  onFileClaim,
  onViewClaim,
  onAddReminder: _onAddReminder,
  onCompleteReminder,
  onUploadDocument,
  onDownloadDocument,
  onDeleteDocument,
}: PolicyDetailPageProps) {
  // Note: onAddReminder is available via _onAddReminder for future use
  const { t } = useTranslation();

  const statusLabels: Record<PolicyStatus, string> = {
    active: t('insurance.statusActive'),
    expired: t('insurance.statusExpired'),
    cancelled: t('insurance.statusCancelled'),
    pending: t('insurance.statusPending'),
  };

  const typeLabels: Record<PolicyType, string> = {
    building: t('insurance.typeBuilding'),
    liability: t('insurance.typeLiability'),
    property: t('insurance.typeProperty'),
    flood: t('insurance.typeFlood'),
    earthquake: t('insurance.typeEarthquake'),
    umbrella: t('insurance.typeUmbrella'),
    directors_officers: t('insurance.typeDirectorsOfficers'),
    workers_comp: t('insurance.typeWorkersComp'),
    other: t('insurance.typeOther'),
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

  const daysUntilExpiry = Math.ceil(
    (new Date(policy.endDate).getTime() - new Date().getTime()) / (1000 * 60 * 60 * 24)
  );
  const isExpiringSoon = daysUntilExpiry > 0 && daysUntilExpiry <= 30;

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
          {t('insurance.backToPolicies')}
        </button>
        <div className="flex items-start justify-between">
          <div>
            <div className="flex items-center gap-2 mb-2">
              <span
                className={`px-2 py-1 text-sm font-medium rounded ${statusColors[policy.status]}`}
              >
                {statusLabels[policy.status]}
              </span>
              {isExpiringSoon && (
                <span className="px-2 py-1 text-sm font-medium rounded bg-orange-100 text-orange-800">
                  {t('insurance.expiringSoon', { days: daysUntilExpiry })}
                </span>
              )}
            </div>
            <h1 className="text-2xl font-bold text-gray-900">{policy.provider}</h1>
            <p className="text-gray-600">
              {typeLabels[policy.policyType]} - {policy.policyNumber}
            </p>
          </div>
          <div className="flex gap-2">
            {policy.status === 'active' && (
              <>
                <button
                  type="button"
                  onClick={onEdit}
                  className="px-3 py-1.5 text-sm border border-gray-300 rounded-lg hover:bg-gray-50"
                >
                  {t('common.edit')}
                </button>
                <button
                  type="button"
                  onClick={onFileClaim}
                  className="px-3 py-1.5 text-sm bg-blue-600 text-white rounded-lg hover:bg-blue-700"
                >
                  {t('insurance.fileClaim')}
                </button>
              </>
            )}
          </div>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Main Content */}
        <div className="lg:col-span-2 space-y-6">
          {/* Coverage Details */}
          <div className="bg-white rounded-lg shadow p-6">
            <h2 className="text-lg font-semibold text-gray-900 mb-4">
              {t('insurance.coverageDetails')}
            </h2>
            <div className="grid grid-cols-2 gap-4">
              <div>
                <p className="text-sm text-gray-500">{t('insurance.coverage')}</p>
                <p className="text-lg font-semibold">
                  {formatCurrency(policy.coverageAmount, policy.currency)}
                </p>
              </div>
              <div>
                <p className="text-sm text-gray-500">{t('insurance.deductible')}</p>
                <p className="text-lg font-semibold">
                  {formatCurrency(policy.deductible, policy.currency)}
                </p>
              </div>
              <div>
                <p className="text-sm text-gray-500">{t('insurance.premium')}</p>
                <p className="text-lg font-semibold">
                  {formatCurrency(policy.premium, policy.currency)}
                  <span className="text-sm font-normal text-gray-500">
                    {' '}
                    / {t(`insurance.frequency.${policy.premiumFrequency}`)}
                  </span>
                </p>
              </div>
              <div>
                <p className="text-sm text-gray-500">{t('insurance.policyPeriod')}</p>
                <p className="font-medium">
                  {new Date(policy.startDate).toLocaleDateString(getActiveLocale())} -{' '}
                  {new Date(policy.endDate).toLocaleDateString(getActiveLocale())}
                </p>
              </div>
            </div>
            {policy.coverageDetails && (
              <div className="mt-4 pt-4 border-t">
                <p className="text-sm text-gray-500 mb-2">{t('insurance.coverageIncludes')}</p>
                <p className="text-gray-700 whitespace-pre-wrap">{policy.coverageDetails}</p>
              </div>
            )}
          </div>

          {/* Provider Information */}
          <div className="bg-white rounded-lg shadow p-6">
            <h2 className="text-lg font-semibold text-gray-900 mb-4">
              {t('insurance.providerInfo')}
            </h2>
            <dl className="space-y-3">
              <div>
                <dt className="text-sm text-gray-500">{t('insurance.provider')}</dt>
                <dd className="font-medium">{policy.provider}</dd>
              </div>
              {policy.providerContact && (
                <div>
                  <dt className="text-sm text-gray-500">{t('insurance.contactName')}</dt>
                  <dd className="font-medium">{policy.providerContact}</dd>
                </div>
              )}
              {policy.providerPhone && (
                <div>
                  <dt className="text-sm text-gray-500">{t('insurance.phone')}</dt>
                  <dd className="font-medium">
                    <a
                      href={`tel:${policy.providerPhone}`}
                      className="text-blue-600 hover:text-blue-800"
                    >
                      {policy.providerPhone}
                    </a>
                  </dd>
                </div>
              )}
              {policy.providerEmail && (
                <div>
                  <dt className="text-sm text-gray-500">{t('insurance.email')}</dt>
                  <dd className="font-medium">
                    <a
                      href={`mailto:${policy.providerEmail}`}
                      className="text-blue-600 hover:text-blue-800"
                    >
                      {policy.providerEmail}
                    </a>
                  </dd>
                </div>
              )}
            </dl>
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

          {/* Related Claims */}
          <div className="bg-white rounded-lg shadow p-6">
            <h2 className="text-lg font-semibold text-gray-900 mb-4">
              {t('insurance.relatedClaims')} ({claims.length})
            </h2>
            {claims.length === 0 ? (
              <p className="text-gray-500">{t('insurance.noClaimsForPolicy')}</p>
            ) : (
              <ul className="divide-y">
                {claims.map((claim) => (
                  <li key={claim.id} className="py-3">
                    <button
                      type="button"
                      onClick={() => onViewClaim(claim.id)}
                      className="w-full text-left hover:bg-gray-50 rounded p-2 -m-2"
                    >
                      <div className="flex items-center justify-between">
                        <span className="font-medium">{claim.title}</span>
                        <span
                          className={`px-2 py-1 text-xs rounded ${
                            claim.status === 'approved'
                              ? 'bg-green-100 text-green-800'
                              : claim.status === 'denied'
                                ? 'bg-red-100 text-red-800'
                                : 'bg-yellow-100 text-yellow-800'
                          }`}
                        >
                          {claim.status}
                        </span>
                      </div>
                      <p className="text-sm text-gray-500 mt-1">
                        {t('insurance.incidentDate')}:{' '}
                        {new Date(claim.incidentDate).toLocaleDateString(getActiveLocale())}
                        {claim.claimAmount && (
                          <span> - {formatCurrency(claim.claimAmount, claim.currency)}</span>
                        )}
                      </p>
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </div>
        </div>

        {/* Sidebar */}
        <div className="space-y-6">
          {/* Policy Details */}
          <div className="bg-white rounded-lg shadow p-6">
            <h3 className="text-lg font-semibold text-gray-900 mb-4">
              {t('insurance.policyDetails')}
            </h3>
            <dl className="space-y-3 text-sm">
              <div>
                <dt className="text-gray-500">{t('insurance.policyNumber')}</dt>
                <dd className="font-medium">{policy.policyNumber}</dd>
              </div>
              <div>
                <dt className="text-gray-500">{t('insurance.policyType')}</dt>
                <dd className="font-medium">{typeLabels[policy.policyType]}</dd>
              </div>
              {policy.buildingName && (
                <div>
                  <dt className="text-gray-500">{t('insurance.building')}</dt>
                  <dd className="font-medium">{policy.buildingName}</dd>
                  {policy.buildingAddress && (
                    <dd className="text-gray-500">{policy.buildingAddress}</dd>
                  )}
                </div>
              )}
              <div>
                <dt className="text-gray-500">{t('insurance.createdAt')}</dt>
                <dd className="font-medium">
                  {new Date(policy.createdAt).toLocaleDateString(getActiveLocale())}
                </dd>
              </div>
              <div>
                <dt className="text-gray-500">{t('insurance.lastUpdated')}</dt>
                <dd className="font-medium">
                  {new Date(policy.updatedAt).toLocaleDateString(getActiveLocale())}
                </dd>
              </div>
            </dl>
          </div>

          {/* Reminders */}
          <div className="bg-white rounded-lg shadow p-6">
            <h3 className="text-lg font-semibold text-gray-900 mb-4">{t('insurance.reminders')}</h3>
            {reminders.length === 0 ? (
              <p className="text-sm text-gray-500">{t('insurance.noReminders')}</p>
            ) : (
              <ul className="space-y-3">
                {reminders.map((reminder) => (
                  <li
                    key={reminder.id}
                    className={`p-3 rounded border ${
                      reminder.isCompleted
                        ? 'bg-gray-50 border-gray-200'
                        : 'bg-yellow-50 border-yellow-200'
                    }`}
                  >
                    <div className="flex items-start justify-between">
                      <div>
                        <p
                          className={`text-sm ${reminder.isCompleted ? 'text-gray-500 line-through' : ''}`}
                        >
                          {reminder.message}
                        </p>
                        <p className="text-xs text-gray-400 mt-1">
                          {new Date(reminder.reminderDate).toLocaleDateString(getActiveLocale())}
                        </p>
                      </div>
                      {!reminder.isCompleted && (
                        <button
                          type="button"
                          onClick={() => onCompleteReminder(reminder.id)}
                          className="text-xs text-green-600 hover:text-green-800"
                        >
                          {t('common.confirm')}
                        </button>
                      )}
                    </div>
                  </li>
                ))}
              </ul>
            )}
          </div>

          {/* Description */}
          {policy.description && (
            <div className="bg-white rounded-lg shadow p-6">
              <h3 className="text-lg font-semibold text-gray-900 mb-4">
                {t('insurance.description')}
              </h3>
              <p className="text-sm text-gray-700 whitespace-pre-wrap">{policy.description}</p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
