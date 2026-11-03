/**
 * DisputeCard - displays a summary card for a dispute.
 * Epic 77: Dispute Resolution (Story 77.1, 77.3)
 */

import { useTranslation } from 'react-i18next';

export type DisputeCategory =
  | 'noise'
  | 'damage'
  | 'payment'
  | 'lease_terms'
  | 'common_area'
  | 'parking'
  | 'pets'
  | 'maintenance'
  | 'privacy'
  | 'harassment'
  | 'other';

export type DisputeStatus =
  | 'filed'
  | 'under_review'
  | 'mediation'
  | 'awaiting_response'
  | 'resolved'
  | 'escalated'
  | 'withdrawn'
  | 'closed';

export type DisputePriority = 'low' | 'medium' | 'high' | 'urgent';

export interface DisputeSummary {
  id: string;
  referenceNumber: string;
  category: DisputeCategory;
  title: string;
  status: DisputeStatus;
  priority: DisputePriority;
  filedByName: string;
  assignedToName?: string;
  partyCount: number;
  createdAt: string;
  updatedAt: string;
}

const statusColors: Record<DisputeStatus, string> = {
  filed: 'bg-blue-100 text-blue-800',
  under_review: 'bg-yellow-100 text-yellow-800',
  mediation: 'bg-purple-100 text-purple-800',
  awaiting_response: 'bg-orange-100 text-orange-800',
  resolved: 'bg-green-100 text-green-800',
  escalated: 'bg-red-100 text-red-800',
  withdrawn: 'bg-gray-100 text-gray-600',
  closed: 'bg-gray-100 text-gray-800',
};

/** i18n keys for dispute status labels. */
const statusLabelKeys: Record<DisputeStatus, string> = {
  filed: 'disputes.statusFiled',
  under_review: 'disputes.statusUnderReview',
  mediation: 'disputes.statusInMediation',
  awaiting_response: 'disputes.statusAwaitingResponse',
  resolved: 'disputes.statusResolved',
  escalated: 'disputes.statusEscalated',
  withdrawn: 'disputes.statusWithdrawn',
  closed: 'disputes.statusClosed',
};

const priorityColors: Record<DisputePriority, string> = {
  low: 'text-gray-500',
  medium: 'text-blue-500',
  high: 'text-orange-500',
  urgent: 'text-red-600 font-bold',
};

/** i18n keys for dispute priority labels. */
const priorityLabelKeys: Record<DisputePriority, string> = {
  low: 'disputes.priorityLow',
  medium: 'disputes.priorityMedium',
  high: 'disputes.priorityHigh',
  urgent: 'disputes.priorityUrgent',
};

/** i18n keys for dispute category labels. */
const categoryLabelKeys: Record<DisputeCategory, string> = {
  noise: 'disputes.categoryNoise',
  damage: 'disputes.categoryPropertyDamage',
  payment: 'disputes.categoryPayment',
  lease_terms: 'disputes.categoryLeaseTerms',
  common_area: 'disputes.categoryCommonArea',
  parking: 'disputes.categoryParking',
  pets: 'disputes.categoryPets',
  maintenance: 'disputes.categoryMaintenance',
  privacy: 'disputes.categoryPrivacy',
  harassment: 'disputes.categoryHarassment',
  other: 'disputes.categoryOther',
};

const categoryIcons: Record<DisputeCategory, string> = {
  noise: 'volume-high',
  damage: 'hammer',
  payment: 'currency-dollar',
  lease_terms: 'document-text',
  common_area: 'building-office',
  parking: 'truck',
  pets: 'heart',
  maintenance: 'wrench',
  privacy: 'eye-slash',
  harassment: 'exclamation-triangle',
  other: 'question-mark-circle',
};

interface DisputeCardProps {
  dispute: DisputeSummary;
  onView: (id: string) => void;
  onManage?: (id: string) => void;
}

export function DisputeCard({ dispute, onView, onManage }: DisputeCardProps) {
  const { t } = useTranslation();
  const daysAgo = Math.floor(
    (Date.now() - new Date(dispute.createdAt).getTime()) / (1000 * 60 * 60 * 24)
  );

  return (
    <div className="bg-white rounded-lg shadow p-4 hover:shadow-md transition-shadow">
      <div className="flex items-start justify-between">
        <div className="flex-1">
          <div className="flex items-center gap-2 mb-1">
            <span className="text-sm font-mono text-gray-500">{dispute.referenceNumber}</span>
            <span
              className={`px-2 py-0.5 text-xs font-medium rounded ${statusColors[dispute.status]}`}
            >
              {t(statusLabelKeys[dispute.status])}
            </span>
          </div>
          <button
            type="button"
            className="text-lg font-semibold text-gray-900 hover:text-blue-600 cursor-pointer text-left"
            onClick={() => onView(dispute.id)}
          >
            {dispute.title}
          </button>
          <div className="mt-2 flex items-center gap-4 text-sm text-gray-500">
            <span>{t(categoryLabelKeys[dispute.category])}</span>
            <span className={priorityColors[dispute.priority]}>
              {t('disputes.priorityWithLabel', {
                priority: t(priorityLabelKeys[dispute.priority]),
              })}
            </span>
            <span>{t('disputes.partyCount', { count: dispute.partyCount })}</span>
          </div>
          <div className="mt-2 text-sm text-gray-500">
            <span>{t('disputes.filedByName', { name: dispute.filedByName })}</span>
            {dispute.assignedToName && (
              <span className="ml-3">
                {t('disputes.assignedToName', { name: dispute.assignedToName })}
              </span>
            )}
            <span className="ml-3">
              {daysAgo === 0 ? t('disputes.today') : t('disputes.filedDaysAgo', { count: daysAgo })}
            </span>
          </div>
        </div>
        <div className="flex gap-2">
          <button
            type="button"
            onClick={() => onView(dispute.id)}
            className="px-3 py-1.5 text-sm border border-gray-300 rounded-lg hover:bg-gray-50"
          >
            {t('disputes.view')}
          </button>
          {onManage &&
            ['filed', 'under_review', 'mediation', 'awaiting_response'].includes(
              dispute.status
            ) && (
              <button
                type="button"
                onClick={() => onManage(dispute.id)}
                className="px-3 py-1.5 text-sm bg-blue-600 text-white rounded-lg hover:bg-blue-700"
              >
                {t('disputes.manage')}
              </button>
            )}
        </div>
      </div>
    </div>
  );
}

export { categoryIcons, categoryLabelKeys, priorityLabelKeys, statusLabelKeys };
