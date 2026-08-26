/**
 * Content Moderation Dashboard Page (Epic 67, Story 67.4).
 * Epic 90, Story 90.4: Wire up moderation handlers to API.
 *
 * Dashboard for reviewing and moderating user-generated content.
 */

import {
  type DecideAppealRequest,
  type TakeModerationActionRequest,
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
import { useToast } from '../../../components';
import { DecideAppealDialog } from '../components/DecideAppealDialog';
import type { ModerationCase } from '../components/ModerationCaseCard';
import { ModerationCaseCard } from '../components/ModerationCaseCard';
import type { ModerationQueueStatsData } from '../components/ModerationQueueStats';
import { ModerationQueueStats } from '../components/ModerationQueueStats';
import { TakeModerationActionDialog } from '../components/TakeModerationActionDialog';

type ModerationActionType = TakeModerationActionRequest['action_type'];
type AppealDecision = DecideAppealRequest['decision'];

// When "overdue only" is active we ask the server for the full breached set
// (see `overdue` param). Request up to the backend's max page size so the
// visible list matches the org-wide `overdue_count` badge instead of whatever
// overdue rows happened to fall in the first default page.
const OVERDUE_PAGE_LIMIT = 200;

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
  const { showToast } = useToast();

  // Filters
  const [statusFilter, setStatusFilter] = useState<string>('pending');
  const [contentTypeFilter, setContentTypeFilter] = useState<string>('');
  const [violationTypeFilter, setViolationTypeFilter] = useState<string>('');
  const [priorityFilter, setPriorityFilter] = useState<string>('');
  const [unassignedOnly, setUnassignedOnly] = useState(false);
  // "Show overdue only", driven by the overdue alert. Applied server-side via
  // the list API's `overdue` param so the result reflects the true org-wide
  // breached set, not just the overdue rows in one fetched page.
  const [overdueOnly, setOverdueOnly] = useState(false);

  // Decision dialogs: which case (if any) each dialog is acting on.
  const [takeActionCaseId, setTakeActionCaseId] = useState<string | null>(null);
  const [decideAppealCaseId, setDecideAppealCaseId] = useState<string | null>(null);

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
    overdue: overdueOnly || undefined,
    limit: overdueOnly ? OVERDUE_PAGE_LIMIT : undefined,
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

  const handleTakeAction = useCallback((caseId: string) => {
    setTakeActionCaseId(caseId);
  }, []);

  const submitTakeAction = useCallback(
    (actionType: ModerationActionType, rationale: string, notifyOwner: boolean) => {
      if (!takeActionCaseId) return;
      takeAction.mutate(
        {
          caseId: takeActionCaseId,
          request: {
            action_type: actionType,
            rationale,
            notify_owner: notifyOwner,
          },
        },
        {
          onSuccess: () => {
            setTakeActionCaseId(null);
            showToast({
              type: 'success',
              title: t('moderation.dialogs.takeAction.successTitle'),
              message: t('moderation.dialogs.takeAction.successMessage'),
            });
          },
          onError: (err) => {
            console.error('Failed to take action:', err);
            showToast({
              type: 'error',
              title: t('moderation.dialogs.takeAction.errorTitle'),
              message: t('moderation.dialogs.takeAction.errorMessage'),
            });
          },
        }
      );
    },
    [takeActionCaseId, takeAction, showToast, t]
  );

  const handleViewContent = useCallback((caseId: string) => {
    // TODO(Phase-2): Use React Router's useNavigate for SPA navigation
    // Phase 1: Full page reload for simplicity
    window.location.href = `/compliance/moderation/cases/${caseId}`;
  }, []);

  const handleDecideAppeal = useCallback((caseId: string) => {
    setDecideAppealCaseId(caseId);
  }, []);

  const submitDecideAppeal = useCallback(
    (decision: AppealDecision, rationale: string) => {
      if (!decideAppealCaseId) return;
      decideAppeal.mutate(
        {
          caseId: decideAppealCaseId,
          request: {
            decision,
            rationale,
          },
        },
        {
          onSuccess: () => {
            setDecideAppealCaseId(null);
            showToast({
              type: 'success',
              title: t('moderation.dialogs.decideAppeal.successTitle'),
              message: t('moderation.dialogs.decideAppeal.successMessage'),
            });
          },
          onError: (err) => {
            console.error('Failed to decide appeal:', err);
            showToast({
              type: 'error',
              title: t('moderation.dialogs.decideAppeal.errorTitle'),
              message: t('moderation.dialogs.decideAppeal.errorMessage'),
            });
          },
        }
      );
    },
    [decideAppealCaseId, decideAppeal, showToast, t]
  );

  const handleFilterByPriority = useCallback((priority: number) => {
    setPriorityFilter(priority.toString());
  }, []);

  const handleFilterByViolationType = useCallback((type: string) => {
    setViolationTypeFilter(type);
  }, []);

  const handleShowOverdue = useCallback(() => {
    // The `overdue` list param already restricts to open (pending/under_review)
    // cases past the SLA, so clear the status filter to avoid narrowing it
    // further, then activate the server-side overdue query via `overdueOnly`.
    setStatusFilter('');
    setOverdueOnly(true);
  }, []);

  const handleClearOverdue = useCallback(() => {
    setOverdueOnly(false);
    setStatusFilter('pending');
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
        {overdueOnly && (
          <div className="moderation-active-filter" role="status">
            <span>{t('moderation.queue.overdueActive')}</span>
            <button type="button" onClick={handleClearOverdue}>
              {t('moderation.queue.clearOverdue')}
            </button>
          </div>
        )}
        <div className="moderation-filters">
          <div className="moderation-filter">
            <label htmlFor="statusFilter">{t('moderation.filters.status')}</label>
            <select
              id="statusFilter"
              value={statusFilter}
              onChange={(e) => {
                // A manual status change supersedes the overdue narrowing.
                setOverdueOnly(false);
                setStatusFilter(e.target.value);
              }}
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

      {/* Overdue truncation notice: the overdue view fetches a single capped
          page (OVERDUE_PAGE_LIMIT). If the org has more overdue cases than the
          cap, the list silently truncates below the (unbounded) `overdue_count`
          badge, re-introducing the badge-vs-list mismatch #2853 fixed. Make the
          truncation explicit instead of hiding it (#2859). */}
      {overdueOnly && cases.length === OVERDUE_PAGE_LIMIT && (
        <div className="moderation-truncation-notice" role="status">
          {t('moderation.queue.overdueTruncated', {
            limit: OVERDUE_PAGE_LIMIT,
            count: stats?.overdue_count ?? cases.length,
          })}
        </div>
      )}

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

      {/* Moderation decision dialogs (replace the Phase-1 window.prompt/alert flow).
       *
       * The `key` binds each dialog's identity to the case it acts on so React
       * remounts a fresh instance per case (and on close → reopen). Without it the
       * dialogs stay mounted — they only render `null` while closed — and their
       * internal action/decision/rationale state would leak from one case into the
       * next one opened. The `-closed` sentinels force a remount across the closed
       * state so a same-case reopen also starts blank. */}
      <TakeModerationActionDialog
        key={takeActionCaseId ?? 'take-action-closed'}
        isOpen={takeActionCaseId !== null}
        isSubmitting={takeAction.isPending}
        onSubmit={submitTakeAction}
        onClose={() => setTakeActionCaseId(null)}
      />
      <DecideAppealDialog
        key={decideAppealCaseId ?? 'decide-appeal-closed'}
        isOpen={decideAppealCaseId !== null}
        isSubmitting={decideAppeal.isPending}
        onSubmit={submitDecideAppeal}
        onClose={() => setDecideAppealCaseId(null)}
      />
    </div>
  );
};

ContentModerationPage.displayName = 'ContentModerationPage';
