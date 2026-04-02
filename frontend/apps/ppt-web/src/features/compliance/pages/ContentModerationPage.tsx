/**
 * Content Moderation Dashboard Page (Epic 67, Story 67.4).
 * Epic 90, Story 90.4: Wire up moderation handlers to API.
 *
 * Dashboard for reviewing and moderating user-generated content.
 */

import {
  useAssignModerationCase,
  useDecideModerationAppeal,
  useModerationCases,
  useModerationStats,
  useModerationTemplates,
  useTakeModerationAction,
} from '@ppt/api-client';
import type React from 'react';
import { useCallback, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useToast } from '../../../components';
import { ModerationCaseCard } from '../components/ModerationCaseCard';
import type { ModerationCase } from '../components/ModerationCaseCard';
import { ModerationQueueStats } from '../components/ModerationQueueStats';
import type { ModerationQueueStatsData } from '../components/ModerationQueueStats';

interface ActionTemplate {
  id: string;
  name: string;
  violation_type: string;
  action_type: string;
  rationale_template: string;
  notify_owner: boolean;
}

export const ContentModerationPage: React.FC = () => {
  const navigate = useNavigate();
  const { showToast } = useToast();

  // Filters
  const [statusFilter, setStatusFilter] = useState<string>('pending');
  const [contentTypeFilter, setContentTypeFilter] = useState<string>('');
  const [violationTypeFilter, setViolationTypeFilter] = useState<string>('');
  const [priorityFilter, setPriorityFilter] = useState<string>('');
  const [unassignedOnly, setUnassignedOnly] = useState(false);

  // Modal state for Take Action form
  const [actionModal, setActionModal] = useState<{
    caseId: string;
    actionType: 'remove' | 'restrict' | 'warn' | 'approve';
    rationale: string;
  } | null>(null);

  // Modal state for Appeal Decision form
  const [appealModal, setAppealModal] = useState<{
    caseId: string;
    decision: 'uphold' | 'overturn';
    rationale: string;
  } | null>(null);

  // Fetch moderation cases from API
  const {
    data: casesData,
    isLoading: casesLoading,
    error: casesError,
  } = useModerationCases({
    status: statusFilter || undefined,
    content_type: contentTypeFilter || undefined,
    violation_type: violationTypeFilter || undefined,
    priority: priorityFilter ? Number.parseInt(priorityFilter, 10) : undefined,
    unassigned_only: unassignedOnly || undefined,
  });

  // Fetch stats from API
  const { data: statsData } = useModerationStats();

  // Fetch templates from API
  const { data: templatesData } = useModerationTemplates();

  // Mutations
  const assignCase = useAssignModerationCase();
  const takeAction = useTakeModerationAction();
  const decideAppeal = useDecideModerationAppeal();

  // Transform API data to component types
  const cases: ModerationCase[] = (casesData?.cases ?? []).map((c) => ({
    id: c.id,
    content_type: c.content_type as ModerationCase['content_type'],
    content_id: c.content_id,
    content_preview: c.content_preview,
    content_owner: {
      user_id: c.owner_id,
      name: c.owner_name,
      previous_violations: 0, // API field not yet available
    },
    report_source: 'user', // API field not yet available
    violation_type: c.violation_type as ModerationCase['violation_type'],
    status: c.status as ModerationCase['status'],
    priority: c.priority,
    assigned_to_name: c.assigned_to_name,
    appeal_filed: c.is_appeal ?? false,
    created_at: c.reported_at,
    age_hours: c.sla_deadline
      ? Math.max(0, (new Date().getTime() - new Date(c.reported_at).getTime()) / (1000 * 60 * 60))
      : 0,
  }));

  const stats: ModerationQueueStatsData | null = statsData?.stats
    ? {
        pending_count: statsData.stats.pending_count,
        under_review_count: statsData.stats.under_review_count,
        by_priority: statsData.stats.by_priority,
        by_violation_type: statsData.stats.by_violation_type.map((v) => ({
          violation_type: v.type,
          count: v.count,
        })),
        avg_resolution_time_hours: statsData.stats.avg_resolution_time_hours,
        overdue_count: statsData.stats.overdue_count,
      }
    : null;

  const templates: ActionTemplate[] = (templatesData?.templates ?? []).map((t) => ({
    id: t.id,
    name: t.name,
    violation_type: t.violation_type,
    action_type: t.action_type,
    rationale_template: t.rationale_template,
    notify_owner: t.notify_owner,
  }));

  const handleAssign = useCallback(
    (caseId: string) => {
      assignCase.mutate(caseId, {
        onError: (err) => {
          console.error('Failed to assign case:', err);
          alert('Failed to assign case. Please try again.');
        },
      });
    },
    [assignCase]
  );

  const handleTakeAction = useCallback((caseId: string) => {
    setActionModal({ caseId, actionType: 'approve', rationale: '' });
  }, []);

  const submitAction = useCallback(() => {
    if (!actionModal || !actionModal.rationale.trim()) return;
    takeAction.mutate(
      {
        caseId: actionModal.caseId,
        request: {
          action_type: actionModal.actionType,
          rationale: actionModal.rationale,
          notify_owner: true,
        },
      },
      {
        onSuccess: () => {
          showToast({ type: 'success', title: 'Action taken successfully' });
          setActionModal(null);
        },
        onError: (err) => {
          showToast({ type: 'error', title: 'Failed to take action', message: err.message });
        },
      }
    );
  }, [actionModal, takeAction, showToast]);

  const handleViewContent = useCallback(
    (caseId: string) => {
      navigate(`/compliance/moderation/cases/${caseId}`);
    },
    [navigate]
  );

  const handleDecideAppeal = useCallback((caseId: string) => {
    setAppealModal({ caseId, decision: 'uphold', rationale: '' });
  }, []);

  const submitAppealDecision = useCallback(() => {
    if (!appealModal || !appealModal.rationale.trim()) return;
    decideAppeal.mutate(
      {
        caseId: appealModal.caseId,
        request: {
          decision: appealModal.decision,
          rationale: appealModal.rationale,
        },
      },
      {
        onSuccess: () => {
          showToast({ type: 'success', title: 'Appeal decision submitted' });
          setAppealModal(null);
        },
        onError: (err) => {
          showToast({ type: 'error', title: 'Failed to decide appeal', message: err.message });
        },
      }
    );
  }, [appealModal, decideAppeal, showToast]);

  const handleFilterByPriority = useCallback((priority: number) => {
    setPriorityFilter(priority.toString());
  }, []);

  const handleFilterByViolationType = useCallback((type: string) => {
    setViolationTypeFilter(type);
  }, []);

  const handleShowOverdue = useCallback(() => {
    // Shows pending cases; overdue-specific filter will be added when API supports it
    setStatusFilter('pending');
  }, []);

  // Loading state
  if (casesLoading) {
    return (
      <div className="content-moderation-page">
        <div className="moderation-page-header">
          <h1>Content Moderation Dashboard</h1>
          <p>Review and moderate user-generated content for DSA compliance.</p>
        </div>
        <div className="moderation-loading">Loading moderation queue...</div>
      </div>
    );
  }

  // Error state
  if (casesError) {
    return (
      <div className="content-moderation-page">
        <div className="moderation-page-header">
          <h1>Content Moderation Dashboard</h1>
          <p>Review and moderate user-generated content for DSA compliance.</p>
        </div>
        <div className="moderation-page-error" role="alert">
          Failed to load moderation cases: {casesError.message}
          <button
            type="button"
            onClick={() => window.location.reload()}
            className="moderation-retry-button"
          >
            Retry
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="content-moderation-page">
      <div className="moderation-page-header">
        <h1>Content Moderation Dashboard</h1>
        <p>Review and moderate user-generated content for DSA compliance.</p>
      </div>

      {/* Queue Statistics */}
      {stats && (
        <ModerationQueueStats
          stats={stats}
          onFilterByPriority={handleFilterByPriority}
          onFilterByViolationType={handleFilterByViolationType}
          onShowOverdue={handleShowOverdue}
        />
      )}

      {/* Filters */}
      <div className="moderation-filters-section">
        <h2>Moderation Queue</h2>
        <div className="moderation-filters">
          <div className="moderation-filter">
            <label htmlFor="statusFilter">Status</label>
            <select
              id="statusFilter"
              value={statusFilter}
              onChange={(e) => setStatusFilter(e.target.value)}
            >
              <option value="">All Statuses</option>
              <option value="pending">Pending</option>
              <option value="under_review">Under Review</option>
              <option value="appealed">Appealed</option>
              <option value="removed">Removed</option>
              <option value="restricted">Restricted</option>
              <option value="warned">Warned</option>
              <option value="approved">Approved</option>
            </select>
          </div>
          <div className="moderation-filter">
            <label htmlFor="contentTypeFilter">Content Type</label>
            <select
              id="contentTypeFilter"
              value={contentTypeFilter}
              onChange={(e) => setContentTypeFilter(e.target.value)}
            >
              <option value="">All Types</option>
              <option value="listing">Listing</option>
              <option value="listing_photo">Listing Photo</option>
              <option value="review">Review</option>
              <option value="comment">Comment</option>
              <option value="community_post">Community Post</option>
              <option value="message">Message</option>
            </select>
          </div>
          <div className="moderation-filter">
            <label htmlFor="violationTypeFilter">Violation Type</label>
            <select
              id="violationTypeFilter"
              value={violationTypeFilter}
              onChange={(e) => setViolationTypeFilter(e.target.value)}
            >
              <option value="">All Violations</option>
              <option value="spam">Spam</option>
              <option value="harassment">Harassment</option>
              <option value="hate_speech">Hate Speech</option>
              <option value="violence">Violence</option>
              <option value="illegal_content">Illegal Content</option>
              <option value="misinformation">Misinformation</option>
              <option value="fraud">Fraud</option>
              <option value="privacy">Privacy</option>
              <option value="inappropriate_content">Inappropriate Content</option>
            </select>
          </div>
          <div className="moderation-filter">
            <label htmlFor="priorityFilter">Priority</label>
            <select
              id="priorityFilter"
              value={priorityFilter}
              onChange={(e) => setPriorityFilter(e.target.value)}
            >
              <option value="">All Priorities</option>
              <option value="1">Critical (P1)</option>
              <option value="2">High (P2)</option>
              <option value="3">Medium (P3)</option>
              <option value="4">Low (P4)</option>
              <option value="5">Lowest (P5)</option>
            </select>
          </div>
          <div className="moderation-filter checkbox">
            <label htmlFor="unassignedOnly">
              <input
                type="checkbox"
                id="unassignedOnly"
                checked={unassignedOnly}
                onChange={(e) => setUnassignedOnly(e.target.checked)}
              />
              Unassigned Only
            </label>
          </div>
        </div>
      </div>

      {/* Cases List */}
      {cases.length > 0 ? (
        <div className="moderation-cases-list">
          {cases.map((case_) => (
            <ModerationCaseCard
              key={case_.id}
              case_={case_}
              onAssign={handleAssign}
              onTakeAction={handleTakeAction}
              onViewContent={handleViewContent}
              onDecideAppeal={handleDecideAppeal}
              showActions={true}
              isModerator={true}
            />
          ))}
        </div>
      ) : (
        <div className="moderation-empty-state">
          <p>No moderation cases found matching the criteria.</p>
          <p>The moderation queue is currently empty.</p>
        </div>
      )}

      {/* Action Templates */}
      <div className="moderation-templates-section">
        <h2>Action Templates</h2>
        <div className="moderation-templates-list">
          {templates.map((template) => (
            <div key={template.id} className="moderation-template-card">
              <h4>{template.name}</h4>
              <p className="template-violation">{template.violation_type}</p>
              <p className="template-action">{template.action_type}</p>
              <p className="template-rationale">{template.rationale_template}</p>
            </div>
          ))}
        </div>
      </div>

      {/* Take Action Modal */}
      {actionModal && (
        <div className="moderation-modal-overlay" role="dialog" aria-label="Take moderation action">
          <div className="moderation-modal">
            <h3>Take Moderation Action</h3>
            <div className="moderation-modal-field">
              <label htmlFor="actionType">Action Type</label>
              <select
                id="actionType"
                value={actionModal.actionType}
                onChange={(e) =>
                  setActionModal({
                    ...actionModal,
                    actionType: e.target.value as 'remove' | 'restrict' | 'warn' | 'approve',
                  })
                }
              >
                <option value="approve">Approve</option>
                <option value="warn">Warn</option>
                <option value="restrict">Restrict</option>
                <option value="remove">Remove</option>
              </select>
            </div>
            <div className="moderation-modal-field">
              <label htmlFor="actionRationale">Rationale</label>
              <textarea
                id="actionRationale"
                value={actionModal.rationale}
                onChange={(e) => setActionModal({ ...actionModal, rationale: e.target.value })}
                placeholder="Enter rationale for this action..."
                rows={3}
                required
              />
            </div>
            <div className="moderation-modal-actions">
              <button type="button" onClick={() => setActionModal(null)}>
                Cancel
              </button>
              <button
                type="button"
                onClick={submitAction}
                disabled={!actionModal.rationale.trim() || takeAction.isPending}
              >
                {takeAction.isPending ? 'Submitting...' : 'Submit Action'}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Appeal Decision Modal */}
      {appealModal && (
        <div className="moderation-modal-overlay" role="dialog" aria-label="Decide appeal">
          <div className="moderation-modal">
            <h3>Decide Appeal</h3>
            <div className="moderation-modal-field">
              <label htmlFor="appealDecision">Decision</label>
              <select
                id="appealDecision"
                value={appealModal.decision}
                onChange={(e) =>
                  setAppealModal({
                    ...appealModal,
                    decision: e.target.value as 'uphold' | 'overturn',
                  })
                }
              >
                <option value="uphold">Uphold Original Decision</option>
                <option value="overturn">Overturn Decision</option>
              </select>
            </div>
            <div className="moderation-modal-field">
              <label htmlFor="appealRationale">Rationale</label>
              <textarea
                id="appealRationale"
                value={appealModal.rationale}
                onChange={(e) => setAppealModal({ ...appealModal, rationale: e.target.value })}
                placeholder="Enter rationale for this decision..."
                rows={3}
                required
              />
            </div>
            <div className="moderation-modal-actions">
              <button type="button" onClick={() => setAppealModal(null)}>
                Cancel
              </button>
              <button
                type="button"
                onClick={submitAppealDecision}
                disabled={!appealModal.rationale.trim() || decideAppeal.isPending}
              >
                {decideAppeal.isPending ? 'Submitting...' : 'Submit Decision'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

ContentModerationPage.displayName = 'ContentModerationPage';
