/**
 * Audit Log — `/ops/audit`
 *
 * Design impl §D. Two-panel layout: sticky 220px filter rail (left) +
 * scrollable table area (right). All filter state is synced to URL query
 * string. Rows expand inline (no modal). Cursor-based "Load more" pagination.
 *
 * Gate: `audit_read` — enforced by ProtectedRoute in App.tsx.
 */

import { CAPABILITIES } from '@ppt/admin-ui';
import { useQuery } from '@tanstack/react-query';
import { type FC, Fragment, useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useSearchParams } from 'react-router-dom';
import { useAdminAuth } from '../auth/AdminAuthContext';
import { useToast } from '../components/Toast';
import { HelpTooltip } from '../features/help';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type AuditRow = {
  id: string;
  occurred_at: string;
  actor_email?: string;
  actor_user_id?: string;
  capability?: string;
  target_kind?: 'user' | 'tenant' | 'capability';
  target_slug?: string;
  outcome?: 'success' | 'mfa_failed' | 'denied';
  reason?: string;
  ip?: string;
  user_agent?: string;
  mfa_recent_at_call?: string;
  request?: { path?: string; headers?: Record<string, string>; body?: string };
  principal_kind?: string;
};

type AuditResponse = {
  items: AuditRow[];
  next_cursor?: string;
};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const HIGH_RISK_CAPABILITIES = new Set([
  'tenant_purge',
  'principal_kind_escalate',
  'grant_principal_kind_escalate',
  'tenant_restore',
]);

const DATE_PRESETS = [
  { label: '15m', value: '15m' },
  { label: '1h', value: '1h' },
  { label: '24h', value: '24h' },
  { label: '7d', value: '7d' },
  { label: '30d', value: '30d' },
] as const;

type DatePreset = (typeof DATE_PRESETS)[number]['value'];

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatOccurredAt(iso: string): string {
  try {
    return new Intl.DateTimeFormat('en-GB', {
      dateStyle: 'short',
      timeStyle: 'medium',
    }).format(new Date(iso));
  } catch {
    return iso;
  }
}

function truncate(s: string | undefined, max: number): string {
  if (!s) return '—';
  return s.length > max ? `${s.slice(0, max)}…` : s;
}

function isHighSeverity(capability?: string): boolean {
  return capability ? HIGH_RISK_CAPABILITIES.has(capability) : false;
}

function buildQueryString(
  filters: {
    actor: string;
    target: string;
    capabilities: string[];
    severity: string;
    outcome: string;
    dateRange: string;
  },
  cursor?: string
): string {
  const params = new URLSearchParams();
  if (filters.actor) params.set('actor', filters.actor);
  if (filters.target) params.set('target', filters.target);
  if (filters.capabilities.length) params.set('capability', filters.capabilities.join(','));
  if (filters.severity !== 'all') params.set('severity', filters.severity);
  if (filters.outcome !== 'all') params.set('outcome', filters.outcome);
  if (filters.dateRange) params.set('range', filters.dateRange);
  if (cursor) params.set('after', cursor);
  return params.toString();
}

function parseMfaRemaining(row: AuditRow): string {
  if (!row.mfa_recent_at_call) return 'Not active at call time';
  const at = new Date(row.mfa_recent_at_call);
  const mfaWindow = 15 * 60 * 1000; // 15 min assumed window
  const elapsed = Date.now() - at.getTime();
  const remaining = mfaWindow - elapsed;
  if (remaining <= 0) return 'Active at call time (expired since)';
  const mins = Math.floor(remaining / 60000);
  return `Active at call time · ${mins} min remaining`;
}

// ---------------------------------------------------------------------------
// Sub-components
// ---------------------------------------------------------------------------

/** Colored pill for capability name */
function CapabilityPill({ cap }: { cap?: string }) {
  if (!cap) return <span style={{ color: 'var(--ppt-fg-muted)' }}>—</span>;
  const high = isHighSeverity(cap);
  return (
    <span
      style={{
        display: 'inline-block',
        padding: '2px 8px',
        borderRadius: 'var(--ppt-radius-full, 9999px)',
        fontSize: '0.75rem',
        fontWeight: 500,
        background: high ? 'var(--ppt-danger-100, #fee2e2)' : 'var(--ppt-brand-100, #eff6ff)',
        color: high ? 'var(--ppt-danger-800, #991b1b)' : 'var(--ppt-brand-800, #1e40af)',
        fontFamily: 'monospace',
        whiteSpace: 'nowrap',
      }}
    >
      {cap}
    </span>
  );
}

/** Colored pill for outcome */
function OutcomePill({ outcome }: { outcome?: string }) {
  if (!outcome) return <span style={{ color: 'var(--ppt-fg-muted)' }}>—</span>;
  const colors: Record<string, { bg: string; fg: string }> = {
    success: { bg: 'var(--ppt-success-100, #dcfce7)', fg: 'var(--ppt-success-800, #166534)' },
    mfa_failed: { bg: 'var(--ppt-warning-100, #fef9c3)', fg: 'var(--ppt-warning-800, #854d0e)' },
    denied: { bg: 'var(--ppt-danger-100, #fee2e2)', fg: 'var(--ppt-danger-800, #991b1b)' },
  };
  const c = colors[outcome] ?? {
    bg: 'var(--ppt-bg-subtle, #f9fafb)',
    fg: 'var(--ppt-fg-muted, #6b7280)',
  };
  return (
    <span
      style={{
        display: 'inline-block',
        padding: '2px 8px',
        borderRadius: 'var(--ppt-radius-full, 9999px)',
        fontSize: '0.75rem',
        fontWeight: 500,
        background: c.bg,
        color: c.fg,
        whiteSpace: 'nowrap',
      }}
    >
      {outcome}
    </span>
  );
}

/** Red/amber/blue severity dot */
function SeverityDot({ capability }: { capability?: string }) {
  const high = isHighSeverity(capability);
  return (
    <span
      style={{
        display: 'inline-block',
        width: 8,
        height: 8,
        borderRadius: '50%',
        background: high ? 'var(--ppt-danger-500, #ef4444)' : 'var(--ppt-brand-400, #60a5fa)',
        flexShrink: 0,
      }}
      title={high ? 'High severity' : 'Normal severity'}
    />
  );
}

/** 8 skeleton rows for loading state */
function SkeletonRows() {
  return (
    <>
      {Array.from({ length: 8 }).map((_, i) => (
        <tr key={`skel-row-${i}`} style={{ opacity: 1 - i * 0.08 }}>
          {Array.from({ length: 6 }).map((__, j) => (
            <td key={`skel-cell-${i}-${j}`} style={{ padding: '10px 12px' }}>
              <span
                style={{
                  display: 'block',
                  height: 14,
                  borderRadius: 4,
                  background: 'var(--ppt-bg-subtle, #f3f4f6)',
                  width: j === 0 ? '80%' : j === 1 ? '70%' : '60%',
                  animation: 'ppt-skeleton-pulse 1.4s ease-in-out infinite',
                }}
              />
            </td>
          ))}
        </tr>
      ))}
    </>
  );
}

/** Inline expanded row — full-width, spans all columns */
function ExpandedRow({ row, onClose }: { row: AuditRow; onClose: () => void }) {
  const handleCopyCurl = useCallback(() => {
    const path = row.request?.path ?? '/unknown';
    const headers = row.request?.headers ?? {};
    // `body` is already a string per the AuditRow type — do NOT re-stringify
    // (extra JSON.stringify wraps it in quotes + escapes, breaking cURL replay).
    const body = row.request?.body ? row.request.body.slice(0, 4096).replace(/'/g, "'\\''") : '';
    const headerFlags = Object.entries(headers)
      .map(([k, v]) => `  -H '${k}: ${v}'`)
      .join(' \\\n');
    const bodyFlag = body ? ` \\\n  -d '${body}'` : '';
    const curl = `curl -X POST 'https://admin.rlt.sk${path}' \\\n${headerFlags}${bodyFlag}`;
    navigator.clipboard.writeText(curl).catch(() => {
      // fallback — silent if clipboard API unavailable
    });
  }, [row]);

  return (
    <tr
      style={{
        background: 'var(--ppt-accent-soft-bg, #eff6ff)',
        borderBottom: '1px solid var(--ppt-border-default, #e5e7eb)',
      }}
    >
      <td colSpan={6} style={{ padding: '16px 24px' }}>
        <div
          style={{
            display: 'grid',
            gridTemplateColumns: '1fr 1fr',
            gap: 24,
          }}
        >
          {/* Left half */}
          <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
            <div>
              <p
                style={{
                  fontSize: '0.75rem',
                  fontWeight: 600,
                  color: 'var(--ppt-fg-muted)',
                  textTransform: 'uppercase',
                  letterSpacing: '0.05em',
                  marginBottom: 4,
                }}
              >
                Audit reason
              </p>
              <pre
                style={{
                  fontFamily: 'monospace',
                  fontSize: '0.8125rem',
                  whiteSpace: 'pre-wrap',
                  wordBreak: 'break-word',
                  color: 'var(--ppt-fg-primary)',
                  margin: 0,
                }}
              >
                {row.reason ?? '(no reason recorded)'}
              </pre>
            </div>
            {row.request && (
              <div>
                <p
                  style={{
                    fontSize: '0.75rem',
                    fontWeight: 600,
                    color: 'var(--ppt-fg-muted)',
                    textTransform: 'uppercase',
                    letterSpacing: '0.05em',
                    marginBottom: 4,
                  }}
                >
                  Request envelope
                </p>
                <div
                  style={{
                    fontFamily: 'monospace',
                    fontSize: '0.8125rem',
                    background: 'var(--ppt-bg-app, #f9fafb)',
                    border: '1px solid var(--ppt-border-default, #e5e7eb)',
                    borderRadius: 6,
                    padding: '10px 12px',
                    color: 'var(--ppt-fg-secondary)',
                  }}
                >
                  <div style={{ fontWeight: 600 }}>POST {row.request.path ?? '/unknown'}</div>
                  {Object.entries(row.request.headers ?? {}).map(([k, v]) => (
                    <div key={k}>
                      <span style={{ color: 'var(--ppt-brand-600, #2563eb)' }}>{k}:</span> {v}
                    </div>
                  ))}
                  {row.request.body && (
                    <div
                      style={{
                        marginTop: 6,
                        whiteSpace: 'pre-wrap',
                        wordBreak: 'break-all',
                        maxHeight: 120,
                        overflow: 'auto',
                        opacity: 0.85,
                      }}
                    >
                      {row.request.body.slice(0, 4096)}
                    </div>
                  )}
                </div>
              </div>
            )}
          </div>

          {/* Right half */}
          <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
            <div>
              <p
                style={{
                  fontSize: '0.75rem',
                  fontWeight: 600,
                  color: 'var(--ppt-fg-muted)',
                  textTransform: 'uppercase',
                  letterSpacing: '0.05em',
                  marginBottom: 4,
                }}
              >
                Principal
              </p>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <span style={{ fontSize: '0.875rem', color: 'var(--ppt-fg-primary)' }}>
                  {row.actor_email ?? row.actor_user_id ?? '—'}
                </span>
                {row.principal_kind && (
                  <span
                    style={{
                      padding: '1px 6px',
                      borderRadius: 4,
                      fontSize: '0.6875rem',
                      fontWeight: 600,
                      background: 'var(--ppt-brand-100, #eff6ff)',
                      color: 'var(--ppt-brand-700, #1d4ed8)',
                      textTransform: 'uppercase',
                      fontFamily: 'monospace',
                    }}
                  >
                    {row.principal_kind}
                  </span>
                )}
              </div>
            </div>

            {(row.ip ?? row.user_agent) && (
              <div>
                <p
                  style={{
                    fontSize: '0.75rem',
                    fontWeight: 600,
                    color: 'var(--ppt-fg-muted)',
                    textTransform: 'uppercase',
                    letterSpacing: '0.05em',
                    marginBottom: 4,
                  }}
                >
                  Network / Client
                </p>
                <div
                  style={{
                    fontFamily: 'monospace',
                    fontSize: '0.8125rem',
                    color: 'var(--ppt-fg-secondary)',
                  }}
                >
                  {row.ip && <div>IP: {row.ip}</div>}
                  {row.user_agent && (
                    <div style={{ wordBreak: 'break-all' }}>UA: {row.user_agent}</div>
                  )}
                </div>
              </div>
            )}

            <div>
              <p
                style={{
                  fontSize: '0.75rem',
                  fontWeight: 600,
                  color: 'var(--ppt-fg-muted)',
                  textTransform: 'uppercase',
                  letterSpacing: '0.05em',
                  marginBottom: 4,
                }}
              >
                MFA state at call time
              </p>
              <span
                style={{
                  fontSize: '0.875rem',
                  color: row.mfa_recent_at_call
                    ? 'var(--ppt-success-700, #15803d)'
                    : 'var(--ppt-fg-muted)',
                }}
              >
                {parseMfaRemaining(row)}
              </span>
            </div>

            <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap', marginTop: 4 }}>
              <button
                type="button"
                onClick={handleCopyCurl}
                style={{
                  padding: '5px 12px',
                  fontSize: '0.8125rem',
                  border: '1px solid var(--ppt-border-default, #e5e7eb)',
                  borderRadius: 6,
                  background: 'var(--ppt-bg-surface, #fff)',
                  color: 'var(--ppt-fg-primary)',
                  cursor: 'pointer',
                  fontFamily: 'inherit',
                }}
              >
                Copy as cURL
              </button>
              <button
                type="button"
                onClick={() => {
                  /* TODO: view related events */
                }}
                style={{
                  padding: '5px 12px',
                  fontSize: '0.8125rem',
                  border: '1px solid var(--ppt-border-default, #e5e7eb)',
                  borderRadius: 6,
                  background: 'var(--ppt-bg-surface, #fff)',
                  color: 'var(--ppt-fg-primary)',
                  cursor: 'pointer',
                  fontFamily: 'inherit',
                  opacity: 0.6,
                }}
                title="TODO — not yet implemented"
              >
                View related
              </button>
              <button
                type="button"
                onClick={onClose}
                style={{
                  marginLeft: 'auto',
                  padding: '5px 12px',
                  fontSize: '0.8125rem',
                  border: 'none',
                  borderRadius: 6,
                  background: 'transparent',
                  color: 'var(--ppt-fg-muted)',
                  cursor: 'pointer',
                  fontFamily: 'inherit',
                }}
              >
                Close ✕
              </button>
            </div>
          </div>
        </div>
      </td>
    </tr>
  );
}

// ---------------------------------------------------------------------------
// Filter rail
// ---------------------------------------------------------------------------

type Filters = {
  actor: string;
  target: string;
  capabilities: string[];
  severity: 'all' | 'high' | 'normal';
  outcome: 'all' | 'success' | 'mfa_failed' | 'denied';
  dateRange: DatePreset | '';
};

const FILTER_DEFAULTS: Filters = {
  actor: '',
  target: '',
  capabilities: [],
  severity: 'all',
  outcome: 'all',
  dateRange: '',
};

function readFiltersFromParams(params: URLSearchParams): Filters {
  const caps = params.get('capability');
  return {
    actor: params.get('actor') ?? '',
    target: params.get('target') ?? '',
    capabilities: caps ? caps.split(',').filter(Boolean) : [],
    severity: (params.get('severity') as Filters['severity']) ?? 'all',
    outcome: (params.get('outcome') as Filters['outcome']) ?? 'all',
    dateRange: (params.get('range') as DatePreset) ?? '',
  };
}

function hasActiveFilters(f: Filters): boolean {
  return (
    f.actor !== '' ||
    f.target !== '' ||
    f.capabilities.length > 0 ||
    f.severity !== 'all' ||
    f.outcome !== 'all' ||
    f.dateRange !== ''
  );
}

interface FilterRailProps {
  filters: Filters;
  onChange: (f: Filters) => void;
}

function FilterRail({ filters, onChange }: FilterRailProps) {
  // Debounce actor + target text inputs
  const actorTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const targetTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const [actorDraft, setActorDraft] = useState(filters.actor);
  const [targetDraft, setTargetDraft] = useState(filters.target);

  // Sync draft from external filter changes (e.g. clear-all)
  useEffect(() => {
    setActorDraft(filters.actor);
  }, [filters.actor]);
  useEffect(() => {
    setTargetDraft(filters.target);
  }, [filters.target]);

  const handleActorChange = (v: string) => {
    setActorDraft(v);
    if (actorTimer.current) clearTimeout(actorTimer.current);
    actorTimer.current = setTimeout(() => onChange({ ...filters, actor: v }), 250);
  };

  const handleTargetChange = (v: string) => {
    setTargetDraft(v);
    if (targetTimer.current) clearTimeout(targetTimer.current);
    targetTimer.current = setTimeout(() => onChange({ ...filters, target: v }), 250);
  };

  const toggleCapability = (cap: string) => {
    const next = filters.capabilities.includes(cap)
      ? filters.capabilities.filter((c) => c !== cap)
      : [...filters.capabilities, cap];
    onChange({ ...filters, capabilities: next });
  };

  const labelStyle: React.CSSProperties = {
    display: 'block',
    fontSize: '0.6875rem',
    fontWeight: 600,
    letterSpacing: '0.06em',
    textTransform: 'uppercase',
    color: 'var(--ppt-fg-muted)',
    marginBottom: 4,
  };

  const inputStyle: React.CSSProperties = {
    width: '100%',
    boxSizing: 'border-box',
    padding: '6px 10px',
    fontSize: '0.8125rem',
    border: '1px solid var(--ppt-border-default, #e5e7eb)',
    borderRadius: 6,
    background: 'var(--ppt-bg-surface, #fff)',
    color: 'var(--ppt-fg-primary)',
    outline: 'none',
    fontFamily: 'inherit',
  };

  const segmentedBase: React.CSSProperties = {
    display: 'inline-flex',
    border: '1px solid var(--ppt-border-default, #e5e7eb)',
    borderRadius: 6,
    overflow: 'hidden',
    width: '100%',
  };

  const segBtn = (active: boolean): React.CSSProperties => ({
    flex: 1,
    padding: '5px 0',
    fontSize: '0.8125rem',
    border: 'none',
    cursor: 'pointer',
    fontFamily: 'inherit',
    background: active ? 'var(--ppt-brand-600, #2563eb)' : 'var(--ppt-bg-surface, #fff)',
    color: active ? '#fff' : 'var(--ppt-fg-secondary)',
    fontWeight: active ? 600 : 400,
    transition: 'background 0.15s',
  });

  return (
    <aside
      style={{
        width: 220,
        flexShrink: 0,
        position: 'sticky',
        top: 0,
        height: '100vh',
        overflowY: 'auto',
        borderRight: '1px solid var(--ppt-border-default, #e5e7eb)',
        padding: 16,
        boxSizing: 'border-box',
        display: 'flex',
        flexDirection: 'column',
        gap: 16,
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <h3
          style={{
            fontSize: '0.875rem',
            fontWeight: 700,
            color: 'var(--ppt-fg-primary)',
            margin: 0,
          }}
        >
          Filters
        </h3>
        {hasActiveFilters(filters) && (
          <button
            type="button"
            onClick={() => onChange(FILTER_DEFAULTS)}
            style={{
              fontSize: '0.75rem',
              color: 'var(--ppt-brand-600, #2563eb)',
              background: 'none',
              border: 'none',
              cursor: 'pointer',
              padding: 0,
              fontFamily: 'inherit',
            }}
          >
            Clear all
          </button>
        )}
      </div>

      {/* Actor */}
      <div>
        <label style={labelStyle} htmlFor="audit-filter-actor">
          Actor
        </label>
        <input
          id="audit-filter-actor"
          type="text"
          value={actorDraft}
          onChange={(e) => handleActorChange(e.target.value)}
          placeholder="email or user id"
          style={inputStyle}
        />
      </div>

      {/* Target */}
      <div>
        <label style={labelStyle} htmlFor="audit-filter-target">
          Target
        </label>
        <input
          id="audit-filter-target"
          type="text"
          value={targetDraft}
          onChange={(e) => handleTargetChange(e.target.value)}
          placeholder="user / tenant / capability"
          style={inputStyle}
        />
      </div>

      {/* Capability multi-select pills */}
      <div>
        <span style={labelStyle}>Capability</span>
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: 4 }}>
          {CAPABILITIES.map((cap) => {
            const selected = filters.capabilities.includes(cap);
            return (
              <button
                type="button"
                key={cap}
                onClick={() => toggleCapability(cap)}
                style={{
                  padding: '2px 7px',
                  fontSize: '0.6875rem',
                  fontFamily: 'monospace',
                  borderRadius: 4,
                  border: '1px solid',
                  cursor: 'pointer',
                  borderColor: selected
                    ? 'var(--ppt-brand-300, #93c5fd)'
                    : 'var(--ppt-border-default, #e5e7eb)',
                  background: selected
                    ? 'var(--ppt-brand-100, #eff6ff)'
                    : 'var(--ppt-bg-surface, #fff)',
                  color: selected ? 'var(--ppt-brand-800, #1e40af)' : 'var(--ppt-fg-secondary)',
                  whiteSpace: 'nowrap',
                }}
              >
                {cap}
              </button>
            );
          })}
        </div>
      </div>

      {/* Severity segmented control */}
      <div>
        <span style={labelStyle}>Severity</span>
        <div style={segmentedBase}>
          {(['all', 'high', 'normal'] as const).map((v) => (
            <button
              type="button"
              key={v}
              onClick={() => onChange({ ...filters, severity: v })}
              style={segBtn(filters.severity === v)}
            >
              {v === 'all' ? 'All' : v.charAt(0).toUpperCase() + v.slice(1)}
            </button>
          ))}
        </div>
      </div>

      {/* Outcome segmented control */}
      <div>
        <span style={labelStyle}>Outcome</span>
        <div style={{ ...segmentedBase, flexDirection: 'column' }}>
          {(['all', 'success', 'mfa_failed', 'denied'] as const).map((v) => (
            <button
              type="button"
              key={v}
              onClick={() => onChange({ ...filters, outcome: v })}
              style={{
                ...segBtn(filters.outcome === v),
                borderBottom:
                  v !== 'denied' ? '1px solid var(--ppt-border-default, #e5e7eb)' : 'none',
              }}
            >
              {v === 'all' ? 'All' : v}
            </button>
          ))}
        </div>
      </div>

      {/* Date range presets */}
      <div>
        <span style={labelStyle}>Date range</span>
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: 4 }}>
          {DATE_PRESETS.map(({ label, value }) => {
            const active = filters.dateRange === value;
            return (
              <button
                type="button"
                key={value}
                onClick={() => onChange({ ...filters, dateRange: active ? '' : value })}
                style={{
                  padding: '3px 10px',
                  fontSize: '0.8125rem',
                  borderRadius: 6,
                  border: '1px solid',
                  cursor: 'pointer',
                  fontFamily: 'inherit',
                  borderColor: active
                    ? 'var(--ppt-brand-400, #60a5fa)'
                    : 'var(--ppt-border-default, #e5e7eb)',
                  background: active
                    ? 'var(--ppt-brand-600, #2563eb)'
                    : 'var(--ppt-bg-surface, #fff)',
                  color: active ? '#fff' : 'var(--ppt-fg-secondary)',
                  fontWeight: active ? 600 : 400,
                }}
              >
                {label}
              </button>
            );
          })}
        </div>
      </div>
    </aside>
  );
}

// ---------------------------------------------------------------------------
// Main page component
// ---------------------------------------------------------------------------

const AuditPage: FC = () => {
  const { t } = useTranslation();
  const [searchParams, setSearchParams] = useSearchParams();
  const { showToast } = useToast();
  const { token } = useAdminAuth();

  // Read filter state from URL on mount
  const [filters, setFilters] = useState<Filters>(() => readFiltersFromParams(searchParams));

  // Cursor for "load more"
  const [cursor, setCursor] = useState<string | undefined>(searchParams.get('after') ?? undefined);
  // Accumulated rows across pages
  const [allRows, setAllRows] = useState<AuditRow[]>([]);

  // Expanded row ID
  const [expandedId, setExpandedId] = useState<string | null>(null);

  // Sync filters -> URL
  useEffect(() => {
    const next = new URLSearchParams();
    if (filters.actor) next.set('actor', filters.actor);
    if (filters.target) next.set('target', filters.target);
    if (filters.capabilities.length) next.set('capability', filters.capabilities.join(','));
    if (filters.severity !== 'all') next.set('severity', filters.severity);
    if (filters.outcome !== 'all') next.set('outcome', filters.outcome);
    if (filters.dateRange) next.set('range', filters.dateRange);
    if (cursor) next.set('after', cursor);
    setSearchParams(next, { replace: true });
  }, [filters, cursor, setSearchParams]);

  // When filters change, reset accumulated rows + cursor
  const handleFiltersChange = useCallback((f: Filters) => {
    setFilters(f);
    setCursor(undefined);
    setAllRows([]);
    setExpandedId(null);
  }, []);

  const qs = buildQueryString(filters, cursor);

  const query = useQuery<AuditResponse>({
    queryKey: ['admin', 'audit', filters, cursor],
    queryFn: async () => {
      const res = await fetch(`/api/v1/admin/audit${qs ? `?${qs}` : ''}`, {
        credentials: 'include',
        headers: token ? { Authorization: `Bearer ${token}` } : {},
      });
      if (!res.ok) {
        throw new Error(`HTTP ${res.status}`);
      }
      return res.json() as Promise<AuditResponse>;
    },
    staleTime: 30_000,
  });

  // Accumulate rows from successive queries
  useEffect(() => {
    if (query.data?.items) {
      if (!cursor) {
        // First page (or filter reset)
        setAllRows(query.data.items);
      } else {
        setAllRows((prev) => {
          // Deduplicate by id
          const ids = new Set(prev.map((r) => r.id));
          const fresh = query.data!.items.filter((r) => !ids.has(r.id));
          return [...prev, ...fresh];
        });
      }
    }
  }, [query.data, cursor]);

  // Client-side severity filter (server may not support it)
  const visibleRows = allRows.filter((row) => {
    if (filters.severity === 'high') return isHighSeverity(row.capability);
    if (filters.severity === 'normal') return !isHighSeverity(row.capability);
    return true;
  });

  const handleExportCsv = async () => {
    const csvQs = buildQueryString(filters);
    const url = `/api/v1/admin/audit/csv${csvQs ? `?${csvQs}` : ''}`;
    try {
      const resp = await fetch(url, {
        credentials: 'include',
        headers: token ? { Authorization: `Bearer ${token}` } : {},
      });
      if (!resp.ok) {
        showToast({
          type: 'error',
          title: `Export failed (${resp.status})`,
        });
        return;
      }
      const blob = await resp.blob();
      const dlUrl = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = dlUrl;
      // Match server's Content-Disposition filename format: YYYYMMDD (no dashes)
      const today = new Date().toISOString().slice(0, 10).replace(/-/g, '');
      a.download = `audit-${today}.csv`;
      document.body.appendChild(a);
      a.click();
      a.remove();
      // Defer revoke so the browser has a tick to start the download before
      // the object URL is invalidated.
      setTimeout(() => URL.revokeObjectURL(dlUrl), 100);
    } catch (err) {
      showToast({
        type: 'error',
        title: 'Export failed',
        message: err instanceof Error ? err.message : String(err),
      });
    }
  };

  const toggleRow = (id: string) => {
    setExpandedId((prev) => (prev === id ? null : id));
  };

  const nextCursor = query.data?.next_cursor;
  const isFirstLoad = query.isLoading && allRows.length === 0;

  return (
    <>
      {/* Skeleton animation keyframe — injected once */}
      <style>{`
        @keyframes ppt-skeleton-pulse {
          0%, 100% { opacity: 1; }
          50% { opacity: 0.4; }
        }
      `}</style>

      <div
        style={{
          display: 'flex',
          alignItems: 'flex-start',
          minHeight: '100vh',
          background: 'var(--ppt-bg-app, #f9fafb)',
        }}
      >
        {/* Left filter rail */}
        <FilterRail filters={filters} onChange={handleFiltersChange} />

        {/* Right content */}
        <div
          style={{
            flex: 1,
            minWidth: 0,
            display: 'flex',
            flexDirection: 'column',
          }}
        >
          {/* Toolbar */}
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              padding: '16px 24px',
              borderBottom: '1px solid var(--ppt-border-default, #e5e7eb)',
              background: 'var(--ppt-bg-surface, #fff)',
              position: 'sticky',
              top: 0,
              zIndex: 10,
            }}
          >
            <h1
              style={{
                fontSize: '1.25rem',
                fontWeight: 700,
                color: 'var(--ppt-fg-primary)',
                margin: 0,
                display: 'flex',
                alignItems: 'center',
                gap: 8,
              }}
            >
              Audit log
              <HelpTooltip text={t('admin.audit.helpTooltip')} />
            </h1>
            <button
              type="button"
              onClick={() => void handleExportCsv()}
              style={{
                padding: '6px 14px',
                fontSize: '0.875rem',
                border: '1px solid var(--ppt-border-default, #e5e7eb)',
                borderRadius: 6,
                background: 'transparent',
                color: 'var(--ppt-fg-secondary)',
                cursor: 'pointer',
                fontFamily: 'inherit',
              }}
            >
              Export CSV
            </button>
          </div>

          {/* Error banner */}
          {query.isError && (
            <div
              role="alert"
              style={{
                margin: '12px 24px 0',
                padding: '10px 14px',
                borderRadius: 6,
                background: 'var(--ppt-danger-50, #fef2f2)',
                border: '1px solid var(--ppt-danger-200, #fecaca)',
                color: 'var(--ppt-danger-800, #991b1b)',
                display: 'flex',
                alignItems: 'center',
                gap: 12,
                fontSize: '0.875rem',
              }}
            >
              <span>Unable to load audit log. Try again.</span>
              <button
                type="button"
                onClick={() => query.refetch()}
                style={{
                  padding: '3px 10px',
                  border: '1px solid var(--ppt-danger-400, #f87171)',
                  borderRadius: 4,
                  background: 'transparent',
                  color: 'var(--ppt-danger-800, #991b1b)',
                  cursor: 'pointer',
                  fontSize: '0.8125rem',
                  fontFamily: 'inherit',
                }}
              >
                Retry
              </button>
            </div>
          )}

          {/* Table */}
          <div style={{ overflowX: 'auto', padding: '0 24px 24px' }}>
            <table
              style={{
                width: '100%',
                borderCollapse: 'collapse',
                fontSize: '0.875rem',
                marginTop: 12,
              }}
            >
              <thead>
                <tr
                  style={{
                    borderBottom: '2px solid var(--ppt-border-default, #e5e7eb)',
                    textAlign: 'left',
                  }}
                >
                  {['When', 'Actor', 'Capability', 'Target', 'Outcome', 'Reason'].map((h) => (
                    <th
                      key={h}
                      style={{
                        padding: '8px 12px',
                        fontSize: '0.75rem',
                        fontWeight: 600,
                        color: 'var(--ppt-fg-muted)',
                        textTransform: 'uppercase',
                        letterSpacing: '0.05em',
                        whiteSpace: 'nowrap',
                      }}
                    >
                      {h}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {isFirstLoad ? (
                  <SkeletonRows />
                ) : visibleRows.length === 0 && !query.isLoading ? (
                  <tr>
                    <td
                      colSpan={6}
                      style={{
                        padding: '40px 12px',
                        textAlign: 'center',
                        color: 'var(--ppt-fg-muted)',
                        fontSize: '0.875rem',
                      }}
                    >
                      No audit events match this filter
                    </td>
                  </tr>
                ) : (
                  visibleRows.map((row) => (
                    <Fragment key={row.id}>
                      <tr
                        onClick={() => toggleRow(row.id)}
                        style={{
                          borderBottom: '1px solid var(--ppt-border-default, #e5e7eb)',
                          background:
                            expandedId === row.id
                              ? 'var(--ppt-accent-soft-bg, #eff6ff)'
                              : 'transparent',
                          cursor: 'pointer',
                          transition: 'background 0.1s',
                        }}
                        onMouseEnter={(e) => {
                          if (expandedId !== row.id) {
                            (e.currentTarget as HTMLTableRowElement).style.background =
                              'var(--ppt-bg-subtle, #f9fafb)';
                          }
                        }}
                        onMouseLeave={(e) => {
                          if (expandedId !== row.id) {
                            (e.currentTarget as HTMLTableRowElement).style.background =
                              'transparent';
                          }
                        }}
                      >
                        <td
                          style={{
                            padding: '10px 12px',
                            whiteSpace: 'nowrap',
                            verticalAlign: 'middle',
                          }}
                        >
                          <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                            <SeverityDot capability={row.capability} />
                            <time
                              dateTime={row.occurred_at}
                              style={{ fontFamily: 'monospace', fontSize: '0.8125rem' }}
                            >
                              {formatOccurredAt(row.occurred_at)}
                            </time>
                          </div>
                        </td>
                        <td
                          style={{
                            padding: '10px 12px',
                            color: 'var(--ppt-fg-secondary)',
                            maxWidth: 200,
                            overflow: 'hidden',
                            textOverflow: 'ellipsis',
                            whiteSpace: 'nowrap',
                          }}
                        >
                          {row.actor_email ?? row.actor_user_id ?? '—'}
                        </td>
                        <td style={{ padding: '10px 12px' }}>
                          <CapabilityPill cap={row.capability} />
                        </td>
                        <td
                          style={{
                            padding: '10px 12px',
                            fontFamily: 'monospace',
                            fontSize: '0.8125rem',
                            color: 'var(--ppt-fg-secondary)',
                          }}
                        >
                          {row.target_slug ? `${row.target_kind ?? ''}/${row.target_slug}` : '—'}
                        </td>
                        <td style={{ padding: '10px 12px' }}>
                          <OutcomePill outcome={row.outcome} />
                        </td>
                        <td
                          style={{
                            padding: '10px 12px',
                            color: 'var(--ppt-fg-muted)',
                            maxWidth: 240,
                            overflow: 'hidden',
                            textOverflow: 'ellipsis',
                            whiteSpace: 'nowrap',
                          }}
                          title={row.reason}
                        >
                          {truncate(row.reason, 60)}
                        </td>
                      </tr>
                      {expandedId === row.id && (
                        <ExpandedRow row={row} onClose={() => setExpandedId(null)} />
                      )}
                    </Fragment>
                  ))
                )}
              </tbody>
            </table>

            {/* Load more */}
            {nextCursor && !query.isLoading && (
              <div style={{ display: 'flex', justifyContent: 'center', marginTop: 16 }}>
                <button
                  type="button"
                  onClick={() => setCursor(nextCursor)}
                  style={{
                    padding: '8px 24px',
                    fontSize: '0.875rem',
                    border: '1px solid var(--ppt-border-default, #e5e7eb)',
                    borderRadius: 6,
                    background: 'var(--ppt-bg-surface, #fff)',
                    color: 'var(--ppt-fg-secondary)',
                    cursor: 'pointer',
                    fontFamily: 'inherit',
                  }}
                >
                  Load more
                </button>
              </div>
            )}

            {/* Loading subsequent pages */}
            {query.isLoading && allRows.length > 0 && (
              <div
                style={{
                  textAlign: 'center',
                  padding: '12px 0',
                  color: 'var(--ppt-fg-muted)',
                  fontSize: '0.875rem',
                }}
              >
                Loading…
              </div>
            )}
          </div>
        </div>
      </div>
    </>
  );
};

export default AuditPage;
