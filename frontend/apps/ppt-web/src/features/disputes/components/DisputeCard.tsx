/**
 * DisputeCard - displays a summary card for a dispute.
 * Epic 77: Dispute Resolution (Story 77.1, 77.3)
 */

import { useTranslation } from 'react-i18next';
import {
  daysSince,
  disputeCategoryKeys,
  disputePriorityKeys,
  disputeStatusKeys,
} from '../utils/labels';

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

const priorityColors: Record<DisputePriority, string> = {
  low: 'text-gray-500',
  medium: 'text-blue-500',
  high: 'text-orange-500',
  urgent: 'text-red-600 font-bold',
};

interface DisputeCardProps {
  dispute: DisputeSummary;
  onView: (id: string) => void;
  onManage?: (id: string) => void;
}

export function DisputeCard({ dispute, onView, onManage }: DisputeCardProps) {
  const { t, i18n } = useTranslation();
  const daysAgo = daysSince(dispute.createdAt);
  const age =
    daysAgo <= 0
      ? t('disputes.today')
      : new Intl.RelativeTimeFormat(i18n.language, { numeric: 'auto' }).format(-daysAgo, 'day');

  return (
    <div className="bg-white rounded-lg shadow p-4 hover:shadow-md transition-shadow">
      <div className="flex items-start justify-between">
        <div className="flex-1">
          <div className="flex items-center gap-2 mb-1">
            <span className="text-sm font-mono text-gray-500">{dispute.referenceNumber}</span>
            <span
              className={`px-2 py-0.5 text-xs font-medium rounded ${statusColors[dispute.status]}`}
            >
              {t(disputeStatusKeys[dispute.status])}
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
            <span>{t(disputeCategoryKeys[dispute.category])}</span>
            <span className={priorityColors[dispute.priority]}>
              {t('disputes.priorityWithLabel', {
                priority: t(disputePriorityKeys[dispute.priority]),
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
            <span className="ml-3">{age}</span>
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
