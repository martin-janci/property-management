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
import { useTranslation } from 'react-i18next';
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
  const { t } = useTranslation();

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
          alert(t('moderation.prompts.assignError'));
        },
      });
    },
    [assignCase, t]
  );

  const handleTakeAction = useCallback(
    (caseId: string) => {
      // TODO(Phase-2): Replace window.prompt with modal form with action templates integration
      // Phase 1: Basic prompts for action type and rationale
      const actionType = window.prompt(t('moderation.prompts.actionType'), 'approve');
      if (!actionType) return;

      const rationale = window.prompt(t('moderation.prompts.actionRationale'));
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
            alert(t('moderation.prompts.actionError'));
          },
        }
      );
    },
    [takeAction, t]
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
      const decision = window.prompt(t('moderation.prompts.decision'), 'uphold');
      if (!decision) return;

      const rationale = window.prompt(t('moderation.prompts.decisionRationale'));
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
            alert(t('moderation.prompts.appealError'));
          },
        }
      );
    },
    [decideAppeal, t]
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
          <h1>{t('moderation.dashboard.title')}</h1>
          <p>{t('moderation.dashboard.subtitle')}</p>
        </div>
        <div className="moderation-loading">{t('moderation.dashboard.loading')}</div>
      </div>
    );
  }

  // Error state
  if (casesError) {
    return (
      <div className="content-moderation-page">
        <div className="moderation-page-header">
          <h1>{t('moderation.dashboard.title')}</h1>
          <p>{t('moderation.dashboard.subtitle')}</p>
        </div>
        <div className="moderation-page-error" role="alert">
          {t('moderation.dashboard.loadError', { message: casesError.message })}
          <button
            type="button"
            onClick={() => window.location.reload()}
            className="moderation-retry-button"
          >
            {t('moderation.dashboard.retry')}
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="content-moderation-page">
      <div className="moderation-page-header">
        <h1>{t('moderation.dashboard.title')}</h1>
        <p>{t('moderation.dashboard.subtitle')}</p>
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
        <h2>{t('moderation.queue.heading')}</h2>
        <div className="moderation-filters">
          <div className="moderation-filter">
            <label htmlFor="statusFilter">{t('moderation.filters.status')}</label>
            <select
              id="statusFilter"
              value={statusFilter}
              onChange={(e) => setStatusFilter(e.target.value)}
            >
              <option value="">{t('moderation.filters.allStatuses')}</option>
              <option value="pending">{t('moderation.status.pending')}</option>
              <option value="under_review">{t('moderation.status.under_review')}</option>
              <option value="appealed">{t('moderation.status.appealed')}</option>
              <option value="removed">{t('moderation.status.removed')}</option>
              <option value="restricted">{t('moderation.status.restricted')}</option>
              <option value="warned">{t('moderation.status.warned')}</option>
              <option value="approved">{t('moderation.status.approved')}</option>
            </select>
          </div>
          <div className="moderation-filter">
            <label htmlFor="contentTypeFilter">{t('moderation.filters.contentType')}</label>
            <select
              id="contentTypeFilter"
              value={contentTypeFilter}
              onChange={(e) => setContentTypeFilter(e.target.value)}
            >
              <option value="">{t('moderation.filters.allTypes')}</option>
              <option value="listing">{t('moderation.contentType.listing')}</option>
              <option value="listing_photo">{t('moderation.contentType.listing_photo')}</option>
              <option value="review">{t('moderation.contentType.review')}</option>
              <option value="comment">{t('moderation.contentType.comment')}</option>
              <option value="community_post">{t('moderation.contentType.community_post')}</option>
              <option value="message">{t('moderation.contentType.message')}</option>
            </select>
          </div>
          <div className="moderation-filter">
            <label htmlFor="violationTypeFilter">{t('moderation.filters.violationType')}</label>
            <select
              id="violationTypeFilter"
              value={violationTypeFilter}
              onChange={(e) => setViolationTypeFilter(e.target.value)}
            >
              <option value="">{t('moderation.filters.allViolations')}</option>
              <option value="spam">{t('moderation.violationType.spam')}</option>
              <option value="harassment">{t('moderation.violationType.harassment')}</option>
              <option value="hate_speech">{t('moderation.violationType.hate_speech')}</option>
              <option value="violence">{t('moderation.violationType.violence')}</option>
              <option value="illegal_content">
                {t('moderation.violationType.illegal_content')}
              </option>
              <option value="misinformation">{t('moderation.violationType.misinformation')}</option>
              <option value="fraud">{t('moderation.violationType.fraud')}</option>
              <option value="privacy">{t('moderation.violationType.privacy')}</option>
              <option value="inappropriate_content">
                {t('moderation.violationType.inappropriate_content')}
              </option>
            </select>
          </div>
          <div className="moderation-filter">
            <label htmlFor="priorityFilter">{t('moderation.filters.priority')}</label>
            <select
              id="priorityFilter"
              value={priorityFilter}
              onChange={(e) => setPriorityFilter(e.target.value)}
            >
              <option value="">{t('moderation.filters.allPriorities')}</option>
              <option value="1">{t('moderation.priorityLabel.1')} (P1)</option>
              <option value="2">{t('moderation.priorityLabel.2')} (P2)</option>
              <option value="3">{t('moderation.priorityLabel.3')} (P3)</option>
              <option value="4">{t('moderation.priorityLabel.4')} (P4)</option>
              <option value="5">{t('moderation.priorityLabel.5')} (P5)</option>
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
              {t('moderation.filters.unassignedOnly')}
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
          <p>{t('moderation.queue.emptyTitle')}</p>
          <p>{t('moderation.queue.emptySubtitle')}</p>
        </div>
      )}

      {/* Action Templates */}
      <div className="moderation-templates-section">
        <h2>{t('moderation.templates.heading')}</h2>
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
