/**
 * ClaimCard component - displays an insurance claim summary card.
 * Insurance Management Feature
 */

import { useTranslation } from 'react-i18next';
import { getActiveLocale } from '../../../lib/locale';
import type { ClaimStatus, ClaimType, InsuranceClaim } from '../types';

interface ClaimCardProps {
  claim: InsuranceClaim;
  onView?: (id: string) => void;
}

const statusColors: Record<ClaimStatus, string> = {
  submitted: 'bg-blue-100 text-blue-800',
  under_review: 'bg-yellow-100 text-yellow-800',
  approved: 'bg-green-100 text-green-800',
  denied: 'bg-red-100 text-red-800',
  settled: 'bg-purple-100 text-purple-800',
  closed: 'bg-gray-100 text-gray-800',
};

const typeColors: Record<ClaimType, string> = {
  property_damage: 'bg-orange-50 text-orange-700',
  liability: 'bg-purple-50 text-purple-700',
  theft: 'bg-red-50 text-red-700',
  water_damage: 'bg-cyan-50 text-cyan-700',
  fire_damage: 'bg-amber-50 text-amber-700',
  natural_disaster: 'bg-teal-50 text-teal-700',
  personal_injury: 'bg-pink-50 text-pink-700',
  other: 'bg-gray-50 text-gray-700',
};

export function ClaimCard({ claim, onView }: ClaimCardProps) {
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

  const daysSinceIncident = Math.floor(
    (new Date().getTime() - new Date(claim.incidentDate).getTime()) / (1000 * 60 * 60 * 24)
  );

  return (
    <div className="bg-white rounded-lg shadow p-4 hover:shadow-md transition-shadow">
      <div className="flex items-start justify-between">
        <div className="flex-1">
          <div className="flex items-center gap-2 flex-wrap">
            <span
              className={`px-2 py-1 text-xs font-medium rounded ${typeColors[claim.claimType]}`}
            >
              {typeLabels[claim.claimType]}
            </span>
            <span className={`px-2 py-1 text-xs font-medium rounded ${statusColors[claim.status]}`}>
              {statusLabels[claim.status]}
            </span>
          </div>
          <h3 className="mt-2 text-lg font-semibold text-gray-900">{claim.title}</h3>
          {claim.claimNumber && (
            <p className="text-sm text-gray-600">
              {t('insurance.claimNumber')}: {claim.claimNumber}
            </p>
          )}
          {claim.policyProvider && (
            <p className="text-sm text-gray-500">
              {t('insurance.policy')}: {claim.policyProvider} ({claim.policyNumber})
            </p>
          )}
        </div>
      </div>

      <div className="mt-3 text-sm text-gray-600 line-clamp-2">{claim.description}</div>

      <div className="mt-4 grid grid-cols-2 gap-4 text-sm">
        <div>
          <span className="text-gray-500">{t('insurance.incidentDate')}:</span>
          <p className="font-medium">
            {new Date(claim.incidentDate).toLocaleDateString(getActiveLocale())}
            <span className="text-gray-400 text-xs ml-1">
              ({daysSinceIncident} {t('insurance.daysAgo')})
            </span>
          </p>
        </div>
        <div>
          <span className="text-gray-500">{t('insurance.reportedDate')}:</span>
          <p className="font-medium">
            {new Date(claim.reportedDate).toLocaleDateString(getActiveLocale())}
          </p>
        </div>
        {claim.claimAmount && (
          <div>
            <span className="text-gray-500">{t('insurance.claimAmount')}:</span>
            <p className="font-medium">{formatCurrency(claim.claimAmount, claim.currency)}</p>
          </div>
        )}
        {claim.approvedAmount && (
          <div>
            <span className="text-gray-500">{t('insurance.approvedAmount')}:</span>
            <p className="font-medium text-green-600">
              {formatCurrency(claim.approvedAmount, claim.currency)}
            </p>
          </div>
        )}
      </div>

      {claim.buildingName && (
        <div className="mt-3 text-sm text-gray-500">
          <span>{claim.buildingName}</span>
          {claim.unitDesignation && <span> - {claim.unitDesignation}</span>}
        </div>
      )}

      <div className="mt-4 flex items-center gap-2 border-t pt-3">
        <button
          type="button"
          onClick={() => onView?.(claim.id)}
          className="text-sm text-blue-600 hover:text-blue-800"
        >
          {t('common.view')}
        </button>
      </div>
    </div>
  );
}
