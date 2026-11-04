/**
 * ViewOutagePage - Detail view for a single outage.
 * UC-12: Utility Outages
 */

import type { OutageCommodity, OutageSeverity, OutageStatus } from '@ppt/api-client';
import { useTranslation } from 'react-i18next';

export interface OutageDetail {
  id: string;
  organizationId: string;
  createdBy: string;
  title: string;
  description: string;
  commodity: OutageCommodity;
  severity: OutageSeverity;
  status: OutageStatus;
  buildingIds: string[];
  scheduledStart: string;
  scheduledEnd?: string;
  actualStart?: string;
  actualEnd?: string;
  resolutionNotes?: string;
  cancelReason?: string;
  createdAt: string;
  updatedAt: string;
  // Joined fields
  createdByName: string;
  buildingNames: string[];
}

interface ViewOutagePageProps {
  outage: OutageDetail;
  isLoading?: boolean;
  onEdit: () => void;
  onStart: () => void;
  onResolve: (notes: string) => void;
  onCancel: (reason: string) => void;
  onBack: () => void;
}

const statusColors: Record<OutageStatus, string> = {
  planned: 'bg-blue-100 text-blue-800',
  ongoing: 'bg-red-100 text-red-800',
  resolved: 'bg-green-100 text-green-800',
  cancelled: 'bg-gray-100 text-gray-800',
};

const severityColors: Record<OutageSeverity, string> = {
  informational: 'bg-gray-100 text-gray-800',
  minor: 'bg-blue-100 text-blue-800',
  major: 'bg-orange-100 text-orange-800',
  critical: 'bg-red-100 text-red-800',
};

const commodityIcons: Record<OutageCommodity, string> = {
  electricity: '⚡',
  gas: '🔥',
  water: '💧',
  heating: '🌡️',
  internet: '🌐',
  other: '⚠️',
};

export function ViewOutagePage({
  outage,
  isLoading,
  onEdit,
  onStart,
  onResolve,
  onCancel,
  onBack,
}: ViewOutagePageProps) {
  const { t, i18n } = useTranslation();

  const canEdit = outage.status === 'planned';
  const canStart = outage.status === 'planned';
  const canResolve = outage.status === 'ongoing';
  const canCancel = outage.status === 'planned' || outage.status === 'ongoing';

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-12">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
      </div>
    );
  }

  return (
    <div className="max-w-3xl mx-auto px-4 py-8">
      {/* Back button */}
      <button
        type="button"
        onClick={onBack}
        className="mb-4 text-blue-600 hover:text-blue-800 flex items-center gap-1"
      >
        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
        </svg>
        {t('outages.backToList')}
      </button>

      <div className="bg-white rounded-lg shadow">
        {/* Header */}
        <div className="p-6 border-b">
          <div className="flex items-start justify-between">
            <div className="flex items-start gap-3">
              <span className="text-3xl" role="img" aria-label={outage.commodity}>
                {commodityIcons[outage.commodity]}
              </span>
              <div>
                <h1 className="text-2xl font-bold text-gray-900">{outage.title}</h1>
                <div className="mt-1 flex items-center gap-3 text-sm text-gray-500">
                  <span>
                    {t('outages.createdBy')} {outage.createdByName}
                  </span>
                  <span>•</span>
                  <span>{new Date(outage.createdAt).toLocaleDateString(i18n.language)}</span>
                </div>
              </div>
            </div>
            <div className="flex items-center gap-2">
              <span
                className={`px-2 py-1 text-xs font-medium rounded ${severityColors[outage.severity]}`}
              >
                {t(`outages.severity.${outage.severity}`)}
              </span>
              <span
                className={`px-2 py-1 text-xs font-medium rounded ${statusColors[outage.status]}`}
              >
                {t(`outages.status.${outage.status}`)}
              </span>
            </div>
          </div>

          {/* Stats */}
          <div className="mt-4 flex items-center gap-6 text-sm">
            <span>
              <strong>{outage.buildingIds.length}</strong> {t('outages.buildingsAffected')}
            </span>
            <span>{t(`outages.commodity.${outage.commodity}`)}</span>
          </div>
        </div>

        {/* Schedule info */}
        <div className="p-6 border-b bg-gray-50">
          <h2 className="text-sm font-semibold text-gray-700 mb-3">{t('outages.schedule')}</h2>
          <div className="grid grid-cols-2 gap-4 text-sm">
            <div>
              <span className="text-gray-500">{t('outages.scheduledStart')}:</span>
              <span className="ml-2 text-gray-900">
                {new Date(outage.scheduledStart).toLocaleString(i18n.language)}
              </span>
            </div>
            {outage.scheduledEnd && (
              <div>
                <span className="text-gray-500">{t('outages.scheduledEnd')}:</span>
                <span className="ml-2 text-gray-900">
                  {new Date(outage.scheduledEnd).toLocaleString(i18n.language)}
                </span>
              </div>
            )}
            {outage.actualStart && (
              <div>
                <span className="text-gray-500">{t('outages.actualStart')}:</span>
                <span className="ml-2 text-gray-900">
                  {new Date(outage.actualStart).toLocaleString(i18n.language)}
                </span>
              </div>
            )}
            {outage.actualEnd && (
              <div>
                <span className="text-gray-500">{t('outages.actualEnd')}:</span>
                <span className="ml-2 text-gray-900">
                  {new Date(outage.actualEnd).toLocaleString(i18n.language)}
                </span>
              </div>
            )}
          </div>
        </div>

        {/* Description */}
        <div className="p-6 border-b">
          <h2 className="text-sm font-semibold text-gray-700 mb-3">{t('outages.description')}</h2>
          <div className="prose prose-sm max-w-none">
            <div className="whitespace-pre-wrap text-gray-700">{outage.description}</div>
          </div>
        </div>

        {/* Affected buildings */}
        <div className="p-6 border-b">
          <h2 className="text-sm font-semibold text-gray-700 mb-3">
            {t('outages.affectedBuildings')}
          </h2>
          <div className="flex flex-wrap gap-2">
            {outage.buildingNames.map((name, index) => (
              <span key={index} className="px-2 py-1 text-sm bg-gray-100 text-gray-700 rounded">
                {name}
              </span>
            ))}
          </div>
        </div>

        {/* Resolution notes */}
        {outage.resolutionNotes && (
          <div className="p-6 border-b bg-green-50">
            <h2 className="text-sm font-semibold text-green-700 mb-3">
              {t('outages.resolutionNotes')}
            </h2>
            <p className="text-sm text-green-800">{outage.resolutionNotes}</p>
          </div>
        )}

        {/* Cancel reason */}
        {outage.cancelReason && (
          <div className="p-6 border-b bg-gray-50">
            <h2 className="text-sm font-semibold text-gray-700 mb-3">
              {t('outages.cancelReason')}
            </h2>
            <p className="text-sm text-gray-600">{outage.cancelReason}</p>
          </div>
        )}

        {/* Actions */}
        <div className="p-6 bg-gray-50 flex items-center gap-3 flex-wrap">
          {canEdit && (
            <button
              type="button"
              onClick={onEdit}
              className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50"
            >
              {t('common.edit')}
            </button>
          )}
          {canStart && (
            <button
              type="button"
              onClick={onStart}
              className="px-4 py-2 text-sm font-medium text-white bg-orange-600 rounded-md hover:bg-orange-700"
            >
              {t('outages.startOutage')}
            </button>
          )}
          {canResolve && (
            <button
              type="button"
              onClick={() => onResolve('')}
              className="px-4 py-2 text-sm font-medium text-white bg-green-600 rounded-md hover:bg-green-700"
            >
              {t('outages.markResolved')}
            </button>
          )}
          {canCancel && (
            <button
              type="button"
              onClick={() => onCancel('')}
              className="px-4 py-2 text-sm font-medium text-red-600 bg-white border border-red-300 rounded-md hover:bg-red-50"
            >
              {t('outages.cancelOutage')}
            </button>
          )}
          <button
            type="button"
            onClick={onBack}
            className="px-4 py-2 text-sm font-medium text-gray-600 bg-white border border-gray-300 rounded-md hover:bg-gray-50"
          >
            {t('common.back')}
          </button>
        </div>
      </div>
    </div>
  );
}
