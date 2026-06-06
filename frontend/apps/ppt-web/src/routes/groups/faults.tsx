/**
 * Faults route group (UC-03).
 *
 * Owns the fault API↔UI mappers and the route-wrapper components, plus the
 * `<Route>` table fragment. Extracted from App.tsx to isolate fault work.
 */
import type {
  FaultSummary as ApiFaultSummary,
  FaultAttachment,
  FaultListQuery,
  FaultTimelineEntry,
  FaultWithDetails,
} from '@ppt/api-client';
import {
  useAddAttachment,
  useAddComment,
  useBuildings,
  useConfirmFault,
  useCreateFault,
  useDeleteAttachment,
  useFault,
  useFaults,
  useReopenFault,
  useResolveFault,
  useTriageFault,
  useUpdateFault,
} from '@ppt/api-client';
import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Route, useNavigate, useParams } from 'react-router-dom';
import { useToast } from '../../components';
import { useAuth } from '../../contexts';
import type { FaultDetail, FaultAttachment as UiFaultAttachment } from '../../features/faults';
import type {
  FaultStatus,
  FaultSummary as UiFaultSummary,
} from '../../features/faults/components/FaultCard';
import type { TimelineAction, TimelineEntry } from '../../features/faults/components/FaultTimeline';
import { CreateFaultPage, EditFaultPage, FaultDetailPage, FaultsPage } from '../lazyRoutes';
import { isManagerRole } from '../shared';

/**
 * Map API FaultStatus (snake_case values) → UI FaultStatus.
 *
 * API: 'reported' | 'triaged' | 'in_progress' | 'scheduled' | 'on_hold' | 'resolved' | 'closed' | 'reopened'
 * UI:  'new'      | 'triaged' | 'in_progress' | 'scheduled' | 'waiting_parts' | 'resolved' | 'closed' | 'reopened'
 */
function mapApiFaultStatusToUi(status: ApiFaultSummary['status']): FaultStatus {
  const mapping: Record<ApiFaultSummary['status'], FaultStatus> = {
    new: 'new',
    triaged: 'triaged',
    in_progress: 'in_progress',
    scheduled: 'scheduled',
    waiting_parts: 'waiting_parts',
    resolved: 'resolved',
    closed: 'closed',
    reopened: 'reopened',
  };
  return mapping[status] ?? 'new';
}

/**
 * Map UI fault status (used by FaultCard) to the API status enum.
 * Module-level (#486) so the route wrapper doesn't recreate it per call.
 */
const UI_FAULT_STATUS_TO_API: Record<FaultStatus, ApiFaultSummary['status'] | undefined> = {
  new: 'new',
  triaged: 'triaged',
  in_progress: 'in_progress',
  waiting_parts: 'waiting_parts',
  scheduled: 'scheduled',
  resolved: 'resolved',
  closed: 'closed',
  reopened: 'reopened',
};

/** Transform API FaultSummary (snake_case) → UI FaultSummary (camelCase) */
function transformApiFaultToUi(fault: ApiFaultSummary): UiFaultSummary {
  return {
    id: fault.id,
    buildingId: fault.building_id,
    unitId: fault.unit_id,
    title: fault.title,
    category: fault.category,
    priority: fault.priority ?? 'medium',
    status: mapApiFaultStatusToUi(fault.status),
    createdAt: fault.created_at,
  };
}

/**
 * Map API FaultWithDetails (snake_case) → page-level FaultDetail (camelCase).
 *
 * The api-client `FaultWithDetails` mirrors the backend model: nullable columns
 * are typed `T | null`, while the page `FaultDetail` uses optional (`T | undefined`)
 * fields, so we strip `null` → `undefined` on the nullable fields.
 */
function mapApiFaultDetailToUi(fault: FaultWithDetails): FaultDetail {
  return {
    id: fault.id,
    organizationId: fault.organization_id,
    buildingId: fault.building_id,
    unitId: fault.unit_id ?? undefined,
    reporterId: fault.reporter_id,
    title: fault.title,
    description: fault.description,
    locationDescription: fault.location_description ?? undefined,
    category: fault.category,
    priority: fault.priority ?? 'medium',
    status: mapApiFaultStatusToUi(fault.status),
    aiCategory: fault.ai_category ?? undefined,
    aiPriority: fault.ai_priority ?? undefined,
    aiConfidence: fault.ai_confidence != null ? Number(fault.ai_confidence) : undefined,
    assignedTo: fault.assigned_to ?? undefined,
    assignedAt: fault.assigned_at ?? undefined,
    triagedBy: fault.triaged_by ?? undefined,
    triagedAt: fault.triaged_at ?? undefined,
    resolvedAt: fault.resolved_at ?? undefined,
    resolvedBy: fault.resolved_by ?? undefined,
    resolutionNotes: fault.resolution_notes ?? undefined,
    confirmedAt: fault.confirmed_at ?? undefined,
    confirmedBy: fault.confirmed_by ?? undefined,
    rating: fault.rating ?? undefined,
    feedback: fault.feedback ?? undefined,
    scheduledDate: fault.scheduled_date ?? undefined,
    estimatedCompletion: fault.estimated_completion ?? undefined,
    createdAt: fault.created_at,
    updatedAt: fault.updated_at,
    reporterName: fault.reporter_name,
    reporterEmail: fault.reporter_email,
    buildingName: fault.building_name,
    buildingAddress: fault.building_address,
    unitDesignation: fault.unit_designation ?? undefined,
    assignedToName: fault.assigned_to_name ?? undefined,
    attachmentCount: fault.attachment_count,
    commentCount: fault.comment_count,
  };
}

/**
 * Map an API fault `event_type` string to the page TimelineEntry `action`.
 * Unknown event types fall back to `status_changed`.
 */
function mapFaultEventTypeToAction(eventType: string): TimelineAction {
  const mapping: Record<string, TimelineAction> = {
    created: 'created',
    reported: 'created',
    triaged: 'triaged',
    assigned: 'assigned',
    status_changed: 'status_changed',
    priority_changed: 'priority_changed',
    work_note: 'work_note',
    comment: 'comment',
    comment_added: 'comment',
    attachment_added: 'attachment_added',
    scheduled: 'scheduled',
    resolved: 'resolved',
    confirmed: 'confirmed',
    reopened: 'reopened',
    rated: 'rated',
  };
  return mapping[eventType] ?? 'status_changed';
}

/** Map API FaultTimelineEntry (snake_case) → page TimelineEntry. */
function mapApiFaultTimelineToUi(entry: FaultTimelineEntry): TimelineEntry {
  return {
    id: entry.id,
    faultId: entry.fault_id,
    userId: entry.user_id,
    action: mapFaultEventTypeToAction(entry.action),
    note: entry.note ?? undefined,
    oldValue: entry.old_value ?? undefined,
    newValue: entry.new_value ?? undefined,
    isInternal: entry.is_internal,
    createdAt: entry.created_at,
    userName: entry.user_name ?? '',
    userEmail: entry.user_email ?? '',
  };
}

/** Map API FaultAttachment (snake_case) → page FaultAttachment. */
function mapApiFaultAttachmentToUi(att: FaultAttachment): UiFaultAttachment {
  return {
    id: att.id,
    faultId: att.fault_id,
    filename: att.file_name,
    originalFilename: att.file_name,
    contentType: att.file_type,
    sizeBytes: att.file_size,
    storageUrl: att.file_url,
    description: undefined,
    createdAt: att.created_at,
  };
}

/**
 * Route wrapper for faults list page (UC-03, gap-79-1).
 *
 * Wired to useFaults from @ppt/api-client (TanStack Query).
 * API FaultSummary uses snake_case and a slightly different status enum;
 * transformApiFaultToUi() bridges the gap to the UI FaultCard type.
 */
function FaultsPageRoute() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { showToast } = useToast();
  const [faultQuery, setFaultQuery] = useState<FaultListQuery>({ page: 1, limit: 10 });

  const { data, isLoading, error, refetch } = useFaults(faultQuery);

  useEffect(() => {
    if (error) {
      showToast({
        type: 'error',
        title: t('faults.failedToLoad', { defaultValue: 'Failed to load faults' }),
        message: error instanceof Error ? error.message : t('auth.unexpectedError'),
      });
    }
  }, [error, showToast, t]);

  const faults = (data?.faults ?? []).map(transformApiFaultToUi);
  const total = data?.count ?? 0;

  // useCallback so child FaultsPage doesn't re-render on every parent render
  // (#486). Status mapping uses the module-level UI_FAULT_STATUS_TO_API.
  const handleFaultsFilterChange = useCallback(
    (params: {
      status?: FaultStatus;
      category?: ApiFaultSummary['category'];
      priority?: ApiFaultSummary['priority'];
      search?: string;
      page?: number;
      pageSize?: number;
    }) => {
      setFaultQuery({
        status: params.status ? UI_FAULT_STATUS_TO_API[params.status] : undefined,
        category: params.category,
        priority: params.priority,
        search: params.search,
        page: params.page,
        limit: params.pageSize,
      });
    },
    []
  );

  return (
    <FaultsPage
      faults={faults}
      total={total}
      isLoading={isLoading}
      isError={!!error}
      onRetry={() => {
        void refetch();
      }}
      onNavigateToCreate={() => navigate('/faults/new')}
      onNavigateToView={(id) => navigate(`/faults/${id}`)}
      onNavigateToEdit={(id) => navigate(`/faults/${id}/edit`)}
      onNavigateToTriage={(id) => navigate(`/faults/${id}`)}
      onFilterChange={handleFaultsFilterChange}
    />
  );
}

function CreateFaultPageRoute() {
  const navigate = useNavigate();
  const { showToast } = useToast();
  const { data: buildingsData } = useBuildings();
  const createFault = useCreateFault();

  const buildings = (buildingsData?.items ?? []).map((b) => ({ id: b.id, name: b.name }));

  return (
    <CreateFaultPage
      buildings={buildings}
      units={[]}
      isSubmitting={createFault.isPending}
      onSubmit={async (data) => {
        try {
          // Map FaultFormData (camelCase) -> CreateFaultRequest (snake_case).
          // NOTE: photos (File[]) are not sent here — base64/upload wiring via
          // useAddAttachment is a separate follow-up (see fault-new screen-map).
          const created = await createFault.mutateAsync({
            building_id: data.buildingId,
            unit_id: data.unitId,
            title: data.title,
            description: data.description,
            location_description: data.locationDescription,
            category: data.category,
            priority: data.priority,
          });
          showToast({ type: 'success', title: 'Created', message: 'Fault reported' });
          navigate(created?.id ? `/faults/${created.id}` : '/faults');
        } catch (err) {
          showToast({
            type: 'error',
            title: 'Failed to report fault',
            message: err instanceof Error ? err.message : 'Please try again.',
          });
        }
      }}
      onCancel={() => navigate('/faults')}
    />
  );
}

/**
 * Route wrapper for fault detail page (UC-03, #970.1).
 *
 * Wired to @ppt/api-client fault hooks (TanStack Query). `useFault` returns the
 * `FaultDetailResponse` ({ fault, timeline, attachments }); the snake_case API
 * shapes are mapped to the page's camelCase `FaultDetail` / `TimelineEntry` /
 * `FaultAttachment` via the module-level mappers near `transformApiFaultToUi`.
 */
function FaultDetailPageRoute() {
  const { faultId } = useParams<{ faultId: string }>();
  const navigate = useNavigate();
  const { showToast } = useToast();
  const { t } = useTranslation();
  const { user } = useAuth();

  const { data, isLoading, error } = useFault(faultId ?? '');
  const triage = useTriageFault(faultId ?? '');
  const resolve = useResolveFault(faultId ?? '');
  const confirm = useConfirmFault(faultId ?? '');
  const reopen = useReopenFault(faultId ?? '');
  const addComment = useAddComment(faultId ?? '');
  const addAttachment = useAddAttachment(faultId ?? '');
  const deleteAttachment = useDeleteAttachment(faultId ?? '');

  useEffect(() => {
    if (error) {
      showToast({
        type: 'error',
        title: t('faults.failedToLoad', { defaultValue: 'Failed to load fault' }),
        message: error instanceof Error ? error.message : t('auth.unexpectedError'),
      });
    }
  }, [error, showToast, t]);

  if (!faultId) {
    return <div>Fault not found</div>;
  }

  const isManager = isManagerRole(user?.role);

  const fault = data?.fault ? mapApiFaultDetailToUi(data.fault) : undefined;
  const timeline = (data?.timeline ?? []).map(mapApiFaultTimelineToUi);
  const attachments = (data?.attachments ?? []).map(mapApiFaultAttachmentToUi);

  if (isLoading || !fault) {
    return (
      <div className="flex items-center justify-center py-12">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
      </div>
    );
  }

  return (
    <FaultDetailPage
      fault={fault}
      timeline={timeline}
      attachments={attachments}
      isManager={isManager}
      isLoading={isLoading}
      onBack={() => navigate('/faults')}
      onEdit={() => navigate(`/faults/${faultId}/edit`)}
      onTriage={(triageData) =>
        triage.mutate({
          priority: triageData.priority,
          category: triageData.category,
          assigned_to: triageData.assignedTo,
        })
      }
      onResolve={(notes) => resolve.mutate({ resolution_notes: notes })}
      onConfirm={(rating) => confirm.mutate(rating)}
      onReopen={(reason) => reopen.mutate(reason)}
      onAddComment={(content, isInternal) =>
        addComment.mutate({ content, is_internal: isInternal })
      }
      onAddAttachment={(file) => addAttachment.mutate(file)}
      onDeleteAttachment={(id) => deleteAttachment.mutate(id)}
    />
  );
}

function EditFaultPageRoute() {
  const { faultId } = useParams<{ faultId: string }>();
  const navigate = useNavigate();
  const { showToast } = useToast();
  const { data, isLoading } = useFault(faultId ?? '');
  const { data: buildingsData } = useBuildings();
  const updateFault = useUpdateFault(faultId ?? '');

  if (!faultId) {
    return <div>Fault not found</div>;
  }
  if (isLoading || !data) {
    return <div className="p-6">Loading…</div>;
  }

  const fault = data.fault;
  const buildings = (buildingsData?.items ?? []).map((b) => ({ id: b.id, name: b.name }));

  return (
    <EditFaultPage
      faultId={faultId}
      initialData={{
        buildingId: fault.building_id,
        unitId: fault.unit_id ?? undefined,
        title: fault.title,
        description: fault.description,
        locationDescription: fault.location_description ?? undefined,
        category: fault.category,
        priority: fault.priority,
      }}
      buildings={buildings}
      units={[]}
      isSubmitting={updateFault.isPending}
      onSubmit={async (formData) => {
        try {
          await updateFault.mutateAsync({
            title: formData.title,
            description: formData.description,
            location_description: formData.locationDescription,
            category: formData.category,
          });
          showToast({ type: 'success', title: 'Updated', message: 'Fault updated' });
          navigate(`/faults/${faultId}`);
        } catch (err) {
          showToast({
            type: 'error',
            title: 'Failed to update fault',
            message: err instanceof Error ? err.message : 'Please try again.',
          });
        }
      }}
      onCancel={() => navigate(`/faults/${faultId}`)}
    />
  );
}

/** Faults routes (UC-03). */
export function faultRoutes() {
  return (
    <>
      <Route path="/faults" element={<FaultsPageRoute />} />
      <Route path="/faults/new" element={<CreateFaultPageRoute />} />
      <Route path="/faults/:faultId" element={<FaultDetailPageRoute />} />
      <Route path="/faults/:faultId/edit" element={<EditFaultPageRoute />} />
    </>
  );
}
