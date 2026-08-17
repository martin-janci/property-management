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
import type { ModerationCase } from '../components/ModerationCaseCard';
import { ModerationCaseCard } from '../components/ModerationCaseCard';
import type { ModerationQueueStatsData } from '../components/ModerationQueueStats';
import { ModerationQueueStats } from '../components/ModerationQueueStats';

interface ActionTemplate {
  id: string;
  name: string;
  violation_type: string;
  action_type: string;
  rationale_template: string;
  notify_owner: boolean;
}

export const ContentModerationPage: React.FC = () => {
  // Filters
  const [statusFilter, setStatusFilter] = useState<string>('pending');
  const [contentTypeFilter, setContentTypeFilter] = useState<string>('');
  const [violationTypeFilter, setViolationTypeFilter] = useState<string>('');
  const [priorityFilter, setPriorityFilter] = useState<string>('');
  const [unassignedOnly, setUnassignedOnly] = useState(false);

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
    },
    violation_type: c.violation_type as ModerationCase['violation_type'],
    status: c.status as ModerationCase['status'],
    priority: c.priority,
    assigned_to_name: c.assigned_to_name,
    appeal_filed: c.is_appeal ?? false,
    created_at: c.reported_at,
    // Derived from the real `reported_at` timestamp the API returns — not fabricated.
    age_hours: Math.max(
      0,
      (new Date().getTime() - new Date(c.reported_at).getTime()) / (1000 * 60 * 60)
    ),
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

  const handleTakeAction = useCallback(
    (caseId: string) => {
      // TODO(Phase-2): Replace window.prompt with modal form with action templates integration
      // Phase 1: Basic prompts for action type and rationale
      const actionType = window.prompt(
        'Enter action type (remove, restrict, warn, approve):',
        'approve'
      );
      if (!actionType) return;

      const rationale = window.prompt('Enter rationale for this action:');
      if (!rationale) return;

      takeAction.mutate(
        {
          caseId,
          request: {
            action_type: actionType as 'remove' | 'restrict' | 'warn' | 'approve',
            rationale,
            notify_owner: true,
          },
        },
        {
          onError: (err) => {
            console.error('Failed to take action:', err);
            alert('Failed to take action. Please try again.');
          },
        }
      );
    },
    [takeAction]
  );

  const handleViewContent = useCallback((caseId: string) => {
    // TODO(Phase-2): Use React Router's useNavigate for SPA navigation
    // Phase 1: Full page reload for simplicity
    window.location.href = `/compliance/moderation/cases/${caseId}`;
  }, []);

  const handleDecideAppeal = useCallback(
    (caseId: string) => {
      // TODO(Phase-2): Replace window.prompt with modal form with validation
      // Phase 1: Basic prompts for decision and rationale
      const decision = window.prompt('Enter decision (uphold, overturn):', 'uphold');
      if (!decision) return;

      const rationale = window.prompt('Enter rationale for this decision:');
      if (!rationale) return;

      decideAppeal.mutate(
        {
          caseId,
          request: {
            decision: decision as 'uphold' | 'overturn',
            rationale,
          },
        },
        {
          onError: (err) => {
            console.error('Failed to decide appeal:', err);
            alert('Failed to decide appeal. Please try again.');
          },
        }
      );
    },
    [decideAppeal]
  );

  const handleFilterByPriority = useCallback((priority: number) => {
    setPriorityFilter(priority.toString());
  }, []);

  const handleFilterByViolationType = useCallback((type: string) => {
    setViolationTypeFilter(type);
  }, []);

  const handleShowOverdue = useCallback(() => {
    // Filter to show overdue cases
    setStatusFilter('pending');
    // TODO: Add overdue filter parameter when API supports it
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
    </div>
  );
};

ContentModerationPage.displayName = 'ContentModerationPage';
