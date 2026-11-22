/**
 * Wire adapter for `GET /api/v1/admin/audit` (+ `/csv`).
 *
 * The api-server returns `{ items: ServerAuditRow[], next_cursor? }` where each
 * row is a raw `audit_logs` record (`user_id`, `resource_type`, `details`, …)
 * plus `actor_email`. Admin pages render a flatter view (`occurred_at`,
 * `capability`, `outcome`, `target_slug`, …), so this module owns both the
 * filter → query-string mapping and the row mapping in one place.
 */

export interface ServerAuditRow {
  id: string;
  user_id?: string | null;
  actor_email?: string | null;
  action: string;
  resource_type?: string | null;
  resource_id?: string | null;
  details?: Record<string, unknown> | null;
  ip_address?: string | null;
  user_agent?: string | null;
  created_at: string;
}

export interface ServerAuditPage {
  items: ServerAuditRow[];
  next_cursor?: string;
}

export type AuditOutcome = 'success' | 'mfa_failed' | 'denied';

export interface AuditEventView {
  id: string;
  occurred_at: string;
  actor_email?: string;
  actor_user_id?: string;
  /** `details.capability` for capability-gated calls, else the raw action. */
  capability?: string;
  target_kind?: string;
  target_slug?: string;
  outcome?: AuditOutcome;
  reason?: string;
  ip?: string;
  user_agent?: string;
  mfa_recent_at_call?: string;
  request?: { path?: string; headers?: Record<string, string>; body?: string };
  principal_kind?: string;
  severity: 'high' | 'normal';
}

export const HIGH_RISK_CAPABILITIES = new Set([
  'tenant_purge',
  'principal_kind_escalate',
  'grant_principal_kind_escalate',
  'tenant_restore',
]);

export interface AuditFilterParams {
  actor?: string;
  target?: string;
  capabilities?: string[];
  severity?: string;
  outcome?: string;
  /** Relative preset such as `24h` or `7d`. */
  dateRange?: string;
  limit?: number;
}

const str = (v: unknown): string | undefined => (typeof v === 'string' && v !== '' ? v : undefined);

export function toAuditEventView(row: ServerAuditRow): AuditEventView {
  const d = row.details ?? {};
  const capability = str(d.capability) ?? row.action;
  const outcome = str(d.outcome) as AuditOutcome | undefined;
  const request = d.request as AuditEventView['request'] | undefined;
  return {
    id: row.id,
    occurred_at: row.created_at,
    actor_email: row.actor_email ?? undefined,
    actor_user_id: row.user_id ?? undefined,
    capability,
    target_kind: row.resource_type ?? undefined,
    target_slug: row.resource_id ?? undefined,
    outcome,
    reason: str(d.reason),
    ip: row.ip_address ?? undefined,
    user_agent: row.user_agent ?? undefined,
    mfa_recent_at_call: str(d.mfa_recent_at_call),
    request: request && typeof request === 'object' ? request : undefined,
    principal_kind: str(d.principal_kind),
    severity: HIGH_RISK_CAPABILITIES.has(capability) ? 'high' : 'normal',
  };
}

/** Query string for the audit list / CSV endpoints (no leading `?`). */
export function buildAuditQuery(filters: AuditFilterParams, cursor?: string): string {
  const params = new URLSearchParams();
  if (filters.actor) params.set('actor', filters.actor);
  if (filters.target) params.set('target', filters.target);
  if (filters.capabilities?.length) params.set('capability', filters.capabilities.join(','));
  if (filters.severity === 'high') params.set('severity', 'high');
  if (filters.outcome && filters.outcome !== 'all') params.set('outcome', filters.outcome);
  if (filters.dateRange) params.set('range', filters.dateRange);
  if (filters.limit) params.set('limit', String(filters.limit));
  if (cursor) params.set('cursor', cursor);
  return params.toString();
}

export interface AuditEventsPage {
  items: AuditEventView[];
  next_cursor?: string;
}

/** Accepts the paged shape and, defensively, a bare array. */
export function parseAuditPage(data: ServerAuditPage | ServerAuditRow[]): AuditEventsPage {
  const raw = Array.isArray(data) ? { items: data } : data;
  return {
    items: (raw.items ?? []).map(toAuditEventView),
    next_cursor: Array.isArray(data) ? undefined : data.next_cursor,
  };
}
