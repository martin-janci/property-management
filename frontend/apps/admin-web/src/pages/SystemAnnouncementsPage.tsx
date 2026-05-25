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
 */

import { useCapability } from '@ppt/admin-ui';
import type {
  CreateSystemAnnouncementRequest,
  SystemAnnouncement,
  SystemAnnouncementSeverity,
  UpdateSystemAnnouncementRequest,
} from '@ppt/api-client';
import {
  useCreateSystemAnnouncement,
  useDeleteSystemAnnouncement,
  useSystemAnnouncements,
  useUpdateSystemAnnouncement,
} from '@ppt/api-client';
import type { CSSProperties, FormEvent } from 'react';
import { useEffect, useId, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useToast } from '../components/Toast';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatDatetime(iso: string): string {
  try {
    return new Intl.DateTimeFormat('en-GB', {
      dateStyle: 'medium',
      timeStyle: 'short',
    }).format(new Date(iso));
  } catch {
    return iso;
  }
}

function toLocalDatetimeInput(iso: string | null | undefined): string {
  if (!iso) return '';
  try {
    const d = new Date(iso);
    const pad = (n: number) => String(n).padStart(2, '0');
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
  } catch {
    return '';
  }
}

function toIso(localDatetime: string): string {
  if (!localDatetime) return '';
  return new Date(localDatetime).toISOString();
}

function isActive(ann: SystemAnnouncement): boolean {
  const now = Date.now();
  const start = new Date(ann.start_at).getTime();
  const end = ann.end_at ? new Date(ann.end_at).getTime() : Number.POSITIVE_INFINITY;
  return start <= now && now < end;
}

function isScheduled(ann: SystemAnnouncement): boolean {
  return new Date(ann.start_at).getTime() > Date.now();
}

// ---------------------------------------------------------------------------
// Severity badge
// ---------------------------------------------------------------------------

const SEVERITY_STYLE: Record<string, { bg: string; fg: string; border: string }> = {
  info: { bg: '#eff6ff', fg: '#1e40af', border: '#bfdbfe' },
  warning: { bg: '#fffbeb', fg: '#92400e', border: '#fde68a' },
  critical: { bg: '#fef2f2', fg: '#991b1b', border: '#fecaca' },
};

function getSeverityStyle(severity: string) {
  return SEVERITY_STYLE[severity] ?? SEVERITY_STYLE.info;
}

function SeverityBadge({ severity }: { severity: SystemAnnouncementSeverity }) {
  const style = getSeverityStyle(severity);
  return (
    <span
      style={{
        display: 'inline-block',
        padding: '2px 8px',
        borderRadius: 9999,
        fontSize: 11,
        fontWeight: 600,
        background: style.bg,
        color: style.fg,
        border: `1px solid ${style.border}`,
        textTransform: 'uppercase',
        letterSpacing: '0.04em',
      }}
    >
      {severity}
    </span>
  );
}

// ---------------------------------------------------------------------------
// Banner preview
// ---------------------------------------------------------------------------

function BannerPreview({
  title,
  message,
  severity,
  isDismissible,
}: {
  title: string;
  message: string;
  severity: SystemAnnouncementSeverity;
  isDismissible: boolean;
}) {
  const style = getSeverityStyle(severity);
  return (
    <div
      role="alert"
      style={{
        background: style.bg,
        border: `1px solid ${style.border}`,
        borderRadius: 8,
        padding: '12px 16px',
        display: 'flex',
        alignItems: 'flex-start',
        gap: 12,
      }}
    >
      <div style={{ flex: 1 }}>
        <div style={{ fontWeight: 600, color: style.fg, fontSize: 14 }}>
          {title || <em style={{ opacity: 0.5 }}>Announcement title</em>}
        </div>
        {message && (
          <div style={{ fontSize: 13, marginTop: 4, color: style.fg, opacity: 0.9 }}>{message}</div>
        )}
      </div>
      {isDismissible && (
        <button
          type="button"
          aria-label="Dismiss (preview only)"
          style={{
            background: 'none',
            border: 'none',
            cursor: 'default',
            color: style.fg,
            fontSize: 16,
            opacity: 0.6,
            padding: 0,
          }}
        >
          ×
        </button>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Announcement form (create / edit)
// ---------------------------------------------------------------------------

interface AnnFormValues {
  title: string;
  message: string;
  severity: SystemAnnouncementSeverity;
  start_at: string;
  end_at: string;
  is_dismissible: boolean;
  requires_acknowledgment: boolean;
}

const EMPTY_FORM: AnnFormValues = {
  title: '',
  message: '',
  severity: 'info',
  start_at: toLocalDatetimeInput(new Date().toISOString()),
  end_at: '',
  is_dismissible: true,
  requires_acknowledgment: false,
};

function annToForm(ann: SystemAnnouncement): AnnFormValues {
  return {
    title: ann.title,
    message: ann.message,
    severity: ann.severity,
    start_at: toLocalDatetimeInput(ann.start_at),
    end_at: toLocalDatetimeInput(ann.end_at),
    is_dismissible: ann.is_dismissible,
    requires_acknowledgment: ann.requires_acknowledgment,
  };
}

const labelStyle: CSSProperties = {
  display: 'block',
  fontSize: 13,
  fontWeight: 500,
  marginBottom: 4,
  color: 'var(--ppt-fg-secondary, #374151)',
};

const inputStyle: CSSProperties = {
  width: '100%',
  padding: '7px 10px',
  border: '1px solid var(--ppt-border-default, #d1d5db)',
  borderRadius: 6,
  fontSize: 13,
  background: 'var(--ppt-bg-surface, #fff)',
  boxSizing: 'border-box',
};

interface AnnFormProps {
  initialValues?: AnnFormValues;
  onSubmit: (values: AnnFormValues) => void;
  onCancel: () => void;
  isSubmitting: boolean;
  mode: 'create' | 'edit';
}

function AnnForm({ initialValues, onSubmit, onCancel, isSubmitting, mode }: AnnFormProps) {
  const { t } = useTranslation();
  const [values, setValues] = useState<AnnFormValues>(initialValues ?? EMPTY_FORM);
  const formId = useId();

  const set = (patch: Partial<AnnFormValues>) => setValues((v) => ({ ...v, ...patch }));

  const handleSubmit = (e: FormEvent) => {
    e.preventDefault();
    onSubmit(values);
  };

  return (
    <form
      id={formId}
      onSubmit={handleSubmit}
      style={{ display: 'flex', flexDirection: 'column', gap: 14 }}
    >
      {/* Preview */}
      <div>
        <div style={{ ...labelStyle, marginBottom: 6 }}>
          {t('admin.announcements.form.preview', 'Banner preview')}
        </div>
        <BannerPreview
          title={values.title}
          message={values.message}
          severity={values.severity}
          isDismissible={values.is_dismissible}
        />
      </div>

      {/* Title */}
      <div>
        <label htmlFor={`${formId}-title`} style={labelStyle}>
          {t('admin.announcements.form.title', 'Title')} *
        </label>
        <input
          id={`${formId}-title`}
          type="text"
          required
          maxLength={200}
          value={values.title}
          onChange={(e) => set({ title: e.target.value })}
          style={inputStyle}
          placeholder={t(
            'admin.announcements.form.titlePlaceholder',
            'Short headline visible in the banner'
          )}
        />
      </div>

      {/* Message */}
      <div>
        <label htmlFor={`${formId}-message`} style={labelStyle}>
          {t('admin.announcements.form.message', 'Message')} *
        </label>
        <textarea
          id={`${formId}-message`}
          required
          rows={3}
          maxLength={2000}
          value={values.message}
          onChange={(e) => set({ message: e.target.value })}
          style={{ ...inputStyle, resize: 'vertical' }}
          placeholder={t(
            'admin.announcements.form.messagePlaceholder',
            'Detailed description shown below the title'
          )}
        />
      </div>

      {/* Severity */}
      <div>
        <label htmlFor={`${formId}-severity`} style={labelStyle}>
          {t('admin.announcements.form.severity', 'Severity')}
        </label>
        <select
          id={`${formId}-severity`}
          value={values.severity}
          onChange={(e) => set({ severity: e.target.value as SystemAnnouncementSeverity })}
          style={inputStyle}
        >
          <option value="info">Info</option>
          <option value="warning">Warning</option>
          <option value="critical">Critical</option>
        </select>
      </div>

      {/* Dates */}
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12 }}>
        <div>
          <label htmlFor={`${formId}-start`} style={labelStyle}>
            {t('admin.announcements.form.startAt', 'Publish at')} *
          </label>
          <input
            id={`${formId}-start`}
            type="datetime-local"
            required
            value={values.start_at}
            onChange={(e) => set({ start_at: e.target.value })}
            style={inputStyle}
          />
        </div>
        <div>
          <label htmlFor={`${formId}-end`} style={labelStyle}>
            {t('admin.announcements.form.endAt', 'Expire at')}
            <span style={{ fontWeight: 400, opacity: 0.6 }}>
              {' '}
              ({t('admin.announcements.form.endAtOptional', 'optional')})
            </span>
          </label>
          <input
            id={`${formId}-end`}
            type="datetime-local"
            value={values.end_at}
            onChange={(e) => set({ end_at: e.target.value })}
            style={inputStyle}
          />
        </div>
      </div>

      {/* Flags */}
      <div style={{ display: 'flex', gap: 20 }}>
        <label
          style={{ display: 'flex', alignItems: 'center', gap: 6, fontSize: 13, cursor: 'pointer' }}
        >
          <input
            type="checkbox"
            checked={values.is_dismissible}
            onChange={(e) => set({ is_dismissible: e.target.checked })}
          />
          {t('admin.announcements.form.isDismissible', 'Users can dismiss')}
        </label>
        <label
          style={{ display: 'flex', alignItems: 'center', gap: 6, fontSize: 13, cursor: 'pointer' }}
        >
          <input
            type="checkbox"
            checked={values.requires_acknowledgment}
            onChange={(e) => set({ requires_acknowledgment: e.target.checked })}
          />
          {t('admin.announcements.form.requiresAck', 'Requires acknowledgment')}
        </label>
      </div>

      {/* Actions */}
      <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 8, paddingTop: 4 }}>
        <button
          type="button"
          onClick={onCancel}
          style={{
            padding: '7px 16px',
            border: '1px solid var(--ppt-border-default, #d1d5db)',
            borderRadius: 6,
            background: 'var(--ppt-bg-surface, #fff)',
            cursor: 'pointer',
            fontSize: 13,
          }}
        >
          {t('common.cancel', 'Cancel')}
        </button>
        <button
          type="submit"
          form={formId}
          disabled={isSubmitting}
          style={{
            padding: '7px 16px',
            border: 'none',
            borderRadius: 6,
            background: 'var(--ppt-brand-600, #2563eb)',
            color: '#fff',
            cursor: isSubmitting ? 'not-allowed' : 'pointer',
            fontSize: 13,
            fontWeight: 500,
            opacity: isSubmitting ? 0.6 : 1,
          }}
        >
          {isSubmitting
            ? t('common.saving', 'Saving…')
            : mode === 'create'
              ? t('admin.announcements.form.create', 'Create announcement')
              : t('admin.announcements.form.save', 'Save changes')}
        </button>
      </div>
    </form>
  );
}

// ---------------------------------------------------------------------------
// Main page
// ---------------------------------------------------------------------------

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

  const handleDelete = async (id: string) => {
    if (
      !window.confirm(
        t(
          'admin.announcements.deleteConfirm',
          'Delete this announcement? This action cannot be undone.'
        )
      )
    ) {
      return;
    }
    try {
      await deleteMutation.mutateAsync(id);
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
      {/* Header */}
      <div style={{ display: 'flex', alignItems: 'center', marginBottom: 20, gap: 12 }}>
        <h2 style={{ margin: 0, fontSize: 20, flex: 1 }}>
          {t('admin.announcements.heading', 'System Announcements')}
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

      {/* Loading */}
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

      {/* Empty */}
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

      {/* Table */}
      {!isLoading && announcements.length > 0 && (
        <div style={cardStyle}>
          <table
            style={{ width: '100%', borderCollapse: 'collapse', fontSize: 13 }}
            aria-label={t('admin.announcements.tableLabel', 'System announcements')}
          >
            <thead>
              <tr style={{ borderBottom: '1px solid var(--ppt-border-default, #e5e7eb)' }}>
                {['Title', 'Severity', 'Status', 'Publish at', 'Expire at', 'Actions'].map((h) => (
                  <th
                    key={h}
                    style={{
                      padding: '8px 12px',
                      textAlign: 'left',
                      fontWeight: 600,
                      fontSize: 12,
                      color: 'var(--ppt-fg-muted, #6b7280)',
                      textTransform: 'uppercase',
                      letterSpacing: '0.04em',
                    }}
                  >
                    {h}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {announcements.map((ann) => {
                const active = isActive(ann);
                const scheduled = isScheduled(ann);
                const statusLabel = active ? 'Active' : scheduled ? 'Scheduled' : 'Expired';
                const statusColor = active ? '#065f46' : scheduled ? '#1e40af' : '#6b7280';
                const statusBg = active ? '#d1fae5' : scheduled ? '#dbeafe' : '#f3f4f6';

                return (
                  <tr
                    key={ann.id}
                    style={{ borderBottom: '1px solid var(--ppt-border-subtle, #f3f4f6)' }}
                  >
                    <td
                      style={{
                        padding: '10px 12px',
                        fontWeight: 500,
                        maxWidth: 260,
                        overflow: 'hidden',
                        textOverflow: 'ellipsis',
                        whiteSpace: 'nowrap',
                      }}
                    >
                      {ann.title}
                      {ann.requires_acknowledgment && (
                        <span
                          style={{
                            marginLeft: 6,
                            fontSize: 11,
                            color: '#92400e',
                            background: '#fef3c7',
                            borderRadius: 4,
                            padding: '1px 5px',
                          }}
                        >
                          ACK
                        </span>
                      )}
                    </td>
                    <td style={{ padding: '10px 12px' }}>
                      <SeverityBadge severity={ann.severity} />
                    </td>
                    <td style={{ padding: '10px 12px' }}>
                      <span
                        style={{
                          display: 'inline-block',
                          padding: '2px 8px',
                          borderRadius: 9999,
                          fontSize: 11,
                          fontWeight: 600,
                          background: statusBg,
                          color: statusColor,
                        }}
                      >
                        {statusLabel}
                      </span>
                    </td>
                    <td
                      style={{
                        padding: '10px 12px',
                        color: 'var(--ppt-fg-muted, #6b7280)',
                        whiteSpace: 'nowrap',
                      }}
                    >
                      {formatDatetime(ann.start_at)}
                    </td>
                    <td
                      style={{
                        padding: '10px 12px',
                        color: 'var(--ppt-fg-muted, #6b7280)',
                        whiteSpace: 'nowrap',
                      }}
                    >
                      {ann.end_at ? formatDatetime(ann.end_at) : '—'}
                    </td>
                    <td style={{ padding: '10px 12px' }}>
                      <div style={{ display: 'flex', gap: 8 }}>
                        {canWrite && (
                          <button
                            type="button"
                            onClick={() => {
                              setEditTarget(ann);
                              setMode('edit');
                            }}
                            style={{
                              padding: '4px 10px',
                              fontSize: 12,
                              border: '1px solid var(--ppt-border-default, #d1d5db)',
                              borderRadius: 5,
                              background: 'var(--ppt-bg-surface, #fff)',
                              cursor: 'pointer',
                            }}
                          >
                            {t('common.edit', 'Edit')}
                          </button>
                        )}
                        {canWrite && (
                          <button
                            type="button"
                            onClick={() => void handleDelete(ann.id)}
                            disabled={deleteMutation.isPending}
                            style={{
                              padding: '4px 10px',
                              fontSize: 12,
                              border: '1px solid #fca5a5',
                              borderRadius: 5,
                              background: '#fef2f2',
                              color: '#991b1b',
                              cursor: 'pointer',
                            }}
                          >
                            {t('common.delete', 'Delete')}
                          </button>
                        )}
                      </div>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
