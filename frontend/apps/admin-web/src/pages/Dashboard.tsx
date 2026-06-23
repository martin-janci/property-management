/**
 * Dashboard at `/` — Section B of the admin design brief.
 *
 * Layout:
 *   (1) Topbar greeting with first name + MFA window subline
 *   (2) 4-stat tile row
 *   (3) Two-column row: high-risk events (left) + right stack
 *       Right stack: active impersonation | pending merge collisions | quick actions
 *
 * Capability gates:
 *   - High-risk events card: audit_read
 *   - Active impersonation card: users_impersonate OR audit_read
 *   - Pending merge collisions card: memberships_grant
 *   - Quick actions card: hidden if all gated out
 */

import { useCapability } from '@ppt/admin-ui';
import { useQuery } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';

import { useAdminAuth } from '../auth/AdminAuthContext';
import { useMfaWindow } from '../components/MfaWindowChip';
import { HelpTooltip } from '../features/help';

// ---------------------------------------------------------------------------
// Styles (injected once)
// ---------------------------------------------------------------------------

const STYLE_ID = 'ppt-dashboard-styles';

function ensureStyles() {
  if (typeof document === 'undefined') return;
  if (document.getElementById(STYLE_ID)) return;
  const el = document.createElement('style');
  el.id = STYLE_ID;
  el.textContent = `
    .ppt-dash { background: var(--ppt-bg-app, #f9fafb); padding: 20px 22px; flex: 1; }

    .ppt-dash__title {
      font-size: 1.25rem;
      font-weight: 700;
      color: var(--ppt-fg-primary, #111827);
      margin: 0 0 4px;
    }
    .ppt-dash__sub {
      color: var(--ppt-fg-muted, #6b7280);
      font-size: 12px;
      margin: 0 0 16px;
    }

    /* 4-stat row */
    .ppt-dash__grid4 {
      display: grid;
      grid-template-columns: repeat(4, 1fr);
      gap: 12px;
    }

    /* Card primitive */
    .ppt-card {
      background: var(--ppt-bg-surface, #ffffff);
      border: 1px solid var(--ppt-border-default, #e5e7eb);
      border-radius: var(--ppt-radius-lg, 10px);
      padding: 16px;
    }

    /* Stat tile */
    .ppt-tile__label {
      font-size: 11px;
      text-transform: uppercase;
      letter-spacing: 0.08em;
      color: var(--ppt-fg-muted, #6b7280);
      font-weight: 600;
      margin: 0 0 10px;
    }
    .ppt-tile__value {
      font-size: 28px;
      font-weight: 700;
      color: var(--ppt-fg-primary, #111827);
      line-height: 1;
    }
    .ppt-tile__detail {
      font-size: 11px;
      color: var(--ppt-fg-muted, #6b7280);
      margin-top: 4px;
    }

    /* Skeleton */
    @keyframes ppt-shimmer {
      0% { background-position: -400px 0; }
      100% { background-position: 400px 0; }
    }
    .ppt-skeleton {
      background: linear-gradient(
        90deg,
        var(--ppt-bg-subtle, #f3f4f6) 25%,
        var(--ppt-border-subtle, #e5e7eb) 50%,
        var(--ppt-bg-subtle, #f3f4f6) 75%
      );
      background-size: 800px 100%;
      animation: ppt-shimmer 1.4s infinite;
      border-radius: var(--ppt-radius-md, 6px);
    }
    .ppt-tile--skeleton .ppt-tile__value {
      height: 28px;
      width: 48px;
      border-radius: var(--ppt-radius-md, 6px);
    }
    .ppt-tile--skeleton .ppt-tile__detail {
      height: 12px;
      width: 80px;
      margin-top: 6px;
      border-radius: var(--ppt-radius-md, 6px);
    }

    /* Two-column row */
    .ppt-dash__grid2 {
      display: grid;
      grid-template-columns: 2fr 1fr;
      gap: 12px;
      margin-top: 14px;
    }

    /* Card section header */
    .ppt-card__header {
      font-size: 12px;
      text-transform: uppercase;
      letter-spacing: 0.08em;
      color: var(--ppt-fg-muted, #6b7280);
      font-weight: 600;
      margin: 0 0 12px;
    }
    .ppt-card__header-row {
      display: flex;
      align-items: center;
      justify-content: space-between;
      margin-bottom: 8px;
    }
    .ppt-card__header-row .ppt-card__header {
      margin: 0;
    }

    /* Row list */
    .ppt-row-list { display: flex; flex-direction: column; }
    .ppt-row {
      display: grid;
      grid-template-columns: 14px 1fr auto;
      gap: 10px;
      padding: 9px 0;
      border-bottom: 1px solid var(--ppt-border-subtle, #f3f4f6);
      align-items: start;
      font-size: 12px;
    }
    .ppt-row:last-child { border-bottom: 0; }

    /* Severity dot */
    .ppt-dot {
      width: 8px;
      height: 8px;
      border-radius: 50%;
      background: var(--ppt-danger-500, #ef4444);
      margin-top: 4px;
      flex-shrink: 0;
    }
    .ppt-dot--amber { background: var(--ppt-warning-500, #f59e0b); }
    .ppt-dot--blue { background: var(--ppt-brand-500, #3b82f6); }

    .ppt-row__text { color: var(--ppt-fg-secondary, #374151); line-height: 1.4; }
    .ppt-row__who { color: var(--ppt-fg-primary, #111827); font-weight: 500; }
    .ppt-row__target {
      font-family: var(--ppt-font-mono, ui-monospace, monospace);
      font-size: 11px;
      color: var(--ppt-fg-muted, #6b7280);
    }
    .ppt-row__time {
      font-family: var(--ppt-font-mono, ui-monospace, monospace);
      color: var(--ppt-fg-subtle, #9ca3af);
      font-size: 11px;
      white-space: nowrap;
    }

    /* Pill */
    .ppt-pill {
      display: inline-flex;
      align-items: center;
      padding: 1px 8px;
      border-radius: 999px;
      font-size: 10.5px;
      font-weight: 500;
      background: var(--ppt-bg-subtle, #f3f4f6);
      color: var(--ppt-fg-secondary, #374151);
      line-height: 1.6;
    }
    .ppt-pill--red { background: var(--ppt-danger-100, #fee2e2); color: var(--ppt-danger-800, #991b1b); }
    .ppt-pill--amber { background: var(--ppt-warning-100, #fef3c7); color: var(--ppt-warning-800, #92400e); }
    .ppt-pill--blue { background: var(--ppt-brand-100, #dbeafe); color: var(--ppt-brand-800, #1e40af); }

    /* Buttons */
    .ppt-btn {
      display: inline-flex;
      align-items: center;
      gap: 6px;
      padding: 5px 11px;
      border-radius: var(--ppt-radius-md, 6px);
      font-size: 11.5px;
      font-weight: 500;
      background: var(--ppt-bg-surface, #ffffff);
      border: 1px solid var(--ppt-border-default, #e5e7eb);
      color: var(--ppt-fg-secondary, #374151);
      cursor: pointer;
    }
    .ppt-btn:hover { background: var(--ppt-bg-subtle, #f3f4f6); }
    .ppt-btn--ghost { background: transparent; border: 0; color: var(--ppt-fg-secondary, #374151); }
    .ppt-btn--ghost:hover { color: var(--ppt-fg-primary, #111827); background: transparent; }
    .ppt-btn--tiny { padding: 3px 8px; font-size: 11px; }
    .ppt-btn--danger { color: var(--ppt-danger-600, #dc2626); }
    .ppt-btn--full { width: 100%; justify-content: flex-start; }

    /* Right stack */
    .ppt-dash__right-stack { display: flex; flex-direction: column; gap: 12px; }

    /* Empty state */
    .ppt-empty { font-size: 12px; color: var(--ppt-fg-muted, #6b7280); padding: 8px 0; }

    /* Error banner */
    .ppt-error-banner {
      font-size: 12px;
      color: var(--ppt-danger-700, #b91c1c);
      background: var(--ppt-danger-50, #fef2f2);
      border: 1px solid var(--ppt-danger-200, #fecaca);
      border-radius: var(--ppt-radius-md, 6px);
      padding: 8px 12px;
      margin-bottom: 12px;
    }
  `;
  document.head.appendChild(el);
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface MetricsSummary {
  tenantsActive: number;
  operatorsOnline: number;
  pendingMerges: number;
  highRisk24h: number;
}

interface AuditEvent {
  id: string;
  actor_email: string;
  capability: string;
  target_slug: string;
  severity: 'high' | 'medium' | 'low';
  occurred_at: string;
  outcome: string;
}

interface ImpersonationSession {
  id: string;
  operator_email: string;
  target_email: string;
  target_id: string;
  started_at: string;
  expires_at: string;
}

interface MergeCollision {
  id: string;
  description: string;
}

// ---------------------------------------------------------------------------
// API helpers
// ---------------------------------------------------------------------------

async function fetchMetricsSummary(token: string | null): Promise<MetricsSummary> {
  const resp = await fetch('/api/v1/admin/metrics/summary', {
    headers: token ? { Authorization: `Bearer ${token}` } : {},
    credentials: 'include',
  });
  if (!resp.ok) {
    throw new Error(`HTTP ${resp.status}`);
  }
  // Backend serializes camelCase (MetricsSummary has
  // `#[serde(rename_all = "camelCase")]`). Coerce each field through
  // `Number()` so missing / null values fall back to 0 instead of NaN.
  const raw = (await resp.json()) as Partial<MetricsSummary>;
  return {
    tenantsActive: Number(raw.tenantsActive ?? 0),
    operatorsOnline: Number(raw.operatorsOnline ?? 0),
    pendingMerges: Number(raw.pendingMerges ?? 0),
    highRisk24h: Number(raw.highRisk24h ?? 0),
  };
}

async function fetchHighRiskEvents(token: string | null): Promise<AuditEvent[]> {
  if (!token) return [];

  // Try the server-side filtered endpoint first; fall back to client-side filtering.
  try {
    const resp = await fetch('/api/v1/admin/audit?severity=high&after=24h&limit=10', {
      headers: { Authorization: `Bearer ${token}` },
      credentials: 'include',
    });
    if (resp.ok) {
      const data = (await resp.json()) as { items?: AuditEvent[] } | AuditEvent[];
      const items = Array.isArray(data) ? data : (data.items ?? []);
      return items.slice(0, 10);
    }

    // Fall back: fetch base audit log and client-filter
    const fallback = await fetch('/api/v1/admin/audit?limit=50', {
      headers: { Authorization: `Bearer ${token}` },
      credentials: 'include',
    });
    if (!fallback.ok) return [];
    const fallbackData = (await fallback.json()) as { items?: AuditEvent[] } | AuditEvent[];
    const allItems = Array.isArray(fallbackData) ? fallbackData : (fallbackData.items ?? []);
    const cutoff = Date.now() - 24 * 60 * 60 * 1000;
    return allItems
      .filter((e) => e.severity === 'high' && new Date(e.occurred_at).getTime() > cutoff)
      .slice(0, 10);
  } catch {
    return [];
  }
}

async function fetchActiveImpersonations(token: string | null): Promise<ImpersonationSession[]> {
  if (!token) return [];
  try {
    const resp = await fetch('/api/v1/admin/impersonation/active', {
      headers: { Authorization: `Bearer ${token}` },
      credentials: 'include',
    });
    if (!resp.ok) return [];
    const data = (await resp.json()) as ImpersonationSession[] | { items?: ImpersonationSession[] };
    return Array.isArray(data) ? data : (data.items ?? []);
  } catch {
    return [];
  }
}

async function fetchMergeCollisions(token: string | null): Promise<MergeCollision[]> {
  if (!token) return [];
  try {
    const resp = await fetch('/api/v1/admin/memberships/merge-collisions', {
      headers: { Authorization: `Bearer ${token}` },
      credentials: 'include',
    });
    if (!resp.ok) return [];
    const data = (await resp.json()) as MergeCollision[] | { items?: MergeCollision[] };
    return Array.isArray(data) ? data : (data.items ?? []);
  } catch {
    return [];
  }
}

// ---------------------------------------------------------------------------
// Sub-components
// ---------------------------------------------------------------------------

function TileSkeleton() {
  return (
    <div className="ppt-card ppt-tile--skeleton">
      <div className="ppt-tile__label ppt-skeleton" style={{ height: '11px', width: '80px' }} />
      <div className="ppt-tile__value ppt-skeleton" />
      <div className="ppt-tile__detail ppt-skeleton" />
    </div>
  );
}

function severityDotClass(severity: string, index: number): string {
  // Vary dot color by severity/index for visual variety matching the mockup
  if (severity === 'high' && index % 3 === 0) return 'ppt-dot';
  if (severity === 'high' && index % 3 === 1) return 'ppt-dot ppt-dot--amber';
  return 'ppt-dot ppt-dot--blue';
}

function pillColorClass(capability: string): string {
  const red = ['tenant_purge', 'tenant_restore', 'principal_kind_escalate'];
  const amber = ['memberships_grant', 'memberships_revoke'];
  if (red.includes(capability)) return 'ppt-pill ppt-pill--red';
  if (amber.includes(capability)) return 'ppt-pill ppt-pill--amber';
  return 'ppt-pill ppt-pill--blue';
}

function formatTime(iso: string): string {
  try {
    return new Date(iso).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  } catch {
    return '—';
  }
}

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

export function Dashboard() {
  ensureStyles();

  const { t } = useTranslation();
  const { token } = useAdminAuth();
  const navigate = useNavigate();
  const { state: mfaState, msRemaining } = useMfaWindow();

  // Capability gates
  const canAuditRead = useCapability('audit_read');
  const canUsersImpersonate = useCapability('users_impersonate');
  const canMembershipsGrant = useCapability('memberships_grant');

  // Derived first name from JWT sub (best-effort; fallback "Operator")
  const firstName = (() => {
    if (!token) return 'Operator';
    try {
      const parts = token.split('.');
      if (parts.length !== 3) return 'Operator';
      const padded = parts[1].replace(/-/g, '+').replace(/_/g, '/');
      const json = atob(padded.padEnd(padded.length + ((4 - (padded.length % 4)) % 4), '='));
      const payload = JSON.parse(json) as Record<string, unknown>;
      const sub = payload.sub ?? payload.email ?? payload.name ?? '';
      const emailLocal = String(sub).split('@')[0];
      const namePart = emailLocal.split(/[._-]/)[0];
      return namePart.length > 0
        ? namePart.charAt(0).toUpperCase() + namePart.slice(1)
        : 'Operator';
    } catch {
      return 'Operator';
    }
  })();

  // Greeting by time-of-day
  const hour = new Date().getHours();
  const greeting = hour < 12 ? 'Good morning' : hour < 18 ? 'Good afternoon' : 'Good evening';

  // MFA subline
  const mfaSubline = (() => {
    if (mfaState === 'verified' || mfaState === 'expiring') {
      const totalSec = Math.max(0, Math.ceil(msRemaining / 1000));
      const m = Math.floor(totalSec / 60);
      return `You're in a recent-MFA window for ${m} more minute${m !== 1 ? 's' : ''}.`;
    }
    return 'MFA window not active.';
  })();

  // TanStack Queries
  const metricsQuery = useQuery({
    queryKey: ['admin', 'metrics', 'summary'],
    queryFn: () => fetchMetricsSummary(token),
    staleTime: 60_000,
    refetchOnWindowFocus: true,
    enabled: token !== null,
  });

  const highRiskQuery = useQuery({
    queryKey: ['admin', 'audit', 'high-risk-24h'],
    queryFn: () => fetchHighRiskEvents(token),
    staleTime: 5_000,
    refetchOnWindowFocus: true,
    enabled: token !== null && canAuditRead,
  });

  const impersonationQuery = useQuery({
    queryKey: ['admin', 'impersonation', 'active'],
    queryFn: () => fetchActiveImpersonations(token),
    staleTime: 5_000,
    refetchInterval: 5_000,
    refetchOnWindowFocus: true,
    enabled: token !== null && (canUsersImpersonate || canAuditRead),
  });

  const mergeCollisionsQuery = useQuery({
    queryKey: ['admin', 'memberships', 'merge-collisions'],
    queryFn: () => fetchMergeCollisions(token),
    staleTime: 30_000,
    refetchOnWindowFocus: true,
    enabled: token !== null && canMembershipsGrant,
  });

  const metrics = metricsQuery.data;
  const highRiskEvents = highRiskQuery.data ?? [];
  const impersonations = impersonationQuery.data ?? [];
  const mergeCollisions = mergeCollisionsQuery.data ?? [];

  // "N things need attention" count
  const attentionCount =
    (metrics?.highRisk24h ?? 0) + impersonations.length + mergeCollisions.length;

  // Quick-actions visibility
  const showFindUser = canAuditRead || canUsersImpersonate || canMembershipsGrant;
  const showAuditLog = canAuditRead;
  const showImpersonations = canUsersImpersonate || canAuditRead;
  const showQuickActions = showFindUser || showAuditLog || showImpersonations;

  return (
    <div className="ppt-dash">
      {/* Greeting */}
      <h1 className="ppt-dash__title" style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
        {greeting}, {firstName}.
        <HelpTooltip text={t('admin.dashboard.helpTooltip')} />
      </h1>
      <p className="ppt-dash__sub">
        {attentionCount > 0
          ? `${attentionCount} thing${attentionCount !== 1 ? 's' : ''} need${attentionCount === 1 ? 's' : ''} attention.`
          : 'Everything looks clear.'}{' '}
        {mfaSubline}
      </p>

      {/* (1) 4-stat tile row */}
      <div className="ppt-dash__grid4">
        {metricsQuery.isLoading ? (
          <>
            <TileSkeleton />
            <TileSkeleton />
            <TileSkeleton />
            <TileSkeleton />
          </>
        ) : (
          <>
            <div className="ppt-card">
              <div className="ppt-tile__label">Tenants active</div>
              <div className="ppt-tile__value">{metrics?.tenantsActive ?? 0}</div>
              <div className="ppt-tile__detail">total registered</div>
            </div>
            <div className="ppt-card">
              <div className="ppt-tile__label">Operators online</div>
              <div className="ppt-tile__value">{metrics?.operatorsOnline ?? 0}</div>
              <div className="ppt-tile__detail">with active sessions</div>
            </div>
            <div className="ppt-card">
              <div className="ppt-tile__label">Pending merges</div>
              <div className="ppt-tile__value">{metrics?.pendingMerges ?? 0}</div>
              <div className="ppt-tile__detail">awaiting resolution</div>
            </div>
            <div className="ppt-card">
              <div className="ppt-tile__label">High-risk · 24h</div>
              <div className="ppt-tile__value" style={{ color: 'var(--ppt-danger-600, #dc2626)' }}>
                {metrics?.highRisk24h ?? 0}
              </div>
              <div className="ppt-tile__detail">purge / escalate events</div>
            </div>
          </>
        )}
      </div>

      {/* (2) Two-column row */}
      <div className="ppt-dash__grid2">
        {/* LEFT: High-risk events */}
        {canAuditRead ? (
          <div className="ppt-card">
            <div className="ppt-card__header-row">
              <h3 className="ppt-card__header">Recent high-risk events · 24h</h3>
              <button
                type="button"
                className="ppt-btn ppt-btn--ghost ppt-btn--tiny"
                onClick={() => navigate('/ops/audit')}
              >
                View all →
              </button>
            </div>

            {highRiskQuery.isError && (
              <div className="ppt-error-banner">Unable to load audit events. Try again.</div>
            )}

            {highRiskQuery.isLoading ? (
              <div className="ppt-row-list">
                {Array.from({ length: 4 }).map((_, i) => (
                  <div key={i} className="ppt-row">
                    <div
                      className="ppt-skeleton"
                      style={{ width: '8px', height: '8px', borderRadius: '50%', marginTop: '4px' }}
                    />
                    <div className="ppt-skeleton" style={{ height: '12px', borderRadius: '4px' }} />
                    <div
                      className="ppt-skeleton"
                      style={{ height: '12px', width: '36px', borderRadius: '4px' }}
                    />
                  </div>
                ))}
              </div>
            ) : highRiskEvents.length === 0 ? (
              <p className="ppt-empty">No high-risk events in the last 24 hours.</p>
            ) : (
              <div className="ppt-row-list">
                {highRiskEvents.map((event, idx) => (
                  <div key={event.id} className="ppt-row">
                    <span className={severityDotClass(event.severity, idx)} aria-hidden="true" />
                    <span className="ppt-row__text">
                      <span className="ppt-row__who">{event.actor_email}</span>
                      {' · '}
                      <span className={pillColorClass(event.capability)}>{event.capability}</span>
                      {' on '}
                      <span className="ppt-row__target">{event.target_slug}</span>
                    </span>
                    <span className="ppt-row__time">{formatTime(event.occurred_at)}</span>
                  </div>
                ))}
              </div>
            )}
          </div>
        ) : (
          /* Placeholder to keep grid layout when audit_read is absent */
          <div />
        )}

        {/* RIGHT stack */}
        <div className="ppt-dash__right-stack">
          {/* Active impersonation */}
          {(canUsersImpersonate || canAuditRead) && (
            <div className="ppt-card">
              <h3 className="ppt-card__header">Active impersonation</h3>
              {impersonationQuery.isLoading ? (
                <p className="ppt-empty">Loading…</p>
              ) : impersonations.length === 0 ? (
                <p className="ppt-empty">No active impersonation sessions.</p>
              ) : (
                <div className="ppt-row-list">
                  {impersonations.map((session) => (
                    <div key={session.id} className="ppt-row">
                      <span className="ppt-dot ppt-dot--amber" aria-hidden="true" />
                      <span className="ppt-row__text">
                        <span className="ppt-row__who">{session.operator_email}</span>
                        {' → '}
                        <span className="ppt-row__target">
                          {session.target_email || session.target_id}
                        </span>
                        <br />
                        <span style={{ color: 'var(--ppt-fg-muted, #6b7280)', fontSize: '11px' }}>
                          started {formatTime(session.started_at)} · expires{' '}
                          {formatTime(session.expires_at)}
                        </span>
                      </span>
                      {canUsersImpersonate && (
                        <button
                          type="button"
                          className="ppt-btn ppt-btn--tiny ppt-btn--danger"
                          onClick={() => navigate('/ops/impersonation')}
                        >
                          Stop
                        </button>
                      )}
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}

          {/* Pending merge collisions */}
          {canMembershipsGrant && (
            <div className="ppt-card">
              <h3 className="ppt-card__header">Pending merge collisions</h3>
              {mergeCollisionsQuery.isLoading ? (
                <p className="ppt-empty">Loading…</p>
              ) : mergeCollisions.length === 0 ? (
                <p className="ppt-empty">No merge collisions.</p>
              ) : (
                <div className="ppt-row-list">
                  {mergeCollisions.map((collision) => (
                    <div key={collision.id} className="ppt-row">
                      <span className="ppt-dot ppt-dot--amber" aria-hidden="true" />
                      <span className="ppt-row__text">{collision.description}</span>
                      <button
                        type="button"
                        className="ppt-btn ppt-btn--tiny"
                        onClick={() => navigate('/identity/memberships')}
                      >
                        Resolve
                      </button>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}

          {/* Quick actions */}
          {showQuickActions && (
            <div className="ppt-card">
              <h3 className="ppt-card__header">Quick actions</h3>
              <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
                {showFindUser && (
                  <button
                    type="button"
                    className="ppt-btn ppt-btn--full"
                    onClick={() => navigate('/identity/users')}
                  >
                    Find user…
                  </button>
                )}
                {showAuditLog && (
                  <button
                    type="button"
                    className="ppt-btn ppt-btn--full"
                    onClick={() => navigate('/ops/audit')}
                  >
                    Open audit log
                  </button>
                )}
                {showImpersonations && (
                  <button
                    type="button"
                    className="ppt-btn ppt-btn--full"
                    onClick={() => navigate('/ops/impersonation')}
                  >
                    View active impersonations ({impersonations.length})
                  </button>
                )}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
