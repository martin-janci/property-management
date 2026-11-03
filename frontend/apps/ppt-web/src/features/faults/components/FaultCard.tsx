/**
 * FaultCard component - displays a fault summary card in lists.
 * Epic 4: Fault Reporting & Resolution (UC-03)
 */
import './FaultCard.css';
import { useTranslation } from 'react-i18next';
import { faultCategoryKeys, faultPriorityKeys, faultStatusKeys } from '../utils/faultLabels';

export type FaultStatus =
  | 'new'
  | 'triaged'
  | 'in_progress'
  | 'waiting_parts'
  | 'scheduled'
  | 'resolved'
  | 'closed'
  | 'reopened';

export type FaultPriority = 'low' | 'medium' | 'high' | 'urgent';

export type FaultCategory =
  | 'plumbing'
  | 'electrical'
  | 'heating'
  | 'structural'
  | 'exterior'
  | 'elevator'
  | 'common_area'
  | 'security'
  | 'cleaning'
  | 'other';

export interface FaultSummary {
  id: string;
  buildingId: string;
  unitId?: string;
  title: string;
  category: FaultCategory;
  priority: FaultPriority;
  status: FaultStatus;
  createdAt: string;
}

interface FaultCardProps {
  fault: FaultSummary;
  onView?: (id: string) => void;
  onEdit?: (id: string) => void;
  onTriage?: (id: string) => void;
  /** Delete the fault (only available for 'new' status faults). */
  onDelete?: (id: string) => void;
}

const statusPillClass: Record<FaultStatus, string> = {
  new: 'fault-status-pill fault-status-pill--new',
  triaged: 'fault-status-pill fault-status-pill--triaged',
  in_progress: 'fault-status-pill fault-status-pill--in_progress',
  waiting_parts: 'fault-status-pill fault-status-pill--waiting_parts',
  scheduled: 'fault-status-pill fault-status-pill--scheduled',
  resolved: 'fault-status-pill fault-status-pill--resolved',
  closed: 'fault-status-pill fault-status-pill--closed',
  reopened: 'fault-status-pill fault-status-pill--reopened',
};

const priorityPillClass: Record<FaultPriority, string> = {
  low: 'fault-priority-pill fault-priority-pill--low',
  medium: 'fault-priority-pill fault-priority-pill--medium',
  high: 'fault-priority-pill fault-priority-pill--high',
  urgent: 'fault-priority-pill fault-priority-pill--urgent',
};

export function FaultCard({ fault, onView, onEdit, onTriage, onDelete }: FaultCardProps) {
  const { t, i18n } = useTranslation();
  const canEdit = fault.status === 'new';
  const canTriage = fault.status === 'new';
  const canDelete = fault.status === 'new';

  return (
    <div className="fault-card">
      <div className="fault-card__header">
        <div className="fault-card__title-row">
          {fault.priority === 'urgent' && (
            <span className="fault-card__urgent-icon" title={t('faults.priorityUrgent')}>
              <svg
                width="16"
                height="16"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2.5"
                strokeLinecap="round"
                strokeLinejoin="round"
                aria-label={t('aria.urgent')}
              >
                <title>{t('faults.priorityUrgent')}</title>
                <path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z" />
                <line x1="12" y1="9" x2="12" y2="13" />
                <line x1="12" y1="17" x2="12.01" y2="17" />
              </svg>
            </span>
          )}
          <h3 className="fault-card__title">{fault.title}</h3>
        </div>
      </div>

      <div className="fault-card__meta">
        <span className={statusPillClass[fault.status]}>{t(faultStatusKeys[fault.status])}</span>
        <span className={priorityPillClass[fault.priority]}>
          {t(faultPriorityKeys[fault.priority])}
        </span>
        <span className="fault-card__category">{t(faultCategoryKeys[fault.category])}</span>
      </div>

      <p className="fault-card__date">
        {t('faults.reportedDate', {
          date: new Date(fault.createdAt).toLocaleDateString(i18n.language),
        })}
      </p>

      <div className="fault-card__actions">
        <button type="button" onClick={() => onView?.(fault.id)} className="fault-card__action">
          {t('common.view')}
        </button>
        {canEdit && (
          <button
            type="button"
            onClick={() => onEdit?.(fault.id)}
            className="fault-card__action fault-card__action--secondary"
          >
            {t('common.edit')}
          </button>
        )}
        {canTriage && (
          <button type="button" onClick={() => onTriage?.(fault.id)} className="fault-card__action">
            {t('faults.triage')}
          </button>
        )}
        {canDelete && (
          <button
            type="button"
            onClick={() => onDelete?.(fault.id)}
            className="fault-card__action fault-card__action--delete"
          >
            {t('common.delete')}
          </button>
        )}
      </div>
    </div>
  );
}
