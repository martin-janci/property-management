/**
 * Platform Health Monitoring page (Epic 10B, Story 10B.3).
 *
 * Displays the health dashboard (current metrics + active alerts + thresholds),
 * lets the operator drill into metric history with a time-range selector, and
 * allows acknowledging active alerts (requires site_settings_write).
 *
 * Backend endpoints consumed:
 *   GET  /api/v1/platform-admin/health/dashboard
 *   GET  /api/v1/platform-admin/health/metrics/{name}/history?range=<1h|6h|24h|7d|30d>
 *   GET  /api/v1/platform-admin/health/alerts?active_only=true
 *   POST /api/v1/platform-admin/health/alerts/{id}/acknowledge
 *   GET  /api/v1/platform-admin/health/thresholds
 *   PUT  /api/v1/platform-admin/health/thresholds/{name}
 */

import { useCapability } from '@ppt/admin-ui';
import type {
  CurrentMetric,
  MetricAlert,
  MetricStatus,
  MetricThreshold,
  TimeRange,
  UpdateThresholdRequest,
} from '@ppt/api-client';
import {
  useAcknowledgeAlert,
  useHealthAlerts,
  useHealthDashboard,
  useMetricHistory,
  useUpdateHealthThreshold,
} from '@ppt/api-client';
import type React from 'react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useToast } from '../components/Toast';
import { HelpTooltip } from '../features/help';

// ---------------------------------------------------------------------------
// Status badge
// ---------------------------------------------------------------------------

const STATUS_COLORS: Record<MetricStatus, { bg: string; fg: string; label: string }> = {
  normal: { bg: '#d1fae5', fg: '#065f46', label: 'Normal' },
  warning: { bg: '#fef3c7', fg: '#92400e', label: 'Warning' },
  critical: { bg: '#fee2e2', fg: '#991b1b', label: 'Critical' },
};

function StatusBadge({ status }: { status: MetricStatus }) {
  const { bg, fg, label } = STATUS_COLORS[status] ?? STATUS_COLORS.normal;
  return (
    <span
      style={{
        display: 'inline-block',
        padding: '2px 8px',
        borderRadius: 9999,
        fontSize: 11,
        fontWeight: 600,
        background: bg,
        color: fg,
        textTransform: 'uppercase',
        letterSpacing: '0.04em',
      }}
    >
      {label}
    </span>
  );
}

// ---------------------------------------------------------------------------
// Metric card
// ---------------------------------------------------------------------------

interface MetricCardProps {
  metric: CurrentMetric;
  onDrill: (name: string) => void;
}

function MetricCard({ metric, onDrill }: MetricCardProps) {
  return (
    <div
      style={{
        padding: 16,
        borderRadius: 8,
        border: '1px solid var(--ppt-border-default, #e5e7eb)',
        background: 'var(--ppt-bg-surface, #fff)',
        display: 'flex',
        flexDirection: 'column',
        gap: 8,
        minWidth: 200,
      }}
    >
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
        <span
          style={{
            fontSize: 12,
            color: 'var(--ppt-fg-muted, #6b7280)',
            fontWeight: 500,
            textTransform: 'uppercase',
            letterSpacing: '0.04em',
          }}
        >
          {metric.metric_name}
        </span>
        <StatusBadge status={metric.status} />
      </div>
      <div style={{ fontSize: 28, fontWeight: 700, color: 'var(--ppt-fg-primary, #111827)' }}>
        {metric.value.toLocaleString(undefined, { maximumFractionDigits: 2 })}
      </div>
      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          fontSize: 11,
          color: 'var(--ppt-fg-muted, #6b7280)',
        }}
      >
        <span>{metric.metric_type}</span>
        <button
          type="button"
          onClick={() => onDrill(metric.metric_name)}
          style={{
            fontSize: 11,
            color: 'var(--ppt-brand-600, #2563eb)',
            background: 'none',
            border: 'none',
            cursor: 'pointer',
            padding: '2px 6px',
            borderRadius: 4,
          }}
        >
          History
        </button>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Metric history panel
// ---------------------------------------------------------------------------

interface MetricHistoryPanelProps {
  metricName: string;
  onClose: () => void;
}

const RANGE_LABELS: Record<TimeRange, string> = {
  '1h': '1 hour',
  '6h': '6 hours',
  '24h': '24 hours',
  '7d': '7 days',
  '30d': '30 days',
};

function MetricHistoryPanel({ metricName, onClose }: MetricHistoryPanelProps) {
  const [range, setRange] = useState<TimeRange>('24h');
  const { data, isLoading, isError } = useMetricHistory(metricName, range);

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
        aria-label={`Metric history for ${metricName}`}
        style={{
          width: '100%',
          maxWidth: 700,
          background: 'var(--ppt-bg-surface, #fff)',
          borderRadius: 12,
          padding: 24,
          display: 'flex',
          flexDirection: 'column',
          gap: 16,
          boxShadow: '0 10px 40px rgba(0,0,0,0.15)',
          maxHeight: '90vh',
          overflow: 'auto',
        }}
      >
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <h2 style={{ margin: 0, fontSize: 16, fontWeight: 600 }}>Metric history: {metricName}</h2>
          <button
            type="button"
            onClick={onClose}
            style={{
              background: 'none',
              border: 'none',
              cursor: 'pointer',
              fontSize: 20,
              color: 'var(--ppt-fg-muted, #6b7280)',
              lineHeight: 1,
            }}
            aria-label="Close"
          >
            &times;
          </button>
        </div>

        {/* Range selector */}
        <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
          {(Object.keys(RANGE_LABELS) as TimeRange[]).map((r) => (
            <button
              key={r}
              type="button"
              onClick={() => setRange(r)}
              style={{
                padding: '4px 12px',
                borderRadius: 6,
                border: '1px solid var(--ppt-border-default, #e5e7eb)',
                background: r === range ? 'var(--ppt-brand-600, #2563eb)' : 'transparent',
                color: r === range ? '#fff' : 'var(--ppt-fg-secondary, #374151)',
                cursor: 'pointer',
                fontSize: 13,
                fontWeight: 500,
              }}
            >
              {RANGE_LABELS[r]}
            </button>
          ))}
        </div>

        {isLoading && (
          <div style={{ color: 'var(--ppt-fg-muted, #6b7280)', fontSize: 13 }}>Loading…</div>
        )}
        {isError && (
          <div style={{ color: '#dc2626', fontSize: 13 }}>Failed to load metric history.</div>
        )}

        {data && (
          <>
            {/* Stats summary */}
            <div
              style={{
                display: 'grid',
                gridTemplateColumns: 'repeat(4, 1fr)',
                gap: 12,
              }}
            >
              {[
                { label: 'Min', value: data.stats.min },
                { label: 'Max', value: data.stats.max },
                { label: 'Avg', value: data.stats.avg },
                { label: 'Count', value: data.stats.count },
              ].map(({ label, value }) => (
                <div
                  key={label}
                  style={{
                    padding: 12,
                    borderRadius: 8,
                    border: '1px solid var(--ppt-border-default, #e5e7eb)',
                    textAlign: 'center',
                  }}
                >
                  <div
                    style={{
                      fontSize: 11,
                      color: 'var(--ppt-fg-muted, #6b7280)',
                      textTransform: 'uppercase',
                      marginBottom: 4,
                    }}
                  >
                    {label}
                  </div>
                  <div style={{ fontSize: 18, fontWeight: 700 }}>
                    {typeof value === 'number'
                      ? value.toLocaleString(undefined, { maximumFractionDigits: 2 })
                      : value}
                  </div>
                </div>
              ))}
            </div>

            {/* Data points table */}
            {data.data_points.length === 0 ? (
              <div style={{ color: 'var(--ppt-fg-muted, #6b7280)', fontSize: 13, padding: 16 }}>
                No data points in this time range.
              </div>
            ) : (
              <div style={{ overflowX: 'auto' }}>
                <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 13 }}>
                  <thead>
                    <tr>
                      <th
                        style={{
                          textAlign: 'left',
                          padding: '8px 12px',
                          borderBottom: '2px solid var(--ppt-border-default, #e5e7eb)',
                          color: 'var(--ppt-fg-muted, #6b7280)',
                          fontWeight: 600,
                        }}
                      >
                        Timestamp
                      </th>
                      <th
                        style={{
                          textAlign: 'right',
                          padding: '8px 12px',
                          borderBottom: '2px solid var(--ppt-border-default, #e5e7eb)',
                          color: 'var(--ppt-fg-muted, #6b7280)',
                          fontWeight: 600,
                        }}
                      >
                        Value
                      </th>
                    </tr>
                  </thead>
                  <tbody>
                    {data.data_points.map((dp, i) => (
                      <tr
                        key={`${dp.recorded_at}-${i}`}
                        style={{
                          borderBottom: '1px solid var(--ppt-border-default, #e5e7eb)',
                        }}
                      >
                        <td
                          style={{ padding: '6px 12px', color: 'var(--ppt-fg-secondary, #374151)' }}
                        >
                          {new Date(dp.recorded_at).toLocaleString()}
                        </td>
                        <td
                          style={{
                            padding: '6px 12px',
                            textAlign: 'right',
                            fontVariantNumeric: 'tabular-nums',
                            fontWeight: 500,
                          }}
                        >
                          {dp.value.toLocaleString(undefined, { maximumFractionDigits: 4 })}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Alerts table
// ---------------------------------------------------------------------------

interface AlertsTableProps {
  alerts: MetricAlert[];
  canAcknowledge: boolean;
  onAcknowledge: (id: string) => void;
  isAcknowledging: boolean;
}

function AlertsTable({ alerts, canAcknowledge, onAcknowledge, isAcknowledging }: AlertsTableProps) {
  if (alerts.length === 0) {
    return (
      <div style={{ color: 'var(--ppt-fg-muted, #6b7280)', fontSize: 13, padding: '12px 0' }}>
        No active alerts.
      </div>
    );
  }

  return (
    <div style={{ overflowX: 'auto' }}>
      <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 13 }}>
        <thead>
          <tr>
            {['Metric', 'Threshold type', 'Value', 'Triggered at', 'Actions'].map((h) => (
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
          {alerts.map((alert) => (
            <tr
              key={alert.id}
              style={{ borderBottom: '1px solid var(--ppt-border-default, #e5e7eb)' }}
            >
              <td style={{ padding: '8px 12px', fontWeight: 500 }}>{alert.metric_name}</td>
              <td style={{ padding: '8px 12px', textTransform: 'capitalize' }}>
                {alert.threshold_type}
              </td>
              <td style={{ padding: '8px 12px', fontVariantNumeric: 'tabular-nums' }}>
                {alert.value.toLocaleString(undefined, { maximumFractionDigits: 4 })}
              </td>
              <td style={{ padding: '8px 12px', color: 'var(--ppt-fg-muted, #6b7280)' }}>
                {new Date(alert.created_at).toLocaleString()}
              </td>
              <td style={{ padding: '8px 12px' }}>
                {alert.acknowledged_at ? (
                  <span
                    style={{
                      fontSize: 11,
                      color: 'var(--ppt-fg-muted, #6b7280)',
                      fontStyle: 'italic',
                    }}
                  >
                    Acknowledged
                  </span>
                ) : canAcknowledge ? (
                  <button
                    type="button"
                    disabled={isAcknowledging}
                    onClick={() => onAcknowledge(alert.id)}
                    style={{
                      padding: '3px 10px',
                      borderRadius: 6,
                      border: '1px solid var(--ppt-border-default, #e5e7eb)',
                      background: 'transparent',
                      cursor: isAcknowledging ? 'not-allowed' : 'pointer',
                      fontSize: 12,
                      fontWeight: 500,
                      opacity: isAcknowledging ? 0.5 : 1,
                    }}
                  >
                    Acknowledge
                  </button>
                ) : (
                  <span style={{ fontSize: 11, color: 'var(--ppt-fg-muted, #6b7280)' }}>—</span>
                )}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Thresholds table
// ---------------------------------------------------------------------------

interface ThresholdsTableProps {
  thresholds: MetricThreshold[];
  canEdit: boolean;
}

interface EditingThreshold {
  name: string;
  warning: string;
  critical: string;
}

function ThresholdsTable({ thresholds, canEdit }: ThresholdsTableProps) {
  const { showToast } = useToast();
  const [editing, setEditing] = useState<EditingThreshold | null>(null);
  const updateThreshold = useUpdateHealthThreshold();

  const handleSave = () => {
    if (!editing) return;
    const data: UpdateThresholdRequest = {};
    const w = Number.parseFloat(editing.warning);
    const c = Number.parseFloat(editing.critical);
    if (!Number.isNaN(w)) data.warning_threshold = w;
    if (!Number.isNaN(c)) data.critical_threshold = c;
    updateThreshold.mutate(
      { metricName: editing.name, data },
      {
        onSuccess: () => {
          showToast({
            type: 'success',
            title: 'Threshold updated',
            message: `${editing.name} thresholds saved.`,
          });
          setEditing(null);
        },
        onError: () => {
          showToast({
            type: 'error',
            title: 'Save failed',
            message: 'Could not update threshold.',
          });
        },
      }
    );
  };

  if (thresholds.length === 0) {
    return (
      <div style={{ color: 'var(--ppt-fg-muted, #6b7280)', fontSize: 13, padding: '12px 0' }}>
        No thresholds configured.
      </div>
    );
  }

  return (
    <>
      {editing && (
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
            aria-label={`Edit thresholds for ${editing.name}`}
            style={{
              width: '100%',
              maxWidth: 420,
              background: 'var(--ppt-bg-surface, #fff)',
              borderRadius: 12,
              padding: 24,
              display: 'flex',
              flexDirection: 'column',
              gap: 16,
              boxShadow: '0 10px 40px rgba(0,0,0,0.15)',
            }}
          >
            <h2 style={{ margin: 0, fontSize: 16, fontWeight: 600 }}>
              Edit thresholds: {editing.name}
            </h2>
            <label style={{ display: 'flex', flexDirection: 'column', gap: 4, fontSize: 13 }}>
              Warning threshold
              <input
                type="number"
                value={editing.warning}
                onChange={(e) => setEditing({ ...editing, warning: e.target.value })}
                style={{
                  padding: '6px 10px',
                  borderRadius: 6,
                  border: '1px solid var(--ppt-border-default, #e5e7eb)',
                  fontSize: 13,
                }}
              />
            </label>
            <label style={{ display: 'flex', flexDirection: 'column', gap: 4, fontSize: 13 }}>
              Critical threshold
              <input
                type="number"
                value={editing.critical}
                onChange={(e) => setEditing({ ...editing, critical: e.target.value })}
                style={{
                  padding: '6px 10px',
                  borderRadius: 6,
                  border: '1px solid var(--ppt-border-default, #e5e7eb)',
                  fontSize: 13,
                }}
              />
            </label>
            <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 10 }}>
              <button
                type="button"
                onClick={() => setEditing(null)}
                disabled={updateThreshold.isPending}
                style={{
                  padding: '7px 14px',
                  borderRadius: 8,
                  border: '1px solid var(--ppt-border-default, #e5e7eb)',
                  background: 'transparent',
                  cursor: 'pointer',
                  fontSize: 13,
                  fontWeight: 500,
                }}
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={handleSave}
                disabled={updateThreshold.isPending}
                style={{
                  padding: '7px 14px',
                  borderRadius: 8,
                  border: 'none',
                  background: 'var(--ppt-brand-600, #2563eb)',
                  color: '#fff',
                  cursor: updateThreshold.isPending ? 'not-allowed' : 'pointer',
                  fontSize: 13,
                  fontWeight: 500,
                  opacity: updateThreshold.isPending ? 0.5 : 1,
                }}
              >
                {updateThreshold.isPending ? 'Saving…' : 'Save'}
              </button>
            </div>
          </div>
        </div>
      )}
      <div style={{ overflowX: 'auto' }}>
        <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 13 }}>
          <thead>
            <tr>
              {['Metric', 'Warning', 'Critical', 'Active', ...(canEdit ? ['Actions'] : [])].map(
                (h) => (
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
                )
              )}
            </tr>
          </thead>
          <tbody>
            {thresholds.map((t) => (
              <tr
                key={t.id}
                style={{ borderBottom: '1px solid var(--ppt-border-default, #e5e7eb)' }}
              >
                <td style={{ padding: '8px 12px', fontWeight: 500 }}>{t.metric_name}</td>
                <td style={{ padding: '8px 12px', fontVariantNumeric: 'tabular-nums' }}>
                  {t.warning_threshold > 0
                    ? t.warning_threshold.toLocaleString(undefined, { maximumFractionDigits: 2 })
                    : '—'}
                </td>
                <td style={{ padding: '8px 12px', fontVariantNumeric: 'tabular-nums' }}>
                  {t.critical_threshold > 0
                    ? t.critical_threshold.toLocaleString(undefined, { maximumFractionDigits: 2 })
                    : '—'}
                </td>
                <td style={{ padding: '8px 12px' }}>
                  {t.is_active ? (
                    <span style={{ color: '#065f46', fontWeight: 600 }}>Yes</span>
                  ) : (
                    <span style={{ color: 'var(--ppt-fg-muted, #6b7280)' }}>No</span>
                  )}
                </td>
                {canEdit && (
                  <td style={{ padding: '8px 12px' }}>
                    <button
                      type="button"
                      onClick={() =>
                        setEditing({
                          name: t.metric_name,
                          warning: String(t.warning_threshold),
                          critical: String(t.critical_threshold),
                        })
                      }
                      style={{
                        padding: '3px 10px',
                        borderRadius: 6,
                        border: '1px solid var(--ppt-border-default, #e5e7eb)',
                        background: 'transparent',
                        cursor: 'pointer',
                        fontSize: 12,
                        fontWeight: 500,
                      }}
                    >
                      Edit
                    </button>
                  </td>
                )}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </>
  );
}

// ---------------------------------------------------------------------------
// Section heading helper
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

const PlatformHealthPage: React.FC = () => {
  const { t } = useTranslation();
  const { showToast } = useToast();

  const canAcknowledge = useCapability('site_settings_write');
  const canEditThresholds = useCapability('site_settings_write');

  const [drilledMetric, setDrilledMetric] = useState<string | null>(null);
  const [activeOnly, setActiveOnly] = useState(true);

  const { data: dashboard, isLoading, isError, refetch } = useHealthDashboard();

  const { data: allAlerts } = useHealthAlerts(activeOnly);

  const { mutate: acknowledgeAlert, isPending: isAcknowledging } = useAcknowledgeAlert();

  const displayedAlerts = allAlerts ?? dashboard?.alerts ?? [];

  return (
    <section style={{ padding: '24px 28px', maxWidth: 1200 }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <h1
          style={{
            fontSize: 20,
            fontWeight: 700,
            margin: 0,
            display: 'flex',
            alignItems: 'center',
            gap: 8,
          }}
        >
          {t('admin.health.title', 'Platform Health')}
          <HelpTooltip text={t('admin.health.helpTooltip')} />
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
          {isLoading ? 'Refreshing…' : 'Refresh'}
        </button>
      </div>

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
            'admin.health.loadError',
            'Failed to load health dashboard. Check your connection and try again.'
          )}
        </div>
      )}

      {/* Current metrics grid */}
      <SectionHeading>{t('admin.health.metricsTitle', 'Current Metrics')}</SectionHeading>
      {isLoading && (
        <div style={{ color: 'var(--ppt-fg-muted, #6b7280)', fontSize: 13 }}>
          {t('admin.common.loading', 'Loading…')}
        </div>
      )}
      {dashboard && (
        <div
          style={{
            display: 'grid',
            gridTemplateColumns: 'repeat(auto-fill, minmax(200px, 1fr))',
            gap: 12,
          }}
        >
          {dashboard.metrics.length === 0 ? (
            <span style={{ color: 'var(--ppt-fg-muted, #6b7280)', fontSize: 13 }}>
              {t('admin.health.noMetrics', 'No metrics recorded yet.')}
            </span>
          ) : (
            dashboard.metrics.map((m) => (
              <MetricCard key={m.metric_name} metric={m} onDrill={setDrilledMetric} />
            ))
          )}
        </div>
      )}

      {/* Active alerts */}
      <SectionHeading>{t('admin.health.alertsTitle', 'Alerts')}</SectionHeading>
      <div style={{ display: 'flex', gap: 10, marginBottom: 12 }}>
        {(['active', 'all'] as const).map((mode) => (
          <button
            key={mode}
            type="button"
            onClick={() => setActiveOnly(mode === 'active')}
            style={{
              padding: '4px 12px',
              borderRadius: 6,
              border: '1px solid var(--ppt-border-default, #e5e7eb)',
              background:
                (mode === 'active') === activeOnly
                  ? 'var(--ppt-brand-600, #2563eb)'
                  : 'transparent',
              color:
                (mode === 'active') === activeOnly ? '#fff' : 'var(--ppt-fg-secondary, #374151)',
              cursor: 'pointer',
              fontSize: 12,
              fontWeight: 500,
            }}
          >
            {mode === 'active'
              ? t('admin.health.activeOnly', 'Active only')
              : t('admin.health.allAlerts', 'All alerts')}
          </button>
        ))}
      </div>
      <AlertsTable
        alerts={displayedAlerts}
        canAcknowledge={canAcknowledge}
        onAcknowledge={(id) =>
          acknowledgeAlert(id, {
            onSuccess: () =>
              showToast({
                type: 'success',
                title: 'Alert acknowledged',
                message: 'The alert has been marked as acknowledged.',
              }),
            onError: () =>
              showToast({
                type: 'error',
                title: 'Acknowledge failed',
                message: 'Could not acknowledge the alert.',
              }),
          })
        }
        isAcknowledging={isAcknowledging}
      />

      {/* Thresholds */}
      <SectionHeading>{t('admin.health.thresholdsTitle', 'Metric Thresholds')}</SectionHeading>
      {dashboard ? (
        <ThresholdsTable thresholds={dashboard.thresholds} canEdit={canEditThresholds} />
      ) : (
        !isLoading && <div style={{ color: 'var(--ppt-fg-muted, #6b7280)', fontSize: 13 }}>—</div>
      )}

      {/* Metric history drill-down panel */}
      {drilledMetric && (
        <MetricHistoryPanel metricName={drilledMetric} onClose={() => setDrilledMetric(null)} />
      )}
    </section>
  );
};

export default PlatformHealthPage;
