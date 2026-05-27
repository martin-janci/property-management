/**
 * Read-only list table for system announcements, with edit/delete actions.
 * Extracted from SystemAnnouncementsPage.tsx (#521).
 */

import type { SystemAnnouncement } from '@ppt/api-client';
import { useTranslation } from 'react-i18next';
import { formatDatetime, lifecycleOf } from '../lib/formatters';
import { SeverityBadge } from './SeverityBadge';

interface AnnouncementsTableProps {
  announcements: SystemAnnouncement[];
  canWrite: boolean;
  isDeleting: boolean;
  onEdit: (ann: SystemAnnouncement) => void;
  onDelete: (id: string) => void;
}

export function AnnouncementsTable({
  announcements,
  canWrite,
  isDeleting,
  onEdit,
  onDelete,
}: AnnouncementsTableProps) {
  const { t } = useTranslation();

  const statusLabel = (lifecycle: 'active' | 'scheduled' | 'expired') => {
    switch (lifecycle) {
      case 'active':
        return t('admin.announcements.status.active', 'Active');
      case 'scheduled':
        return t('admin.announcements.status.scheduled', 'Scheduled');
      case 'expired':
        return t('admin.announcements.status.expired', 'Expired');
    }
  };

  return (
    <table
      style={{ width: '100%', borderCollapse: 'collapse', fontSize: 13 }}
      aria-label={t('admin.announcements.tableLabel', 'System announcements')}
    >
      <thead>
        <tr style={{ borderBottom: '1px solid var(--ppt-border-default, #e5e7eb)' }}>
          {[
            { key: 'title', label: t('admin.announcements.col.title', 'Title') },
            { key: 'severity', label: t('admin.announcements.col.severity', 'Severity') },
            { key: 'status', label: t('admin.announcements.col.status', 'Status') },
            { key: 'publishAt', label: t('admin.announcements.col.publishAt', 'Publish at') },
            { key: 'expireAt', label: t('admin.announcements.col.expireAt', 'Expire at') },
            { key: 'actions', label: t('admin.announcements.col.actions', 'Actions') },
          ].map(({ key, label }) => (
            <th
              key={key}
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
              {label}
            </th>
          ))}
        </tr>
      </thead>
      <tbody>
        {announcements.map((ann) => {
          const lifecycle = lifecycleOf(ann);
          const statusColor =
            lifecycle === 'active' ? '#065f46' : lifecycle === 'scheduled' ? '#1e40af' : '#6b7280';
          const statusBg =
            lifecycle === 'active' ? '#d1fae5' : lifecycle === 'scheduled' ? '#dbeafe' : '#f3f4f6';

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
                    {t('admin.announcements.ackBadge', 'ACK')}
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
                  {statusLabel(lifecycle)}
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
                      onClick={() => onEdit(ann)}
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
                      onClick={() => onDelete(ann.id)}
                      disabled={isDeleting}
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
  );
}
