/**
 * Support Data page (Epic 10B, Story 10B.5).
 *
 * Displays platform-wide tenant diagnostics: user counts, active session
 * stats, and fault status summary. Data is fetched from:
 *   GET /api/v1/platform-admin/support-data
 *
 * Requires `audit_read` capability (same gate as the backend route).
 */

import type { FaultStatusCount } from '@ppt/api-client';
import { useSupportData } from '@ppt/api-client';
import type React from 'react';
import { useTranslation } from 'react-i18next';

// ---------------------------------------------------------------------------
// Stat card
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
        minWidth: 160,
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
          fontSize: 28,
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
// Fault-by-status table
// ---------------------------------------------------------------------------

interface FaultTableProps {
  rows: FaultStatusCount[];
  totalFaults: number;
}

function FaultByStatusTable({ rows, totalFaults }: FaultTableProps) {
  if (rows.length === 0) {
    return (
      <div style={{ color: 'var(--ppt-fg-muted, #6b7280)', fontSize: 13, padding: '12px 0' }}>
        No faults recorded.
      </div>
    );
  }

  return (
    <div style={{ overflowX: 'auto' }}>
      <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 13 }}>
        <thead>
          <tr>
            {['Status', 'Count', '% of total'].map((h) => (
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
          {rows.map((row) => {
            const pct =
              totalFaults > 0
                ? ((row.count / totalFaults) * 100).toLocaleString(undefined, {
                    maximumFractionDigits: 1,
                  })
                : '0';
            return (
              <tr
                key={row.status}
                style={{ borderBottom: '1px solid var(--ppt-border-default, #e5e7eb)' }}
              >
                <td style={{ padding: '8px 12px', fontWeight: 500, textTransform: 'capitalize' }}>
                  {row.status}
                </td>
                <td
                  style={{
                    padding: '8px 12px',
                    fontVariantNumeric: 'tabular-nums',
                  }}
                >
                  {row.count.toLocaleString()}
                </td>
                <td
                  style={{
                    padding: '8px 12px',
                    color: 'var(--ppt-fg-muted, #6b7280)',
                    fontVariantNumeric: 'tabular-nums',
                  }}
                >
                  {pct} %
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Section heading
// ---------------------------------------------------------------------------

function SectionHeading({ children }: { children: React.ReactNode }) {
  return (
    <h2
      style={{
        fontSize: 15,
        fontWeight: 600,
        color: 'var(--ppt-fg-primary, #111827)',
        margin: '24px 0 12px',
        paddingBottom: 8,
        borderBottom: '1px solid var(--ppt-border-default, #e5e7eb)',
      }}
    >
      {children}
    </h2>
  );
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

const SupportDataPage: React.FC = () => {
  const { t } = useTranslation();
  const { data, isLoading, isError, refetch } = useSupportData();

  const supportData = data?.data;

  return (
    <section style={{ padding: '24px 28px', maxWidth: 1000 }}>
      {/* Header */}
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <h1 style={{ fontSize: 20, fontWeight: 700, margin: 0 }}>
          {t('admin.supportData.title', 'Support Data')}
        </h1>
        <button
          type="button"
          onClick={() => void refetch()}
          disabled={isLoading}
          style={{
            padding: '6px 14px',
            borderRadius: 8,
            border: '1px solid var(--ppt-border-default, #e5e7eb)',
            background: 'transparent',
            cursor: isLoading ? 'wait' : 'pointer',
            fontSize: 13,
            fontWeight: 500,
          }}
        >
          {isLoading
            ? t('admin.common.refreshing', 'Refreshing…')
            : t('admin.common.refresh', 'Refresh')}
        </button>
      </div>

      {/* Error */}
      {isError && (
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
            'admin.supportData.loadError',
            'Failed to load support data. Check your connection and try again.'
          )}
        </div>
      )}

      {/* User counts */}
      <SectionHeading>{t('admin.supportData.userCountsTitle', 'User Counts')}</SectionHeading>
      {isLoading && (
        <div style={{ color: 'var(--ppt-fg-muted, #6b7280)', fontSize: 13 }}>
          {t('admin.common.loading', 'Loading…')}
        </div>
      )}
      {supportData && (
        <div
          style={{
            display: 'grid',
            gridTemplateColumns: 'repeat(auto-fill, minmax(160px, 1fr))',
            gap: 12,
          }}
        >
          <StatCard
            label={t('admin.supportData.totalUsers', 'Total users')}
            value={supportData.total_users}
            accent
          />
          <StatCard
            label={t('admin.supportData.activeUsers', 'Active')}
            value={supportData.active_users}
          />
          <StatCard
            label={t('admin.supportData.pendingUsers', 'Pending')}
            value={supportData.pending_users}
          />
          <StatCard
            label={t('admin.supportData.suspendedUsers', 'Suspended')}
            value={supportData.suspended_users}
          />
        </div>
      )}

      {/* Active sessions */}
      <SectionHeading>
        {t('admin.supportData.activeSessionsTitle', 'Active Sessions')}
      </SectionHeading>
      {isLoading && (
        <div style={{ color: 'var(--ppt-fg-muted, #6b7280)', fontSize: 13 }}>
          {t('admin.common.loading', 'Loading…')}
        </div>
      )}
      {supportData && (
        <div style={{ display: 'flex', gap: 12 }}>
          <StatCard
            label={t('admin.supportData.activeSessions', 'Active sessions')}
            value={supportData.active_sessions}
            accent
          />
        </div>
      )}

      {/* Fault summary */}
      <SectionHeading>{t('admin.supportData.faultSummaryTitle', 'Fault Summary')}</SectionHeading>
      {isLoading && (
        <div style={{ color: 'var(--ppt-fg-muted, #6b7280)', fontSize: 13 }}>
          {t('admin.common.loading', 'Loading…')}
        </div>
      )}
      {supportData && (
        <>
          <div style={{ marginBottom: 16 }}>
            <StatCard
              label={t('admin.supportData.totalFaults', 'Total faults')}
              value={supportData.total_faults}
              accent
            />
          </div>
          <FaultByStatusTable
            rows={supportData.fault_by_status}
            totalFaults={supportData.total_faults}
          />
        </>
      )}
    </section>
  );
};

export default SupportDataPage;
