/**
 * Organizations page (Epic 10B, Story 10B.1) — super-admin operator view of
 * all platform organizations.
 *
 * Data sources (all `/api/v1/platform-admin/*`, via @ppt/api-client admin
 * module — MFA-aware `authenticatedFetchJson`):
 *   GET  /organizations                 — paginated list (agencies_read)
 *   GET  /organizations/{id}            — detail drill-in (agencies_read)
 *   POST /organizations/{id}/suspend    — suspend + cascade session revoke
 *   POST /organizations/{id}/reactivate — reactivate (agencies_suspend)
 *   GET  /stats                         — platform stats cards (audit_read)
 */

import { useCapability } from '@ppt/admin-ui';
import type { AdminOrganizationSummary } from '@ppt/api-client';
import {
  useOrganization,
  useOrganizations,
  usePlatformStats,
  useReactivateOrganization,
  useSuspendOrganization,
} from '@ppt/api-client';
import type React from 'react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';

import { DestructiveConfirmDialog } from '../components/DestructiveConfirmDialog';
import { useToast } from '../components/Toast';

const PAGE_SIZE = 20;

// ---------------------------------------------------------------------------
// Stat card (platform stats summary)
// ---------------------------------------------------------------------------

interface StatCardProps {
  label: string;
  value: number | string;
  accent?: boolean;
}

function StatCard({ label, value, accent = false }: StatCardProps) {
  return (
    <div
      style={{
        padding: 16,
        borderRadius: 8,
        border: `1px solid ${accent ? 'var(--ppt-brand-200, #bfdbfe)' : 'var(--ppt-border-default, #e5e7eb)'}`,
        background: accent ? 'var(--ppt-brand-50, #eff6ff)' : 'var(--ppt-bg-surface, #fff)',
        display: 'flex',
        flexDirection: 'column',
        gap: 6,
        minWidth: 150,
      }}
    >
      <span
        style={{
          fontSize: 11,
          fontWeight: 600,
          letterSpacing: '0.04em',
          textTransform: 'uppercase',
          color: accent ? 'var(--ppt-brand-600, #2563eb)' : 'var(--ppt-fg-muted, #6b7280)',
        }}
      >
        {label}
      </span>
      <span
        style={{
          fontSize: 26,
          fontWeight: 700,
          color: accent ? 'var(--ppt-brand-700, #1d4ed8)' : 'var(--ppt-fg-primary, #111827)',
          fontVariantNumeric: 'tabular-nums',
        }}
      >
        {typeof value === 'number' ? value.toLocaleString() : value}
      </span>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Status badge
// ---------------------------------------------------------------------------

function StatusBadge({ status }: { status: string }) {
  const colors: Record<string, { bg: string; fg: string }> = {
    active: { bg: '#dcfce7', fg: '#166534' },
    suspended: { bg: '#fee2e2', fg: '#991b1b' },
    pending: { bg: '#fef9c3', fg: '#854d0e' },
  };
  const c = colors[status] ?? { bg: '#e5e7eb', fg: '#374151' };
  return (
    <span
      style={{
        display: 'inline-block',
        padding: '2px 10px',
        borderRadius: 999,
        fontSize: 12,
        fontWeight: 600,
        background: c.bg,
        color: c.fg,
        textTransform: 'capitalize',
      }}
    >
      {status}
    </span>
  );
}

// ---------------------------------------------------------------------------
// Modal shell
// ---------------------------------------------------------------------------

function ModalShell({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 900,
        background: 'rgba(0,0,0,0.45)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        padding: 16,
      }}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-label={label}
        style={{
          width: '100%',
          maxWidth: 480,
          maxHeight: '85vh',
          overflowY: 'auto',
          background: 'var(--ppt-bg-surface, #fff)',
          borderRadius: 12,
          padding: 24,
          display: 'flex',
          flexDirection: 'column',
          gap: 16,
          boxShadow: '0 10px 40px rgba(0,0,0,0.15)',
        }}
      >
        {children}
      </div>
    </div>
  );
}

const buttonSecondary: React.CSSProperties = {
  padding: '7px 14px',
  borderRadius: 8,
  border: '1px solid var(--ppt-border-default, #e5e7eb)',
  background: 'transparent',
  cursor: 'pointer',
  fontSize: 13,
  fontWeight: 500,
};

// ---------------------------------------------------------------------------
// Detail drill-in dialog
// ---------------------------------------------------------------------------

function DetailRow({ label, value }: { label: string; value: React.ReactNode }) {
  return (
    <div style={{ display: 'flex', justifyContent: 'space-between', gap: 12, fontSize: 13 }}>
      <span style={{ color: 'var(--ppt-fg-muted, #6b7280)' }}>{label}</span>
      <span style={{ fontWeight: 500, textAlign: 'right' }}>{value}</span>
    </div>
  );
}

interface DetailDialogProps {
  orgId: string;
  onClose: () => void;
}

function DetailDialog({ orgId, onClose }: DetailDialogProps) {
  const { t } = useTranslation();
  const { data: org, isLoading, isError } = useOrganization(orgId);

  return (
    <ModalShell label={t('admin.organizations.detail.dialogLabel', 'Organization details')}>
      <h2 style={{ margin: 0, fontSize: 16, fontWeight: 600 }}>
        {org?.name ?? t('admin.organizations.detail.title', 'Organization details')}
      </h2>
      {isLoading && (
        <div style={{ color: 'var(--ppt-fg-muted, #6b7280)', fontSize: 13 }}>
          {t('admin.common.loading', 'Loading…')}
        </div>
      )}
      {isError && (
        <div
          role="alert"
          style={{
            padding: '10px 14px',
            borderRadius: 8,
            background: '#fee2e2',
            color: '#991b1b',
            fontSize: 13,
          }}
        >
          {t('admin.organizations.detail.loadError', 'Failed to load organization details.')}
        </div>
      )}
      {org && (
        <>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
            <DetailRow label={t('admin.organizations.columns.slug', 'Slug')} value={org.slug} />
            <DetailRow
              label={t('admin.organizations.detail.contactEmail', 'Contact email')}
              value={org.contact_email}
            />
            <DetailRow
              label={t('admin.organizations.columns.status', 'Status')}
              value={<StatusBadge status={org.status} />}
            />
            <DetailRow
              label={t('admin.organizations.columns.created', 'Created')}
              value={new Date(org.created_at).toLocaleDateString()}
            />
            <DetailRow
              label={t('admin.organizations.detail.members', 'Members')}
              value={`${org.metrics.member_count.toLocaleString()} (${org.metrics.active_member_count.toLocaleString()} ${t(
                'admin.organizations.detail.activeSuffix',
                'active'
              )})`}
            />
            <DetailRow
              label={t('admin.organizations.columns.buildings', 'Buildings')}
              value={org.metrics.building_count.toLocaleString()}
            />
            <DetailRow
              label={t('admin.organizations.detail.units', 'Units')}
              value={org.metrics.unit_count.toLocaleString()}
            />
          </div>
          {org.status === 'suspended' && (
            <div
              style={{
                padding: '10px 14px',
                borderRadius: 8,
                background: '#fee2e2',
                color: '#991b1b',
                fontSize: 13,
                display: 'flex',
                flexDirection: 'column',
                gap: 4,
              }}
            >
              <strong>{t('admin.organizations.detail.suspendedHeading', 'Suspended')}</strong>
              {org.suspension_reason && (
                <span>
                  {t('admin.organizations.detail.suspensionReason', 'Reason')}:{' '}
                  {org.suspension_reason}
                </span>
              )}
              {org.suspended_at && (
                <span>
                  {t('admin.organizations.detail.suspendedAt', 'Since')}:{' '}
                  {new Date(org.suspended_at).toLocaleString()}
                </span>
              )}
            </div>
          )}
        </>
      )}
      <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
        <button type="button" onClick={onClose} style={buttonSecondary}>
          {t('admin.organizations.detail.close', 'Close')}
        </button>
      </div>
    </ModalShell>
  );
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

const OrganizationsPage: React.FC = () => {
  const { t } = useTranslation();
  const { showToast } = useToast();
  const canSuspend = useCapability('agencies_suspend');
  const canReadStats = useCapability('audit_read');

  const [page, setPage] = useState(1);
  const [statusFilter, setStatusFilter] = useState('');
  const [searchInput, setSearchInput] = useState('');
  const [search, setSearch] = useState('');

  const [suspendTarget, setSuspendTarget] = useState<AdminOrganizationSummary | null>(null);
  const [reactivateTarget, setReactivateTarget] = useState<AdminOrganizationSummary | null>(null);
  const [detailId, setDetailId] = useState<string | null>(null);

  const listQuery = useOrganizations({
    page,
    page_size: PAGE_SIZE,
    status: statusFilter || undefined,
    search: search || undefined,
  });
  // /stats requires audit_read server-side — skip the guaranteed-403 otherwise.
  const statsQuery = usePlatformStats({ enabled: canReadStats });

  const suspend = useSuspendOrganization();
  const reactivate = useReactivateOrganization();

  const handleSuspendConfirm = async (reason: string) => {
    if (!suspendTarget) return;
    try {
      const result = await suspend.mutateAsync({ id: suspendTarget.id, data: { reason } });
      showToast({
        type: 'success',
        title: t('admin.organizations.suspend.successTitle', 'Organization suspended'),
        message: result.message,
      });
      setSuspendTarget(null);
    } catch {
      showToast({
        type: 'error',
        title: t('admin.organizations.suspend.failedTitle', 'Suspend failed'),
        message: t(
          'admin.organizations.suspend.failedMessage',
          'Could not suspend the organization.'
        ),
      });
    }
  };

  const handleReactivateConfirm = async (note: string) => {
    if (!reactivateTarget) return;
    try {
      const result = await reactivate.mutateAsync({
        id: reactivateTarget.id,
        data: { note: note ? note : null },
      });
      showToast({
        type: 'success',
        title: t('admin.organizations.reactivate.successTitle', 'Organization reactivated'),
        message: result.message,
      });
      setReactivateTarget(null);
    } catch {
      showToast({
        type: 'error',
        title: t('admin.organizations.reactivate.failedTitle', 'Reactivate failed'),
        message: t(
          'admin.organizations.reactivate.failedMessage',
          'Could not reactivate the organization.'
        ),
      });
    }
  };

  const orgs = listQuery.data?.organizations ?? [];
  const total = listQuery.data?.total ?? 0;
  const totalPages = Math.max(1, Math.ceil(total / PAGE_SIZE));
  const stats = statsQuery.data?.stats;

  const applySearch = () => {
    setPage(1);
    setSearch(searchInput.trim());
  };

  return (
    <section style={{ padding: '24px 28px', maxWidth: 1100 }}>
      {/* Header */}
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <h1 style={{ fontSize: 20, fontWeight: 700, margin: 0 }}>
          {t('admin.organizations.title', 'Organizations')}
        </h1>
        <button
          type="button"
          onClick={() => void listQuery.refetch()}
          disabled={listQuery.isLoading}
          style={{ ...buttonSecondary, cursor: listQuery.isLoading ? 'wait' : 'pointer' }}
        >
          {listQuery.isLoading
            ? t('admin.common.refreshing', 'Refreshing…')
            : t('admin.common.refresh', 'Refresh')}
        </button>
      </div>

      {/* Platform stats summary cards */}
      {canReadStats && stats && (
        <div
          style={{
            display: 'grid',
            gridTemplateColumns: 'repeat(auto-fill, minmax(150px, 1fr))',
            gap: 12,
            marginTop: 16,
          }}
        >
          <StatCard
            label={t('admin.organizations.stats.activeOrgs', 'Active orgs')}
            value={stats.active_orgs}
            accent
          />
          <StatCard
            label={t('admin.organizations.stats.suspendedOrgs', 'Suspended orgs')}
            value={stats.suspended_orgs}
          />
          <StatCard
            label={t('admin.organizations.stats.activeUsers', 'Active users')}
            value={stats.active_users}
          />
          <StatCard
            label={t('admin.organizations.stats.totalBuildings', 'Buildings')}
            value={stats.total_buildings}
          />
          <StatCard
            label={t('admin.organizations.stats.totalUnits', 'Units')}
            value={stats.total_units}
          />
        </div>
      )}

      {/* Filters */}
      <div
        style={{
          display: 'flex',
          gap: 10,
          alignItems: 'center',
          marginTop: 20,
          flexWrap: 'wrap',
        }}
      >
        <input
          type="search"
          value={searchInput}
          onChange={(e) => setSearchInput(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') applySearch();
          }}
          placeholder={t('admin.organizations.searchPlaceholder', 'Search by name or slug…')}
          aria-label={t('admin.organizations.searchLabel', 'Search organizations')}
          style={{
            padding: '7px 12px',
            borderRadius: 8,
            border: '1px solid var(--ppt-border-default, #e5e7eb)',
            fontSize: 13,
            minWidth: 240,
          }}
        />
        <button type="button" onClick={applySearch} style={buttonSecondary}>
          {t('admin.organizations.searchButton', 'Search')}
        </button>
        <select
          value={statusFilter}
          onChange={(e) => {
            setPage(1);
            setStatusFilter(e.target.value);
          }}
          aria-label={t('admin.organizations.statusFilterLabel', 'Filter by status')}
          style={{
            padding: '7px 12px',
            borderRadius: 8,
            border: '1px solid var(--ppt-border-default, #e5e7eb)',
            fontSize: 13,
            background: 'var(--ppt-bg-surface, #fff)',
          }}
        >
          <option value="">{t('admin.organizations.statusAll', 'All statuses')}</option>
          <option value="active">{t('admin.organizations.statusActive', 'Active')}</option>
          <option value="suspended">{t('admin.organizations.statusSuspended', 'Suspended')}</option>
        </select>
      </div>

      {/* Error */}
      {listQuery.isError && (
        <div
          role="alert"
          style={{
            marginTop: 16,
            padding: '12px 16px',
            borderRadius: 8,
            background: '#fee2e2',
            color: '#991b1b',
            fontSize: 13,
          }}
        >
          {t(
            'admin.organizations.loadError',
            'Failed to load organizations. Check your connection and try again.'
          )}
        </div>
      )}

      {/* Loading */}
      {listQuery.isLoading && (
        <div style={{ marginTop: 16, color: 'var(--ppt-fg-muted, #6b7280)', fontSize: 13 }}>
          {t('admin.common.loading', 'Loading…')}
        </div>
      )}

      {/* Empty */}
      {!listQuery.isLoading && !listQuery.isError && orgs.length === 0 && (
        <div style={{ marginTop: 16, color: 'var(--ppt-fg-muted, #6b7280)', fontSize: 13 }}>
          {t('admin.organizations.empty', 'No organizations.')}
        </div>
      )}

      {/* Table */}
      {orgs.length > 0 && (
        <div style={{ overflowX: 'auto', marginTop: 16 }}>
          <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 13 }}>
            <thead>
              <tr>
                {[
                  t('admin.organizations.columns.name', 'Name'),
                  t('admin.organizations.columns.slug', 'Slug'),
                  t('admin.organizations.columns.members', 'Members'),
                  t('admin.organizations.columns.buildings', 'Buildings'),
                  t('admin.organizations.columns.created', 'Created'),
                  t('admin.organizations.columns.status', 'Status'),
                  t('admin.common.actions', 'Actions'),
                ].map((h) => (
                  <th
                    key={h}
                    style={{
                      textAlign: 'left',
                      padding: '8px 12px',
                      borderBottom: '2px solid var(--ppt-border-default, #e5e7eb)',
                      color: 'var(--ppt-fg-muted, #6b7280)',
                      fontWeight: 600,
                    }}
                  >
                    {h}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {orgs.map((org) => (
                <tr
                  key={org.id}
                  style={{
                    borderBottom: '1px solid var(--ppt-border-default, #e5e7eb)',
                    background: org.status === 'suspended' ? '#fef2f2' : 'transparent',
                  }}
                >
                  <td style={{ padding: '8px 12px', fontWeight: 500 }}>{org.name}</td>
                  <td style={{ padding: '8px 12px', color: 'var(--ppt-fg-muted, #6b7280)' }}>
                    {org.slug}
                  </td>
                  <td style={{ padding: '8px 12px', fontVariantNumeric: 'tabular-nums' }}>
                    {org.member_count.toLocaleString()}
                  </td>
                  <td style={{ padding: '8px 12px', fontVariantNumeric: 'tabular-nums' }}>
                    {org.building_count.toLocaleString()}
                  </td>
                  <td style={{ padding: '8px 12px' }}>
                    {new Date(org.created_at).toLocaleDateString()}
                  </td>
                  <td style={{ padding: '8px 12px' }}>
                    <StatusBadge status={org.status} />
                  </td>
                  <td style={{ padding: '8px 12px', whiteSpace: 'nowrap' }}>
                    <button
                      type="button"
                      onClick={() => setDetailId(org.id)}
                      style={{ ...buttonSecondary, padding: '4px 10px', fontSize: 12 }}
                    >
                      {t('admin.organizations.actions.details', 'Details')}
                    </button>
                    {canSuspend && org.status === 'active' && (
                      <button
                        type="button"
                        onClick={() => setSuspendTarget(org)}
                        style={{
                          ...buttonSecondary,
                          padding: '4px 10px',
                          fontSize: 12,
                          marginLeft: 8,
                          color: 'var(--ppt-danger-600, #dc2626)',
                          borderColor: 'var(--ppt-danger-200, #fecaca)',
                        }}
                      >
                        {t('admin.organizations.actions.suspend', 'Suspend')}
                      </button>
                    )}
                    {canSuspend && org.status === 'suspended' && (
                      <button
                        type="button"
                        onClick={() => setReactivateTarget(org)}
                        style={{
                          ...buttonSecondary,
                          padding: '4px 10px',
                          fontSize: 12,
                          marginLeft: 8,
                          color: 'var(--ppt-brand-600, #2563eb)',
                        }}
                      >
                        {t('admin.organizations.actions.reactivate', 'Reactivate')}
                      </button>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {/* Pagination */}
      {total > 0 && (
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 12,
            marginTop: 16,
            fontSize: 13,
            color: 'var(--ppt-fg-muted, #6b7280)',
          }}
        >
          <button
            type="button"
            onClick={() => setPage((p) => Math.max(1, p - 1))}
            disabled={page <= 1}
            style={{ ...buttonSecondary, opacity: page <= 1 ? 0.5 : 1 }}
          >
            {t('admin.organizations.pagination.previous', 'Previous')}
          </button>
          <span>
            {t('admin.organizations.pagination.pageOf', 'Page {{page}} of {{totalPages}}', {
              page,
              totalPages,
            })}{' '}
            ·{' '}
            {t('admin.organizations.pagination.total', '{{total}} organizations', {
              total,
            })}
          </span>
          <button
            type="button"
            onClick={() => setPage((p) => Math.min(totalPages, p + 1))}
            disabled={page >= totalPages}
            style={{ ...buttonSecondary, opacity: page >= totalPages ? 0.5 : 1 }}
          >
            {t('admin.organizations.pagination.next', 'Next')}
          </button>
        </div>
      )}

      {/* Dialogs */}
      <DestructiveConfirmDialog
        open={suspendTarget !== null}
        title={t('admin.organizations.suspend.title', 'Suspend {{name}}?', {
          name: suspendTarget?.name ?? '',
        })}
        body={t(
          'admin.organizations.suspend.warning',
          'All members of this organization will be logged out immediately and will not be able to sign in until the organization is reactivated.'
        )}
        confirmText={suspendTarget?.name ?? ''}
        confirmLabel={t('admin.organizations.suspend.confirmLabel', 'Type the name to confirm')}
        reasonAction="agency_suspend"
        onConfirm={handleSuspendConfirm}
        onCancel={() => setSuspendTarget(null)}
      />
      <DestructiveConfirmDialog
        open={reactivateTarget !== null}
        title={t('admin.organizations.reactivate.title', 'Reactivate {{name}}?', {
          name: reactivateTarget?.name ?? '',
        })}
        body={t(
          'admin.organizations.reactivate.info',
          'Members of this organization will be able to sign in again.'
        )}
        confirmText={reactivateTarget?.name ?? ''}
        confirmLabel={t('admin.organizations.reactivate.confirmLabel', 'Type the name to confirm')}
        reasonAction="agency_reactivate"
        reasonRequired={false}
        onConfirm={handleReactivateConfirm}
        onCancel={() => setReactivateTarget(null)}
      />
      {detailId && <DetailDialog orgId={detailId} onClose={() => setDetailId(null)} />}
    </section>
  );
};

export default OrganizationsPage;
