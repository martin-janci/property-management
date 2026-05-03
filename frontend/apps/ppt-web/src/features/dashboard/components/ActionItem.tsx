/**
 * Individual action item card in the action queue.
 * Displays action details and inline action buttons.
 *
 * @module features/dashboard/components/ActionItem
 */

import { forwardRef } from 'react';
import { useTranslation } from 'react-i18next';
import type { ActionButton, ActionItem as ActionItemType } from '../hooks/useActionQueue';
import './ActionItem.css';

interface ActionItemProps {
  item: ActionItemType;
  isSelected?: boolean;
  isHighlighted?: boolean;
  isExecuting?: boolean;
  onAction: (itemId: string, action: ActionButton['action']) => void;
  onSelect?: (itemId: string) => void;
}

const typeIcons: Record<ActionItemType['type'], string> = {
  fault_pending: '🔧',
  fault_escalated: '⚠️',
  approval_pending: '📋',
  vote_active: '🗳️',
  message_unread: '✉️',
  meter_due: '📊',
  person_months_due: '👥',
  announcement_unread: '📢',
};

export const ActionItem = forwardRef<HTMLDivElement, ActionItemProps>(function ActionItem(
  { item, isSelected = false, isHighlighted = false, isExecuting = false, onAction, onSelect },
  ref
) {
  const { t } = useTranslation();

  const formatTimeAgo = (dateString: string) => {
    const date = new Date(dateString);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / (1000 * 60));
    const diffHours = Math.floor(diffMs / (1000 * 60 * 60));
    const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

    if (diffMins < 60) {
      return t('dashboard.minutesAgo', { count: diffMins });
    }
    if (diffHours < 24) {
      return t('dashboard.hoursAgo', { count: diffHours });
    }
    return t('dashboard.daysAgo', { count: diffDays });
  };

  const formatDueDate = (dateString: string) => {
    const date = new Date(dateString);
    const now = new Date();
    const diffMs = date.getTime() - now.getTime();
    const diffDays = Math.ceil(diffMs / (1000 * 60 * 60 * 24));

    if (diffDays < 0) {
      return t('dashboard.overdue', { days: Math.abs(diffDays) });
    }
    if (diffDays === 0) {
      return t('dashboard.dueToday');
    }
    if (diffDays === 1) {
      return t('dashboard.dueTomorrow');
    }
    return t('dashboard.dueInDays', { days: diffDays });
  };

  const rootClass = [
    'action-item',
    `action-item--${item.priority}`,
    isSelected ? 'action-item--selected' : '',
    isHighlighted ? 'action-item--highlighted' : '',
    isExecuting ? 'action-item--executing' : '',
  ]
    .filter(Boolean)
    .join(' ');

  return (
    <div
      ref={ref}
      className={rootClass}
      onClick={() => onSelect?.(item.id)}
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          onSelect?.(item.id);
        }
      }}
      role="button"
      tabIndex={0}
      aria-selected={isSelected}
      aria-busy={isExecuting}
    >
      <div className="action-item__body">
        <div className="action-item__main">
          <span className="action-item__icon" aria-hidden="true">
            {typeIcons[item.type]}
          </span>

          <div className="action-item__content">
            <div className="action-item__title-row">
              <h3 className="action-item__title">{item.title}</h3>
              <span
                className={`action-item__priority-badge action-item__priority-badge--${item.priority}`}
              >
                {t(`dashboard.priority.${item.priority}`)}
              </span>
            </div>

            <p className="action-item__description">{item.description}</p>

            <div className="action-item__meta">
              <span>{formatTimeAgo(item.createdAt)}</span>
              {item.dueDate && (
                <span
                  className={
                    item.dueDate < new Date().toISOString() ? 'action-item__due--overdue' : ''
                  }
                >
                  {formatDueDate(item.dueDate)}
                </span>
              )}
            </div>
          </div>
        </div>

        <div className="action-item__actions">
          {item.actions.map((action) => (
            <button
              key={action.id}
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                onAction(item.id, action.action);
              }}
              disabled={isExecuting}
              className={`action-item__btn action-item__btn--${action.variant}`}
            >
              {action.label}
            </button>
          ))}
        </div>
      </div>
    </div>
  );
});
