/**
 * Epic 10B.1 — `/admin/tenants/agencies/:id` org drill-in.
 *
 * Renders the widened `GET /api/v1/admin/agencies/{id}` response: created /
 * updated timestamps, status, and the member / building / unit usage metrics
 * the platform_admin repo already materialises. Billing status is intentionally
 * out of scope here (no backing repo field yet — tracked as a follow-up).
 */

import { type AgencyDetail, useAgency } from '@ppt/api-client';
import type React from 'react';
import { useTranslation } from 'react-i18next';
import { Link, useParams } from 'react-router-dom';
import { HelpTooltip } from '../features/help';

function formatDateTime(iso: string | undefined): string {
  if (!iso) return '—';
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleString(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

interface MetricCardProps {
  label: string;
  value: string | number;
}

function MetricCard({ label, value }: MetricCardProps) {
  return (
    <div
      style={{
        border: '1px solid var(--ppt-border-default, #e5e7eb)',
        borderRadius: 'var(--ppt-radius-lg, 12px)',
        padding: '16px 20px',
        minWidth: 140,
        display: 'flex',
        flexDirection: 'column',
        gap: 6,
        background: 'var(--ppt-bg-surface, #fff)',
      }}
    >
      <span style={{ fontSize: 12, color: 'var(--ppt-fg-secondary, #6b7280)' }}>{label}</span>
      <span style={{ fontSize: 22, fontWeight: 600, color: 'var(--ppt-fg-primary, #111827)' }}>
        {value}
      </span>
    </div>
  );
}

const AgencyDetailPage: React.FC = () => {
  const { t } = useTranslation();
  const { id = '' } = useParams<{ id: string }>();
  const query = useAgency(id);

  const heading = (title: string) => (
    <h1 style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
      {title}
      <HelpTooltip
        text={t('admin.agencyDetail.helpTooltip', {
          defaultValue:
            'Per-organisation detail: status, creation date, and current usage metrics.',
        })}
      />
    </h1>
  );

  if (query.isLoading) {
    return (
      <section>
        {heading(t('admin.agencyDetail.title', { defaultValue: 'Agency detail' }))}
        <div role="status" aria-live="polite">
          {t('admin.agencyDetail.loading', { defaultValue: 'Loading agency…' })}
        </div>
      </section>
    );
  }

  if (query.isError) {
    const message =
      query.error instanceof Error
        ? query.error.message
        : t('admin.agencies.unknownError', { defaultValue: 'Unknown error' });
    return (
      <section>
        {heading(t('admin.agencyDetail.title', { defaultValue: 'Agency detail' }))}
        <div role="alert" className="ppt-admin-error">
          <p>
            {t('admin.agencyDetail.loadError', {
              defaultValue: 'Failed to load agency: {{message}}',
              message,
            })}
          </p>
          <button type="button" onClick={() => query.refetch()}>
            {t('admin.common.retry', { defaultValue: 'Retry' })}
          </button>
        </div>
        <p>
          <Link to="/tenants/agencies">
            {t('admin.agencyDetail.back', { defaultValue: '← Back to agencies' })}
          </Link>
        </p>
      </section>
    );
  }

  const agency = query.data as AgencyDetail;

  return (
    <section>
      <p style={{ marginBottom: 8 }}>
        <Link to="/tenants/agencies">
          {t('admin.agencyDetail.back', { defaultValue: '← Back to agencies' })}
        </Link>
      </p>
      {heading(agency.name)}

      <dl
        style={{
          display: 'grid',
          gridTemplateColumns: 'max-content 1fr',
          gap: '6px 16px',
          margin: '12px 0 24px',
        }}
      >
        <dt style={{ fontWeight: 600 }}>
          {t('admin.agencies.columns.slug', { defaultValue: 'Slug' })}
        </dt>
        <dd style={{ margin: 0 }}>{agency.slug}</dd>

        <dt style={{ fontWeight: 600 }}>
          {t('admin.agencies.columns.status', { defaultValue: 'Status' })}
        </dt>
        <dd style={{ margin: 0 }}>{agency.status}</dd>

        <dt style={{ fontWeight: 600 }}>
          {t('admin.agencyDetail.createdAt', { defaultValue: 'Created' })}
        </dt>
        <dd style={{ margin: 0 }}>{formatDateTime(agency.created_at)}</dd>

        <dt style={{ fontWeight: 600 }}>
          {t('admin.agencyDetail.updatedAt', { defaultValue: 'Last updated' })}
        </dt>
        <dd style={{ margin: 0 }}>{formatDateTime(agency.updated_at)}</dd>
      </dl>

      <h2 style={{ fontSize: 16, fontWeight: 600 }}>
        {t('admin.agencyDetail.usageTitle', { defaultValue: 'Usage' })}
      </h2>
      <div style={{ display: 'flex', flexWrap: 'wrap', gap: 16, marginTop: 12 }}>
        <MetricCard
          label={t('admin.agencyDetail.metrics.members', { defaultValue: 'Members' })}
          value={agency.member_count}
        />
        <MetricCard
          label={t('admin.agencyDetail.metrics.activeMembers', { defaultValue: 'Active members' })}
          value={agency.active_member_count}
        />
        <MetricCard
          label={t('admin.agencyDetail.metrics.buildings', { defaultValue: 'Buildings' })}
          value={agency.building_count}
        />
        <MetricCard
          label={t('admin.agencyDetail.metrics.units', { defaultValue: 'Units' })}
          value={agency.unit_count}
        />
      </div>
    </section>
  );
};

export default AgencyDetailPage;
