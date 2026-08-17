/**
 * Content Moderation Case Card Component (Epic 67, Story 67.4).
 *
 * Displays a content moderation case with status, actions, and appeal info.
 */

import type React from 'react';
import { useTranslation } from 'react-i18next';

export type ModerationStatus =
  | 'pending'
  | 'under_review'
  | 'approved'
  | 'removed'
  | 'restricted'
  | 'warned'
  | 'appealed'
  | 'appeal_approved'
  | 'appeal_rejected';

export type ModeratedContentType =
  | 'listing'
  | 'listing_photo'
  | 'user_profile'
  | 'review'
  | 'comment'
  | 'message'
  | 'announcement'
  | 'document'
  | 'community_post';

export type ViolationType =
  | 'spam'
  | 'harassment'
  | 'hate_speech'
  | 'violence'
  | 'illegal_content'
  | 'misinformation'
  | 'fraud'
  | 'privacy'
  | 'intellectual_property'
  | 'inappropriate_content'
  | 'other';

export type ModerationActionType =
  | 'remove'
  | 'restrict'
  | 'warn'
  | 'approve'
  | 'ignore'
  | 'escalate';

export interface ContentOwnerInfo {
  user_id: string;
  name: string;
  // `previous_violations` is intentionally omitted: the moderation API does not
  // return a prior-violation count, so it must not be surfaced (or defaulted) here.
}

export interface ModerationCase {
  id: string;
  content_type: ModeratedContentType;
  content_id: string;
  content_preview?: string;
  content_owner: ContentOwnerInfo;
  violation_type?: ViolationType;
  report_reason?: string;
  status: ModerationStatus;
  priority: number;
  assigned_to_name?: string;
  decision?: ModerationActionType;
  decision_rationale?: string;
  appeal_filed: boolean;
  appeal_reason?: string;
  created_at: string;
  age_hours: number;
}

export interface ModerationCaseCardProps {
  case_: ModerationCase;
  onAssign?: (caseId: string) => void;
  onTakeAction?: (caseId: string) => void;
  onViewContent?: (caseId: string) => void;
  onDecideAppeal?: (caseId: string) => void;
  showActions?: boolean;
  isModerator?: boolean;
}

// Fallback humanizer for enum values that lack an explicit translation key.
const humanize = (value: string): string =>
  value
    .split('_')
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(' ');

const formatAge = (hours: number): string => {
  if (hours < 1) {
    return `${Math.round(hours * 60)}m`;
  }
  if (hours < 24) {
    return `${Math.round(hours)}h`;
  }
  const days = Math.floor(hours / 24);
  return `${days}d`;
};

const formatDate = (dateStr: string): string => {
  return new Date(dateStr).toLocaleDateString('en-GB', {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
};

export const ModerationCaseCard: React.FC<ModerationCaseCardProps> = ({
  case_,
  onAssign,
  onTakeAction,
  onViewContent,
  onDecideAppeal,
  showActions = true,
  isModerator = false,
}) => {
  const { t } = useTranslation();

  const getStatusLabel = (status: ModerationStatus): string =>
    t(`moderation.status.${status}`, humanize(status));
  const getContentTypeLabel = (type: ModeratedContentType): string =>
    t(`moderation.contentType.${type}`, humanize(type));
  const getViolationTypeLabel = (type?: ViolationType): string =>
    type
      ? t(`moderation.violationType.${type}`, humanize(type))
      : t('moderation.violationType.notSpecified');
  const getActionLabel = (action?: ModerationActionType): string =>
    action ? t(`moderation.actionType.${action}`, humanize(action)) : '';
  const getPriorityLabel = (priority: number): string =>
    t(`moderation.priorityLabel.${priority}`, t('moderation.priorityLabel.fallback', { priority }));

  const isOverdue = case_.age_hours >= 24 && case_.status === 'pending';
  const canTakeAction =
    isModerator && (case_.status === 'pending' || case_.status === 'under_review');
  const canDecideAppeal = isModerator && case_.status === 'appealed';

  return (
    <div className={`moderation-case-card ${isOverdue ? 'overdue' : ''}`}>
      <div className="moderation-case-header">
        <div className="moderation-case-id">
          <span className="moderation-case-number">#{case_.id.slice(0, 8)}</span>
          <span className={`moderation-priority priority-${case_.priority}`}>
            {getPriorityLabel(case_.priority)}
          </span>
        </div>
        <div className="moderation-case-status">
          <span className={`moderation-status-badge ${case_.status}`}>
            {getStatusLabel(case_.status)}
          </span>
          <span className={`moderation-case-age ${isOverdue ? 'overdue' : ''}`}>
            {t('moderation.card.ageAgo', { age: formatAge(case_.age_hours) })}
          </span>
        </div>
      </div>

      <div className="moderation-case-content">
        <div className="moderation-content-info">
          <span className="moderation-content-type">{getContentTypeLabel(case_.content_type)}</span>
          {case_.content_preview ? (
            <p className="moderation-content-preview">{case_.content_preview}</p>
          ) : (
            <p className="moderation-content-preview unavailable">No content preview available</p>
          )}
        </div>

        <div className="moderation-violation-info">
          <span className="moderation-violation-type">
            {getViolationTypeLabel(case_.violation_type)}
          </span>
          {case_.report_reason && <p className="moderation-report-reason">{case_.report_reason}</p>}
        </div>
      </div>

      <div className="moderation-case-details">
        <div className="moderation-owner-info">
          <h5>{t('moderation.card.contentOwner')}</h5>
          <span className="moderation-owner-name">{case_.content_owner.name}</span>
        </div>

        {case_.assigned_to_name && (
          <div className="moderation-assigned-info">
            <h5>{t('moderation.card.assignedTo')}</h5>
            <span className="moderation-assignee">{case_.assigned_to_name}</span>
          </div>
        )}
      </div>

      {/* Decision Info */}
      {case_.decision && (
        <div className="moderation-decision-info">
          <h5>{t('moderation.card.decision')}</h5>
          <span className={`moderation-decision ${case_.decision}`}>
            {getActionLabel(case_.decision)}
          </span>
          {case_.decision_rationale && (
            <p className="moderation-rationale">{case_.decision_rationale}</p>
          )}
        </div>
      )}

      {/* Appeal Info */}
      {case_.appeal_filed && (
        <div className="moderation-appeal-info">
          <h5>{t('moderation.card.appealFiled')}</h5>
          {case_.appeal_reason && <p className="moderation-appeal-reason">{case_.appeal_reason}</p>}
          {case_.status === 'appeal_approved' && (
            <span className="moderation-appeal-outcome approved">
              {t('moderation.card.appealUpheld')}
            </span>
          )}
          {case_.status === 'appeal_rejected' && (
            <span className="moderation-appeal-outcome rejected">
              {t('moderation.card.appealRejected')}
            </span>
          )}
        </div>
      )}

      {/* Actions */}
      {showActions && (
        <div className="moderation-case-actions">
          {onViewContent && (
            <button
              type="button"
              className="moderation-action-button secondary"
              onClick={() => onViewContent(case_.id)}
            >
              {t('moderation.card.viewContent')}
            </button>
          )}
          {!case_.assigned_to_name && isModerator && onAssign && (
            <button
              type="button"
              className="moderation-action-button secondary"
              onClick={() => onAssign(case_.id)}
            >
              {t('moderation.card.assignToMe')}
            </button>
          )}
          {canTakeAction && onTakeAction && (
            <button
              type="button"
              className="moderation-action-button primary"
              onClick={() => onTakeAction(case_.id)}
            >
              {t('moderation.card.takeAction')}
            </button>
          )}
          {canDecideAppeal && onDecideAppeal && (
            <button
              type="button"
              className="moderation-action-button primary"
              onClick={() => onDecideAppeal(case_.id)}
            >
              {t('moderation.card.decideAppeal')}
            </button>
          )}
        </div>
      )}

      <div className="moderation-case-footer">
        <span className="moderation-case-created">
          {t('moderation.card.created', { date: formatDate(case_.created_at) })}
        </span>
      </div>
    </div>
  );
};

ModerationCaseCard.displayName = 'ModerationCaseCard';
