/**
 * PolicyCard component - displays an insurance policy summary card.
 * Insurance Management Feature
 */

import { useTranslation } from 'react-i18next';
import { getActiveLocale } from '../../../lib/locale';
import type { InsurancePolicy, PolicyStatus, PolicyType } from '../types';

interface PolicyCardProps {
  policy: InsurancePolicy;
  onView?: (id: string) => void;
  onEdit?: (id: string) => void;
  onFileClaim?: (id: string) => void;
}

const statusColors: Record<PolicyStatus, string> = {
  active: 'bg-green-100 text-green-800',
  expired: 'bg-red-100 text-red-800',
  cancelled: 'bg-gray-100 text-gray-800',
  pending: 'bg-yellow-100 text-yellow-800',
};

const typeColors: Record<PolicyType, string> = {
  building: 'bg-blue-50 text-blue-700',
  liability: 'bg-purple-50 text-purple-700',
  property: 'bg-indigo-50 text-indigo-700',
  flood: 'bg-cyan-50 text-cyan-700',
  earthquake: 'bg-orange-50 text-orange-700',
  umbrella: 'bg-teal-50 text-teal-700',
  directors_officers: 'bg-pink-50 text-pink-700',
  workers_comp: 'bg-amber-50 text-amber-700',
  other: 'bg-gray-50 text-gray-700',
};

export function PolicyCard({ policy, onView, onEdit, onFileClaim }: PolicyCardProps) {
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

  const daysUntilExpiry = Math.ceil(
    (new Date(policy.endDate).getTime() - new Date().getTime()) / (1000 * 60 * 60 * 24)
  );
  const isExpiringSoon = daysUntilExpiry > 0 && daysUntilExpiry <= 30;

  return (
    <div className="bg-white rounded-lg shadow p-4 hover:shadow-md transition-shadow">
      <div className="flex items-start justify-between">
        <div className="flex-1">
          <div className="flex items-center gap-2 flex-wrap">
            <span
              className={`px-2 py-1 text-xs font-medium rounded ${typeColors[policy.policyType]}`}
            >
              {typeLabels[policy.policyType]}
            </span>
            <span
              className={`px-2 py-1 text-xs font-medium rounded ${statusColors[policy.status]}`}
            >
              {statusLabels[policy.status]}
            </span>
            {isExpiringSoon && (
              <span className="px-2 py-1 text-xs font-medium rounded bg-orange-100 text-orange-800">
                {t('insurance.expiringSoon', { days: daysUntilExpiry })}
              </span>
            )}
          </div>
          <h3 className="mt-2 text-lg font-semibold text-gray-900">{policy.provider}</h3>
          <p className="text-sm text-gray-600">
            {t('insurance.policyNumber')}: {policy.policyNumber}
          </p>
          {policy.buildingName && <p className="text-sm text-gray-500">{policy.buildingName}</p>}
        </div>
      </div>

      <div className="mt-4 grid grid-cols-2 gap-4 text-sm">
        <div>
          <span className="text-gray-500">{t('insurance.coverage')}:</span>
          <p className="font-medium">{formatCurrency(policy.coverageAmount, policy.currency)}</p>
        </div>
        <div>
          <span className="text-gray-500">{t('insurance.premium')}:</span>
          <p className="font-medium">
            {formatCurrency(policy.premium, policy.currency)}/
            {t(`insurance.frequency.${policy.premiumFrequency}`)}
          </p>
        </div>
        <div>
          <span className="text-gray-500">{t('insurance.deductible')}:</span>
          <p className="font-medium">{formatCurrency(policy.deductible, policy.currency)}</p>
        </div>
        <div>
          <span className="text-gray-500">{t('insurance.expires')}:</span>
          <p className={`font-medium ${isExpiringSoon ? 'text-orange-600' : ''}`}>
            {new Date(policy.endDate).toLocaleDateString(getActiveLocale())}
          </p>
        </div>
      </div>

      <div className="mt-4 flex items-center gap-2 border-t pt-3">
        <button
          type="button"
          onClick={() => onView?.(policy.id)}
          className="text-sm text-blue-600 hover:text-blue-800"
        >
          {t('common.view')}
        </button>
        {policy.status === 'active' && (
          <>
            <button
              type="button"
              onClick={() => onEdit?.(policy.id)}
              className="text-sm text-gray-600 hover:text-gray-800"
            >
              {t('common.edit')}
            </button>
            <button
              type="button"
              onClick={() => onFileClaim?.(policy.id)}
              className="text-sm text-green-600 hover:text-green-800"
            >
              {t('insurance.fileClaim')}
            </button>
          </>
        )}
      </div>
    </div>
  );
}
