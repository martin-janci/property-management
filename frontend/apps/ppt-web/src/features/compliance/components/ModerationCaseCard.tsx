/**
 * Content Moderation Case Card Component (Epic 67, Story 67.4).
 *
 * Displays a content moderation case with status, actions, and appeal info.
 */

import type React from 'react';

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

const getStatusLabel = (status: ModerationStatus): string => {
  switch (status) {
    case 'pending':
      return 'Pending';
    case 'under_review':
      return 'Under Review';
    case 'approved':
      return 'Approved';
    case 'removed':
      return 'Removed';
    case 'restricted':
      return 'Restricted';
    case 'warned':
      return 'Warned';
    case 'appealed':
      return 'Appealed';
    case 'appeal_approved':
      return 'Appeal Approved';
    case 'appeal_rejected':
      return 'Appeal Rejected';
    default:
      return status;
  }
};

const getContentTypeLabel = (type: ModeratedContentType): string => {
  return type
    .split('_')
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(' ');
};

const getViolationTypeLabel = (type?: ViolationType): string => {
  if (!type) return 'Not specified';
  return type
    .split('_')
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(' ');
};

const getActionLabel = (action?: ModerationActionType): string => {
  if (!action) return '';
  return action.charAt(0).toUpperCase() + action.slice(1);
};

const getPriorityLabel = (priority: number): string => {
  switch (priority) {
    case 1:
      return 'Critical';
    case 2:
      return 'High';
    case 3:
      return 'Medium';
    case 4:
      return 'Low';
    case 5:
      return 'Lowest';
    default:
      return `P${priority}`;
  }
};

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
            {formatAge(case_.age_hours)} ago
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
          <h5>Content Owner</h5>
          <span className="moderation-owner-name">{case_.content_owner.name}</span>
        </div>

        {case_.assigned_to_name && (
          <div className="moderation-assigned-info">
            <h5>Assigned To</h5>
            <span className="moderation-assignee">{case_.assigned_to_name}</span>
          </div>
        )}
      </div>

      {/* Decision Info */}
      {case_.decision && (
        <div className="moderation-decision-info">
          <h5>Decision</h5>
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
          <h5>Appeal Filed</h5>
          {case_.appeal_reason && <p className="moderation-appeal-reason">{case_.appeal_reason}</p>}
          {case_.status === 'appeal_approved' && (
            <span className="moderation-appeal-outcome approved">Appeal Upheld</span>
          )}
          {case_.status === 'appeal_rejected' && (
            <span className="moderation-appeal-outcome rejected">Appeal Rejected</span>
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
              View Content
            </button>
          )}
          {!case_.assigned_to_name && isModerator && onAssign && (
            <button
              type="button"
              className="moderation-action-button secondary"
              onClick={() => onAssign(case_.id)}
            >
              Assign to Me
            </button>
          )}
          {canTakeAction && onTakeAction && (
            <button
              type="button"
              className="moderation-action-button primary"
              onClick={() => onTakeAction(case_.id)}
            >
              Take Action
            </button>
          )}
          {canDecideAppeal && onDecideAppeal && (
            <button
              type="button"
              className="moderation-action-button primary"
              onClick={() => onDecideAppeal(case_.id)}
            >
              Decide Appeal
            </button>
          )}
        </div>
      )}

      <div className="moderation-case-footer">
        <span className="moderation-case-created">Created: {formatDate(case_.created_at)}</span>
      </div>
    </div>
  );
};

ModerationCaseCard.displayName = 'ModerationCaseCard';
