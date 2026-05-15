/**
 * Audit-log viewer.
 *
 * Reads from `GET /admin/audit` (capability `audit_read`). The component is
 * presentational — the parent feeds it `entries` after fetching. Filters are
 * controlled state held by the parent.
 */

import type { ReactNode } from 'react';

export interface AuditEntry {
  id: string;
  user_id: string | null;
  action: string;
  resource_type: string | null;
  resource_id: string | null;
  details: unknown;
  ip_address: string | null;
  user_agent: string | null;
  created_at: string;
}

export interface AuditFilter {
  actor_id?: string;
  action?: string;
  target_type?: string;
  since?: string;
  until?: string;
}

export interface AuditViewerProps {
  /** Fetched audit rows. */
  entries: ReadonlyArray<AuditEntry>;
  /** Current filter state. */
  filter: AuditFilter;
  /** Filter change handler. */
  onFilterChange: (next: AuditFilter) => void;
  /** Optional loading state. */
  loading?: boolean;
  /** Optional empty/error message. */
  emptyMessage?: ReactNode;
}

export function AuditViewer({
  entries,
  filter,
  onFilterChange,
  loading = false,
  emptyMessage,
}: AuditViewerProps) {
  return (
    <div className="ppt-admin-audit">
      <fieldset className="ppt-admin-audit-filters">
        <legend>Filters</legend>
        <label>
          Actor ID:
          <input
            type="text"
            value={filter.actor_id ?? ''}
            onChange={(e) =>
              onFilterChange({ ...filter, actor_id: e.target.value || undefined })
            }
          />
        </label>
        <label>
          Action:
          <input
            type="text"
            value={filter.action ?? ''}
            onChange={(e) => onFilterChange({ ...filter, action: e.target.value || undefined })}
          />
        </label>
        <label>
          Target type:
          <input
            type="text"
            value={filter.target_type ?? ''}
            onChange={(e) =>
              onFilterChange({ ...filter, target_type: e.target.value || undefined })
            }
          />
        </label>
        <label>
          Since:
          <input
            type="datetime-local"
            value={filter.since ?? ''}
            onChange={(e) => onFilterChange({ ...filter, since: e.target.value || undefined })}
          />
        </label>
        <label>
          Until:
          <input
            type="datetime-local"
            value={filter.until ?? ''}
            onChange={(e) => onFilterChange({ ...filter, until: e.target.value || undefined })}
          />
        </label>
      </fieldset>

      {loading ? <div>Loading…</div> : null}

      <table className="ppt-admin-audit-table">
        <thead>
          <tr>
            <th>Time</th>
            <th>Actor</th>
            <th>Action</th>
            <th>Target</th>
            <th>IP</th>
          </tr>
        </thead>
        <tbody>
          {entries.length === 0 && !loading ? (
            <tr>
              <td colSpan={5}>
                <em>{emptyMessage ?? 'No audit events match the current filters.'}</em>
              </td>
            </tr>
          ) : (
            entries.map((e) => (
              <tr key={e.id}>
                <td>
                  <time dateTime={e.created_at}>{e.created_at}</time>
                </td>
                <td>{e.user_id ?? <em>(system)</em>}</td>
                <td>{e.action}</td>
                <td>
                  {e.resource_type ?? '—'}
                  {e.resource_id ? ` / ${e.resource_id}` : ''}
                </td>
                <td>{e.ip_address ?? '—'}</td>
              </tr>
            ))
          )}
        </tbody>
      </table>
    </div>
  );
}
