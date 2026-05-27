/**
 * SystemAnnouncementsPage — `/platform/announcements`
 *
 * Admin dashboard for managing platform-wide system announcements (Epic 10B,
 * Story 10B.4). Provides CRUD for announcements that target all organisations,
 * with scheduled publish/expire (start_at / end_at) and a live banner preview.
 *
 * Backend endpoints:
 *   GET    /api/v1/platform-admin/announcements           — list all
 *   POST   /api/v1/platform-admin/announcements           — create
 *   PUT    /api/v1/platform-admin/announcements/{id}      — update
 *   DELETE /api/v1/platform-admin/announcements/{id}      — soft-delete
 *
 * Capability required: site_settings_write (write) / site_settings_read (read).
 *
 * Refactored under #521:
 *   - 867 LoC monolith → orchestrator + focused components under
 *     `features/system-announcements/{components,lib}/`
 *   - explicit validator in `lib/schema.ts` replaces native HTML5 hints; the
 *     module is a single seam for a future `react-hook-form`+`zod` migration
 *   - `window.confirm` replaced with the project's `DestructiveConfirmDialog`
 *   - SystemAnnouncementsPage.test.tsx covers list / status / capability gate
 */

import { useCapability } from '@ppt/admin-ui';
import type {
  CreateSystemAnnouncementRequest,
  SystemAnnouncement,
  UpdateSystemAnnouncementRequest,
} from '@ppt/api-client';
import {
  useCreateSystemAnnouncement,
  useDeleteSystemAnnouncement,
  useSystemAnnouncements,
  useUpdateSystemAnnouncement,
} from '@ppt/api-client';
import type { CSSProperties } from 'react';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { DestructiveConfirmDialog } from '../../components/DestructiveConfirmDialog';
import { useToast } from '../../components/Toast';
import { HelpTooltip } from '../help';
import { AnnForm, type AnnFormValues, annToForm } from './components/AnnForm';
import { AnnouncementsTable } from './components/AnnouncementsTable';
import { toIso } from './lib/formatters';

const containerStyle: CSSProperties = {
  padding: 24,
  maxWidth: 1000,
  margin: '0 auto',
};

const cardStyle: CSSProperties = {
  background: 'var(--ppt-bg-surface, #fff)',
  border: '1px solid var(--ppt-border-default, #e5e7eb)',
  borderRadius: 10,
  padding: 24,
};

export default function SystemAnnouncementsPage() {
  const { t } = useTranslation();
  const { showToast } = useToast();

  const canWrite = useCapability('site_settings_write');

  const [includeDeleted, setIncludeDeleted] = useState(false);
  const {
    data: announcements = [],
    isLoading,
    error,
    refetch,
  } = useSystemAnnouncements({ include_deleted: includeDeleted });

  const createMutation = useCreateSystemAnnouncement();
  const updateMutation = useUpdateSystemAnnouncement();
  const deleteMutation = useDeleteSystemAnnouncement();

  const [mode, setMode] = useState<'list' | 'create' | 'edit'>('list');
  const [editTarget, setEditTarget] = useState<SystemAnnouncement | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<SystemAnnouncement | null>(null);

  useEffect(() => {
    if (error) {
      showToast({
        type: 'error',
        title: t('admin.announcements.loadError', 'Failed to load announcements'),
        message: error instanceof Error ? error.message : String(error),
      });
    }
  }, [error, showToast, t]);

  const handleCreate = async (values: AnnFormValues) => {
    const req: CreateSystemAnnouncementRequest = {
      title: values.title,
      message: values.message,
      severity: values.severity,
      start_at: toIso(values.start_at),
      end_at: values.end_at ? toIso(values.end_at) : null,
      is_dismissible: values.is_dismissible,
      requires_acknowledgment: values.requires_acknowledgment,
    };
    try {
      await createMutation.mutateAsync(req);
      showToast({
        type: 'success',
        title: t('admin.announcements.createSuccess', 'Announcement created'),
        message: '',
      });
      setMode('list');
    } catch (err) {
      showToast({
        type: 'error',
        title: t('admin.announcements.createError', 'Failed to create'),
        message: err instanceof Error ? err.message : String(err),
      });
    }
  };

  const handleUpdate = async (values: AnnFormValues) => {
    if (!editTarget) return;
    const req: UpdateSystemAnnouncementRequest = {
      title: values.title,
      message: values.message,
      severity: values.severity,
      start_at: toIso(values.start_at),
      end_at: values.end_at ? toIso(values.end_at) : null,
      is_dismissible: values.is_dismissible,
      requires_acknowledgment: values.requires_acknowledgment,
    };
    try {
      await updateMutation.mutateAsync({ id: editTarget.id, data: req });
      showToast({
        type: 'success',
        title: t('admin.announcements.updateSuccess', 'Announcement updated'),
        message: '',
      });
      setMode('list');
      setEditTarget(null);
    } catch (err) {
      showToast({
        type: 'error',
        title: t('admin.announcements.updateError', 'Failed to update'),
        message: err instanceof Error ? err.message : String(err),
      });
    }
  };

  const handleConfirmDelete = async () => {
    if (!deleteTarget) return;
    try {
      await deleteMutation.mutateAsync(deleteTarget.id);
      showToast({
        type: 'success',
        title: t('admin.announcements.deleteSuccess', 'Announcement deleted'),
        message: '',
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: t('admin.announcements.deleteError', 'Failed to delete'),
        message: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setDeleteTarget(null);
    }
  };

  if (mode === 'create') {
    return (
      <div style={containerStyle}>
        <button
          type="button"
          onClick={() => setMode('list')}
          style={{
            fontSize: 13,
            cursor: 'pointer',
            marginBottom: 16,
            background: 'none',
            border: 'none',
            color: 'var(--ppt-brand-600, #2563eb)',
          }}
        >
          ← {t('admin.announcements.backToList', 'Back to list')}
        </button>
        <h2 style={{ margin: '0 0 20px', fontSize: 20 }}>
          {t('admin.announcements.create.heading', 'New system announcement')}
        </h2>
        <div style={cardStyle}>
          <AnnForm
            mode="create"
            onSubmit={handleCreate}
            onCancel={() => setMode('list')}
            isSubmitting={createMutation.isPending}
          />
        </div>
      </div>
    );
  }

  if (mode === 'edit' && editTarget) {
    return (
      <div style={containerStyle}>
        <button
          type="button"
          onClick={() => {
            setMode('list');
            setEditTarget(null);
          }}
          style={{
            fontSize: 13,
            cursor: 'pointer',
            marginBottom: 16,
            background: 'none',
            border: 'none',
            color: 'var(--ppt-brand-600, #2563eb)',
          }}
        >
          ← {t('admin.announcements.backToList', 'Back to list')}
        </button>
        <h2 style={{ margin: '0 0 20px', fontSize: 20 }}>
          {t('admin.announcements.edit.heading', 'Edit announcement')}
        </h2>
        <div style={cardStyle}>
          <AnnForm
            mode="edit"
            initialValues={annToForm(editTarget)}
            onSubmit={handleUpdate}
            onCancel={() => {
              setMode('list');
              setEditTarget(null);
            }}
            isSubmitting={updateMutation.isPending}
          />
        </div>
      </div>
    );
  }

  // List view
  return (
    <div style={containerStyle}>
      <div style={{ display: 'flex', alignItems: 'center', marginBottom: 20, gap: 12 }}>
        <h2
          style={{
            margin: 0,
            fontSize: 20,
            flex: 1,
            display: 'flex',
            alignItems: 'center',
            gap: 8,
          }}
        >
          {t('admin.announcements.heading', 'System Announcements')}
          <HelpTooltip text={t('admin.announcements.helpTooltip')} />
        </h2>
        <label
          style={{ display: 'flex', alignItems: 'center', gap: 6, fontSize: 13, cursor: 'pointer' }}
        >
          <input
            type="checkbox"
            checked={includeDeleted}
            onChange={(e) => setIncludeDeleted(e.target.checked)}
          />
          {t('admin.announcements.showDeleted', 'Show deleted')}
        </label>
        <button
          type="button"
          onClick={() => void refetch()}
          style={{
            padding: '6px 12px',
            border: '1px solid var(--ppt-border-default, #d1d5db)',
            borderRadius: 6,
            background: 'var(--ppt-bg-surface, #fff)',
            cursor: 'pointer',
            fontSize: 13,
          }}
        >
          {t('common.refresh', 'Refresh')}
        </button>
        {canWrite && (
          <button
            type="button"
            onClick={() => setMode('create')}
            style={{
              padding: '6px 14px',
              border: 'none',
              borderRadius: 6,
              background: 'var(--ppt-brand-600, #2563eb)',
              color: '#fff',
              cursor: 'pointer',
              fontSize: 13,
              fontWeight: 500,
            }}
          >
            + {t('admin.announcements.new', 'New announcement')}
          </button>
        )}
      </div>

      {isLoading && (
        <div
          style={{
            padding: 40,
            textAlign: 'center',
            color: 'var(--ppt-fg-muted, #6b7280)',
            fontSize: 14,
          }}
        >
          {t('common.loading', 'Loading…')}
        </div>
      )}

      {!isLoading && announcements.length === 0 && (
        <div
          style={{
            ...cardStyle,
            textAlign: 'center',
            padding: 40,
            color: 'var(--ppt-fg-muted, #6b7280)',
            fontSize: 14,
          }}
        >
          {t(
            'admin.announcements.empty',
            'No announcements yet. Create one to show a banner to all organisations.'
          )}
        </div>
      )}

      {!isLoading && announcements.length > 0 && (
        <div style={cardStyle}>
          <AnnouncementsTable
            announcements={announcements}
            canWrite={canWrite}
            isDeleting={deleteMutation.isPending}
            onEdit={(ann) => {
              setEditTarget(ann);
              setMode('edit');
            }}
            onDelete={(id) => {
              const target = announcements.find((a) => a.id === id) ?? null;
              setDeleteTarget(target);
            }}
          />
        </div>
      )}

      <DestructiveConfirmDialog
        open={deleteTarget !== null}
        title={t('admin.announcements.deleteDialog.title', 'Delete announcement?')}
        body={
          <span>
            {t(
              'admin.announcements.deleteDialog.body',
              'This will soft-delete the announcement. Type its title to confirm.'
            )}
          </span>
        }
        confirmText={deleteTarget?.title ?? ''}
        confirmLabel={t(
          'admin.announcements.deleteDialog.confirmLabel',
          'Type the title to confirm'
        )}
        onConfirm={handleConfirmDelete}
        onCancel={() => setDeleteTarget(null)}
      />
    </div>
  );
}
