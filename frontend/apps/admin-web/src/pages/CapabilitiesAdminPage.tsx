/**
 * CapabilitiesAdminPage — /identity/capabilities
 *
 * Two views:
 *   - Registry view (?view=registry or default): card grid of all 17 capability strings.
 *   - Per-user view (?view=user&id=<userId>): table of grants for one user.
 *
 * Page gating: memberships_grant (enforced by ProtectedRoute in App.tsx).
 *
 * Grant flow:
 *   1. Open drawer → fill capability + optional expires_at.
 *   2. For tenant_purge / principal_kind_escalate: AuditReasonPrompt required (min 30 chars).
 *   3. For principal_kind_escalate: extra warning banner + 3000ms confirm delay.
 *   4. Submit → MFA challenge → POST /api/v1/admin/capabilities/users/:id/grant.
 *
 * Revoke flow:
 *   - Per-row button → MFA challenge (+ AuditReasonPrompt for high-risk caps)
 *     → DELETE /api/v1/admin/capabilities/users/:id/grant/:grantId
 */

import { CAPABILITIES, useCapability, useMfaChallenge } from '@ppt/admin-ui';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useCallback, useEffect, useRef, useState } from 'react';
import { useSearchParams } from 'react-router-dom';

import { useAdminAuth } from '../auth/AdminAuthContext';
import { AuditReasonPrompt, useAuditReasonValid } from '../components/AuditReasonPrompt';

// ---------------------------------------------------------------------------
// Constants & helpers
// ---------------------------------------------------------------------------

// TODO: replace stub descriptions with data from GET /api/v1/admin/capabilities/registry
const CAPABILITY_DESCRIPTIONS: Record<string, string> = {
  agencies_read: 'View agencies and their memberships.',
  agencies_write: 'Create and edit agency records.',
  agencies_suspend: 'Suspend or reinstate an agency.',
  users_read: 'List and inspect user accounts.',
  users_write: 'Edit user profile fields and metadata.',
  users_impersonate: 'Impersonate a user for support.',
  memberships_grant: 'Grant capabilities to platform principals.',
  memberships_revoke: 'Revoke capabilities from principals.',
  site_settings_read: 'View tenant-wide site settings.',
  site_settings_write: 'Modify site settings.',
  mobile_config_write: 'Push mobile-app configuration.',
  feature_flags_write: 'Toggle and configure feature flags.',
  tenant_export: 'Trigger a full tenant data export.',
  tenant_purge: 'Permanently delete a tenant and all its data.',
  tenant_restore: 'Restore a tenant from an archive.',
  audit_read: 'View the platform audit log.',
  principal_kind_escalate: 'Escalate a user to a higher principal kind.',
};

const HIGH_RISK_CAPS = new Set(['tenant_purge', 'principal_kind_escalate']);

function formatDate(iso: string | null): string {
  if (!iso) return '—';
  try {
    return new Intl.DateTimeFormat('en-GB', {
      dateStyle: 'short',
      timeStyle: 'short',
    }).format(new Date(iso));
  } catch {
    return iso;
  }
}

function timeAgo(iso: string | null): string {
  if (!iso) return 'unknown';
  const diff = Date.now() - new Date(iso).getTime();
  const mins = Math.floor(diff / 60_000);
  if (mins < 1) return 'just now';
  if (mins < 60) return `${mins}m ago`;
  const hrs = Math.floor(mins / 60);
  if (hrs < 24) return `${hrs}h ago`;
  return `${Math.floor(hrs / 24)}d ago`;
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface CapabilityGrant {
  id: string;
  capability: string;
  granted_at: string;
  granted_by: string;
  expires_at: string | null;
  revoked_at: string | null;
  status: 'active' | 'expired' | 'revoked';
}

interface UserCapabilitiesResponse {
  user_id: string;
  email: string;
  grants: CapabilityGrant[];
  last_change_at: string | null;
  last_change_by: string | null;
}

// ---------------------------------------------------------------------------
// Styles injection
// ---------------------------------------------------------------------------

const STYLE_ID = 'ppt-cap-page-styles';

function ensureStyles() {
  if (typeof document === 'undefined') return;
  if (document.getElementById(STYLE_ID)) return;
  const el = document.createElement('style');
  el.id = STYLE_ID;
  el.textContent = `
    /* ---- Page shell ---- */
    .cap-page {
      padding: 24px 32px;
      background: var(--ppt-bg-app, #f9fafb);
      min-height: 100%;
    }
    .cap-page__header {
      display: flex;
      align-items: flex-start;
      justify-content: space-between;
      gap: 16px;
      margin-bottom: 24px;
      flex-wrap: wrap;
    }
    .cap-page__title {
      font-size: 1.375rem;
      font-weight: 700;
      color: var(--ppt-fg-primary, #111827);
      margin: 0;
    }
    .cap-page__subtitle {
      font-size: 0.8125rem;
      color: var(--ppt-fg-muted, #6b7280);
      margin: 4px 0 0;
    }
    .cap-page__actions {
      display: flex;
      align-items: center;
      gap: 8px;
      flex-shrink: 0;
    }

    /* ---- Registry card grid ---- */
    .cap-grid {
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
      gap: 12px;
    }
    .cap-card {
      background: var(--ppt-bg-surface, #ffffff);
      border: 1px solid var(--ppt-border-default, #e5e7eb);
      border-radius: var(--ppt-radius-lg, 12px);
      padding: 16px;
      display: flex;
      flex-direction: column;
      gap: 8px;
      transition: box-shadow 120ms ease;
    }
    .cap-card:hover {
      box-shadow: var(--ppt-shadow-card, 0 2px 8px rgba(0,0,0,0.08));
    }
    .cap-card__name {
      font-family: var(--ppt-font-mono, 'JetBrains Mono', monospace);
      font-size: 14px;
      font-weight: 600;
      color: var(--ppt-fg-primary, #111827);
    }
    .cap-card__desc {
      font-size: 12px;
      color: var(--ppt-fg-secondary, #374151);
      line-height: 1.5;
      flex: 1;
    }
    .cap-card__footer {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 8px;
      margin-top: 4px;
    }
    .cap-card__holders {
      font-size: 12px;
      color: var(--ppt-fg-muted, #6b7280);
      font-variant-numeric: tabular-nums;
    }
    .cap-card__link {
      font-size: 12px;
      color: var(--ppt-fg-link, #2563eb);
      text-decoration: none;
      background: none;
      border: none;
      cursor: pointer;
      padding: 0;
    }
    .cap-card__link:hover { text-decoration: underline; }

    /* ---- Per-user table ---- */
    .cap-table-wrap {
      overflow-x: auto;
      background: var(--ppt-bg-surface, #ffffff);
      border: 1px solid var(--ppt-border-default, #e5e7eb);
      border-radius: var(--ppt-radius-lg, 12px);
    }
    .cap-table {
      width: 100%;
      border-collapse: collapse;
      font-size: 13px;
    }
    .cap-table thead {
      background: var(--ppt-bg-subtle, #f3f4f6);
    }
    .cap-table th {
      padding: 10px 14px;
      text-align: left;
      font-size: 11px;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.06em;
      color: var(--ppt-fg-muted, #6b7280);
      white-space: nowrap;
      border-bottom: 1px solid var(--ppt-border-default, #e5e7eb);
    }
    .cap-table td {
      padding: 10px 14px;
      border-bottom: 1px solid var(--ppt-border-subtle, #f3f4f6);
      vertical-align: middle;
    }
    .cap-table tbody tr:last-child td { border-bottom: none; }
    .cap-table tbody tr.cap-row--expired { opacity: 0.55; }
    .cap-mono {
      font-family: var(--ppt-font-mono, 'JetBrains Mono', monospace);
      font-size: 12px;
    }

    /* ---- Status pills ---- */
    .cap-pill {
      display: inline-flex;
      align-items: center;
      padding: 2px 8px;
      border-radius: 999px;
      font-size: 11px;
      font-weight: 600;
      letter-spacing: 0.04em;
    }
    .cap-pill--active {
      background: var(--ppt-success-100, #d1fae5);
      color: var(--ppt-success-800, #065f46);
    }
    .cap-pill--expired, .cap-pill--revoked {
      background: var(--ppt-bg-subtle, #f3f4f6);
      color: var(--ppt-fg-muted, #6b7280);
    }

    /* ---- Buttons ---- */
    .cap-btn {
      display: inline-flex;
      align-items: center;
      gap: 6px;
      padding: 7px 14px;
      border-radius: var(--ppt-radius-md, 8px);
      font-size: 13px;
      font-weight: 500;
      cursor: pointer;
      border: none;
      transition: background 120ms ease;
      outline: none;
    }
    .cap-btn:focus-visible { box-shadow: 0 0 0 3px rgba(37,99,235,0.35); }
    .cap-btn--primary {
      background: var(--ppt-brand-600, #2563eb);
      color: #fff;
    }
    .cap-btn--primary:hover { background: var(--ppt-brand-700, #1d4ed8); }
    .cap-btn--secondary {
      background: transparent;
      color: var(--ppt-fg-secondary, #374151);
      border: 1px solid var(--ppt-border-default, #e5e7eb);
    }
    .cap-btn--secondary:hover { background: var(--ppt-bg-hover, #f3f4f6); }
    .cap-btn--danger {
      background: transparent;
      color: var(--ppt-danger-600, #dc2626);
      border: 1px solid var(--ppt-danger-300, #fca5a5);
      font-size: 12px;
      padding: 4px 10px;
    }
    .cap-btn--danger:hover { background: var(--ppt-danger-50, #fef2f2); }
    .cap-btn:disabled {
      opacity: 0.45;
      cursor: not-allowed;
    }

    /* ---- Grant drawer ---- */
    .cap-drawer-backdrop {
      position: fixed;
      inset: 0;
      z-index: 800;
      background: rgba(0,0,0,0.35);
    }
    .cap-drawer {
      position: fixed;
      right: 0;
      top: 0;
      bottom: 0;
      width: 460px;
      background: var(--ppt-bg-surface, #ffffff);
      box-shadow: -4px 0 24px rgba(0,0,0,0.12);
      z-index: 801;
      display: flex;
      flex-direction: column;
      overflow: hidden;
    }
    .cap-drawer__header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 20px 24px 16px;
      border-bottom: 1px solid var(--ppt-border-default, #e5e7eb);
      flex-shrink: 0;
    }
    .cap-drawer__title {
      font-size: 15px;
      font-weight: 600;
      color: var(--ppt-fg-primary, #111827);
      margin: 0;
    }
    .cap-drawer__close {
      background: none;
      border: none;
      cursor: pointer;
      color: var(--ppt-fg-muted, #6b7280);
      padding: 4px;
      border-radius: 4px;
      display: flex;
      align-items: center;
    }
    .cap-drawer__close:hover { color: var(--ppt-fg-primary, #111827); }
    .cap-drawer__body {
      flex: 1;
      overflow-y: auto;
      padding: 20px 24px;
      display: flex;
      flex-direction: column;
      gap: 16px;
    }
    .cap-drawer__footer {
      padding: 16px 24px;
      border-top: 1px solid var(--ppt-border-default, #e5e7eb);
      display: flex;
      justify-content: flex-end;
      gap: 10px;
      flex-shrink: 0;
    }
    .cap-field {
      display: flex;
      flex-direction: column;
      gap: 6px;
    }
    .cap-field__label {
      font-size: 13px;
      font-weight: 500;
      color: var(--ppt-fg-secondary, #374151);
    }
    .cap-field__select, .cap-field__input {
      width: 100%;
      box-sizing: border-box;
      font-size: 13px;
      padding: 8px 10px;
      border: 1px solid var(--ppt-border-default, #e5e7eb);
      border-radius: var(--ppt-radius-md, 8px);
      background: var(--ppt-bg-input, #ffffff);
      color: var(--ppt-fg-primary, #111827);
      outline: none;
      transition: border-color 120ms ease, box-shadow 120ms ease;
    }
    .cap-field__select:focus, .cap-field__input:focus {
      border-color: var(--ppt-brand-500, #3b82f6);
      box-shadow: 0 0 0 2px rgba(59,130,246,0.25);
    }

    /* ---- Warning callout ---- */
    .cap-warning {
      display: flex;
      align-items: flex-start;
      gap: 10px;
      padding: 12px 14px;
      background: var(--ppt-warning-50, #fffbeb);
      border: 1px solid var(--ppt-warning-300, #fcd34d);
      border-radius: var(--ppt-radius-md, 8px);
      font-size: 13px;
      color: var(--ppt-warning-800, #92400e);
      line-height: 1.5;
    }

    /* ---- Inline error banner ---- */
    .cap-error-banner {
      padding: 10px 14px;
      background: var(--ppt-danger-50, #fef2f2);
      border: 1px solid var(--ppt-danger-300, #fca5a5);
      border-radius: var(--ppt-radius-md, 8px);
      font-size: 13px;
      color: var(--ppt-danger-800, #991b1b);
    }

    /* ---- Loading skeleton ---- */
    .cap-skeleton {
      background: var(--ppt-bg-subtle, #f3f4f6);
      border-radius: 6px;
      animation: cap-shimmer 1.4s ease infinite;
    }
    @keyframes cap-shimmer {
      0%, 100% { opacity: 1; }
      50% { opacity: 0.5; }
    }

    /* ---- Spinner ---- */
    .cap-spinner {
      width: 14px;
      height: 14px;
      border: 2px solid rgba(255,255,255,0.4);
      border-top-color: #fff;
      border-radius: 50%;
      animation: cap-spin 0.7s linear infinite;
      flex-shrink: 0;
    }
    @keyframes cap-spin { to { transform: rotate(360deg); } }
    .cap-spinner--dark {
      border-color: rgba(0,0,0,0.1);
      border-top-color: var(--ppt-fg-secondary, #374151);
    }
  `;
  document.head.appendChild(el);
}

// ---------------------------------------------------------------------------
// Registry view
// ---------------------------------------------------------------------------

interface RegistryViewProps {
  onViewUser: (userId: string) => void;
}

function RegistryView({ onViewUser }: RegistryViewProps) {
  // TODO: fetch holder counts from GET /api/v1/admin/capabilities/registry
  // when backend supports it; currently returns placeholder "—".
  return (
    <div className="cap-grid">
      {CAPABILITIES.map((cap) => (
        <article key={cap} className="cap-card">
          <span className="cap-card__name">{cap}</span>
          <span className="cap-card__desc">
            {CAPABILITY_DESCRIPTIONS[cap] ?? 'No description available.'}
          </span>
          <div className="cap-card__footer">
            <span className="cap-card__holders">Holders: —</span>
            <button type="button" className="cap-card__link" onClick={() => onViewUser('me')}>
              View holders →
            </button>
          </div>
        </article>
      ))}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Per-user view
// ---------------------------------------------------------------------------

interface UserViewProps {
  userId: string;
  token: string | null;
  onSwitchToRegistry: () => void;
}

function UserView({ userId, token, onSwitchToRegistry }: UserViewProps) {
  const queryClient = useQueryClient();
  const { prompt: promptMfa } = useMfaChallenge();
  const canRevoke = useCapability('memberships_revoke');
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [revokeState, setRevokeState] = useState<{
    grantId: string;
    capability: string;
  } | null>(null);

  const isMe = userId === 'me';

  const { data, isLoading, error, refetch } = useQuery<UserCapabilitiesResponse>({
    queryKey: ['admin', 'capabilities', 'user', userId],
    queryFn: async () => {
      const url = isMe
        ? '/api/v1/admin/capabilities/me'
        : `/api/v1/admin/capabilities/users/${userId}`;
      const headers: Record<string, string> = {};
      if (token) headers.Authorization = `Bearer ${token}`;
      const res = await fetch(url, { headers, credentials: 'include' });
      if (!res.ok) {
        // Try fallback to /me on 404
        if (!isMe && res.status === 404) {
          const fallback = await fetch('/api/v1/admin/capabilities/me', {
            headers,
            credentials: 'include',
          });
          if (!fallback.ok) throw new Error(`Unable to load capabilities: ${fallback.status}`);
          return fallback.json() as Promise<UserCapabilitiesResponse>;
        }
        throw new Error(`Unable to load capabilities: ${res.status}`);
      }
      return res.json() as Promise<UserCapabilitiesResponse>;
    },
    staleTime: 30_000,
    retry: 1,
  });

  const revokeMutation = useMutation({
    mutationFn: async ({ grantId, reason }: { grantId: string; reason?: string }) => {
      const mfaOk = await promptMfa('Revoke capability');
      if (!mfaOk) throw new Error('MFA cancelled');

      const headers: Record<string, string> = { 'X-MFA-Recent': '1' };
      if (token) headers.Authorization = `Bearer ${token}`;
      if (reason) headers['Content-Type'] = 'application/json';

      const res = await fetch(`/api/v1/admin/capabilities/users/${userId}/grant/${grantId}`, {
        method: 'DELETE',
        headers,
        credentials: 'include',
        ...(reason ? { body: JSON.stringify({ reason }) } : {}),
      });
      if (!res.ok) throw new Error(`Revoke failed: ${res.status}`);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['admin', 'capabilities', 'user', userId] });
      setRevokeState(null);
    },
  });

  const activeGrants = data?.grants.filter((g) => g.status === 'active') ?? [];
  const lastChange = data?.last_change_at ?? null;
  const lastChangeBy = data?.last_change_by ?? null;

  if (isLoading) {
    return (
      <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
        {[...Array(4)].map((_, i) => (
          // skeleton rows use index as key — they have no stable identity
          <div key={i} className="cap-skeleton" style={{ height: 40 }} />
        ))}
      </div>
    );
  }

  if (error) {
    return (
      <div className="cap-error-banner">
        {error instanceof Error ? error.message : 'Unable to load capabilities. Try again.'}
        <button
          type="button"
          className="cap-btn cap-btn--secondary"
          onClick={() => refetch()}
          style={{ marginLeft: 12 }}
        >
          Try again
        </button>
      </div>
    );
  }

  return (
    <>
      {/* Revoke reason prompt for high-risk caps */}
      {revokeState && (
        <RevokeReasonDialog
          capability={revokeState.capability}
          onConfirm={(reason) => {
            revokeMutation.mutate({ grantId: revokeState.grantId, reason });
          }}
          onCancel={() => setRevokeState(null)}
          isPending={revokeMutation.isPending}
          error={revokeMutation.error instanceof Error ? revokeMutation.error.message : undefined}
        />
      )}

      <div className="cap-page__header">
        <div>
          <h1 className="cap-page__title">Capabilities · {data?.email ?? userId}</h1>
          <p className="cap-page__subtitle">
            {activeGrants.length} active grant{activeGrants.length !== 1 ? 's' : ''}
            {lastChange && lastChangeBy
              ? ` · last change ${timeAgo(lastChange)} by ${lastChangeBy}`
              : ''}
          </p>
        </div>
        <div className="cap-page__actions">
          <button type="button" className="cap-btn cap-btn--secondary" onClick={onSwitchToRegistry}>
            Registry
          </button>
          <button
            type="button"
            className="cap-btn cap-btn--primary"
            onClick={() => setDrawerOpen(true)}
          >
            + Grant capability
          </button>
        </div>
      </div>

      {revokeMutation.error && (
        <div className="cap-error-banner" style={{ marginBottom: 16 }}>
          {revokeMutation.error instanceof Error
            ? revokeMutation.error.message
            : 'Revoke failed. Try again.'}
        </div>
      )}

      <div className="cap-table-wrap">
        <table className="cap-table">
          <thead>
            <tr>
              <th>Capability</th>
              <th>Granted</th>
              <th>Granted by</th>
              <th>Expires</th>
              <th>Status</th>
              <th aria-label="Actions" />
            </tr>
          </thead>
          <tbody>
            {(data?.grants ?? []).length === 0 && (
              <tr>
                <td
                  colSpan={6}
                  style={{
                    textAlign: 'center',
                    color: 'var(--ppt-fg-muted, #6b7280)',
                    padding: '24px',
                  }}
                >
                  No capability grants found.
                </td>
              </tr>
            )}
            {(data?.grants ?? []).map((grant) => {
              const isExpiredOrRevoked = grant.status !== 'active';
              const isHighRisk = HIGH_RISK_CAPS.has(grant.capability);
              return (
                <tr key={grant.id} className={isExpiredOrRevoked ? 'cap-row--expired' : ''}>
                  <td>
                    <code className="cap-mono">{grant.capability}</code>
                  </td>
                  <td style={{ whiteSpace: 'nowrap' }}>{formatDate(grant.granted_at)}</td>
                  <td>
                    <code className="cap-mono">{grant.granted_by}</code>
                  </td>
                  <td style={{ whiteSpace: 'nowrap' }}>
                    {grant.expires_at ? formatDate(grant.expires_at) : '—'}
                  </td>
                  <td>
                    <span className={`cap-pill cap-pill--${grant.status}`}>{grant.status}</span>
                  </td>
                  <td>
                    {!isExpiredOrRevoked && canRevoke && (
                      <button
                        type="button"
                        className="cap-btn cap-btn--danger"
                        disabled={revokeMutation.isPending}
                        onClick={() => {
                          if (isHighRisk) {
                            setRevokeState({ grantId: grant.id, capability: grant.capability });
                          } else {
                            revokeMutation.mutate({ grantId: grant.id });
                          }
                        }}
                      >
                        Revoke
                      </button>
                    )}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      {drawerOpen && (
        <GrantDrawer
          userId={userId}
          token={token}
          onClose={() => setDrawerOpen(false)}
          onSuccess={() => {
            setDrawerOpen(false);
            queryClient.invalidateQueries({
              queryKey: ['admin', 'capabilities', 'user', userId],
            });
          }}
        />
      )}
    </>
  );
}

// ---------------------------------------------------------------------------
// Revoke reason dialog (for high-risk caps only)
// ---------------------------------------------------------------------------

interface RevokeReasonDialogProps {
  capability: string;
  onConfirm: (reason: string) => void;
  onCancel: () => void;
  isPending: boolean;
  error?: string;
}

function RevokeReasonDialog({
  capability,
  onConfirm,
  onCancel,
  isPending,
  error,
}: RevokeReasonDialogProps) {
  const [reason, setReason] = useState('');
  const action =
    capability === 'tenant_purge' ? 'grant_tenant_purge' : 'grant_principal_kind_escalate';
  const isValid = useAuditReasonValid(reason, 30);

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 900,
        background: 'rgba(0,0,0,0.5)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        padding: 16,
      }}
    >
      <div
        style={{
          maxWidth: 480,
          width: '100%',
          background: 'var(--ppt-bg-surface, #fff)',
          borderRadius: 'var(--ppt-radius-lg, 12px)',
          padding: 24,
          display: 'flex',
          flexDirection: 'column',
          gap: 16,
          boxShadow: 'var(--ppt-shadow-modal, 0 10px 40px rgba(0,0,0,0.15))',
        }}
      >
        <h2
          style={{
            margin: 0,
            fontSize: 16,
            fontWeight: 600,
            color: 'var(--ppt-danger-700, #b91c1c)',
          }}
        >
          Revoke high-risk capability
        </h2>
        <p style={{ margin: 0, fontSize: 13, color: 'var(--ppt-fg-secondary, #374151)' }}>
          Revoking <code className="cap-mono">{capability}</code> requires an audit reason.
        </p>
        <AuditReasonPrompt action={action as never} value={reason} onChange={setReason} required />
        {error && <div className="cap-error-banner">{error}</div>}
        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 10 }}>
          <button
            type="button"
            className="cap-btn cap-btn--secondary"
            onClick={onCancel}
            disabled={isPending}
          >
            Cancel
          </button>
          <button
            type="button"
            className="cap-btn cap-btn--danger"
            disabled={!isValid || isPending}
            onClick={() => onConfirm(reason)}
          >
            {isPending && <span className="cap-spinner cap-spinner--dark" />}
            Revoke
          </button>
        </div>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Grant capability drawer
// ---------------------------------------------------------------------------

interface GrantDrawerProps {
  userId: string;
  token: string | null;
  onClose: () => void;
  onSuccess: () => void;
}

function GrantDrawer({ userId, token, onClose, onSuccess }: GrantDrawerProps) {
  const { prompt: promptMfa } = useMfaChallenge();
  const [capability, setCapability] = useState('');
  const [expiresAt, setExpiresAt] = useState('');
  const [reason, setReason] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [isPending, setIsPending] = useState(false);
  const [delayRemaining, setDelayRemaining] = useState(0);
  const delayTimerRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const isHighRisk = HIGH_RISK_CAPS.has(capability);
  const isEscalate = capability === 'principal_kind_escalate';
  const requiresReason = isHighRisk;
  const reasonAction = isHighRisk
    ? capability === 'tenant_purge'
      ? 'grant_tenant_purge'
      : 'grant_principal_kind_escalate'
    : null;
  const reasonIsValid = useAuditReasonValid(reason, 30);
  const reasonSatisfied = requiresReason ? reasonIsValid : true;

  // 3-second delay for principal_kind_escalate
  useEffect(() => {
    if (!isEscalate || !reasonSatisfied || !capability) {
      setDelayRemaining(0);
      if (delayTimerRef.current) {
        clearInterval(delayTimerRef.current);
        delayTimerRef.current = null;
      }
      return;
    }
    // Start the delay when capability changes to escalate and reason is valid
    setDelayRemaining(3000);
    const interval = 100;
    delayTimerRef.current = setInterval(() => {
      setDelayRemaining((prev) => {
        const next = prev - interval;
        if (next <= 0) {
          if (delayTimerRef.current) {
            clearInterval(delayTimerRef.current);
            delayTimerRef.current = null;
          }
          return 0;
        }
        return next;
      });
    }, interval);
    return () => {
      if (delayTimerRef.current) {
        clearInterval(delayTimerRef.current);
        delayTimerRef.current = null;
      }
    };
  }, [isEscalate, reasonSatisfied, capability]);

  const delayInProgress = isEscalate && delayRemaining > 0;
  const canSubmit = capability !== '' && reasonSatisfied && !delayInProgress && !isPending;

  const handleSubmit = useCallback(async () => {
    if (!canSubmit) return;
    setError(null);
    setIsPending(true);
    try {
      const mfaOk = await promptMfa('Grant capability');
      if (!mfaOk) {
        setError('MFA verification cancelled.');
        setIsPending(false);
        return;
      }
      const headers: Record<string, string> = {
        'Content-Type': 'application/json',
        'X-MFA-Recent': '1',
      };
      if (token) headers.Authorization = `Bearer ${token}`;

      const body: Record<string, unknown> = { capability };
      if (expiresAt) body.expires_at = expiresAt;
      if (requiresReason && reason) body.reason = reason;

      const res = await fetch(`/api/v1/admin/capabilities/users/${userId}/grant`, {
        method: 'POST',
        headers,
        credentials: 'include',
        body: JSON.stringify(body),
      });
      if (!res.ok) {
        const text = await res.text().catch(() => '');
        throw new Error(text || `Grant failed: ${res.status}`);
      }
      onSuccess();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Grant failed. Try again.');
    } finally {
      setIsPending(false);
    }
  }, [
    canSubmit,
    promptMfa,
    token,
    capability,
    expiresAt,
    requiresReason,
    reason,
    userId,
    onSuccess,
  ]);

  // Close on ESC
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && !isPending) onClose();
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [isPending, onClose]);

  return (
    <>
      {/* ESC handler is on window above; backdrop click provides secondary dismiss */}
      <div className="cap-drawer-backdrop" onClick={!isPending ? onClose : undefined} />
      <aside className="cap-drawer" aria-label="Grant capability" role="dialog" aria-modal="true">
        <div className="cap-drawer__header">
          <h2 className="cap-drawer__title">Grant capability</h2>
          <button
            type="button"
            className="cap-drawer__close"
            onClick={onClose}
            disabled={isPending}
            aria-label="Close drawer"
          >
            <svg
              width="18"
              height="18"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
            >
              <line x1="18" y1="6" x2="6" y2="18" />
              <line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        </div>

        <div className="cap-drawer__body">
          {/* High-risk warning banner */}
          {isEscalate && (
            <div className="cap-warning">
              <svg
                width="16"
                height="16"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                style={{ flexShrink: 0, marginTop: 1 }}
              >
                <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z" />
                <line x1="12" y1="9" x2="12" y2="13" />
                <line x1="12" y1="17" x2="12.01" y2="17" />
              </svg>
              <span>
                <strong>High-risk grant:</strong> this gives platform-principal escalation. Ensure
                you have explicit approval before proceeding.
              </span>
            </div>
          )}

          {/* Capability selector */}
          <div className="cap-field">
            <label className="cap-field__label" htmlFor="cap-drawer-select">
              Capability
            </label>
            <select
              id="cap-drawer-select"
              className="cap-field__select"
              value={capability}
              onChange={(e) => {
                setCapability(e.target.value);
                setReason('');
                setError(null);
              }}
            >
              <option value="">Select a capability…</option>
              {CAPABILITIES.map((cap) => (
                <option key={cap} value={cap}>
                  {cap}
                </option>
              ))}
            </select>
          </div>

          {/* Audit reason (for high-risk caps) */}
          {requiresReason && reasonAction && (
            <AuditReasonPrompt
              action={reasonAction as never}
              value={reason}
              onChange={setReason}
              required
            />
          )}

          {/* Optional expires_at */}
          <div className="cap-field">
            <label className="cap-field__label" htmlFor="cap-drawer-expires">
              Expires at{' '}
              <span style={{ fontWeight: 400, color: 'var(--ppt-fg-muted, #6b7280)' }}>
                (optional — leave blank for never)
              </span>
            </label>
            <input
              id="cap-drawer-expires"
              type="datetime-local"
              className="cap-field__input"
              value={expiresAt}
              onChange={(e) => setExpiresAt(e.target.value)}
            />
          </div>

          {/* Inline error */}
          {error && <div className="cap-error-banner">{error}</div>}
        </div>

        <div className="cap-drawer__footer">
          <button
            type="button"
            className="cap-btn cap-btn--secondary"
            onClick={onClose}
            disabled={isPending}
          >
            Cancel
          </button>
          <button
            type="button"
            className="cap-btn cap-btn--primary"
            onClick={handleSubmit}
            disabled={!canSubmit}
          >
            {isPending && <span className="cap-spinner" />}
            {delayInProgress ? `Wait ${Math.ceil(delayRemaining / 1000)}s` : 'Grant capability'}
          </button>
        </div>
      </aside>
    </>
  );
}

// ---------------------------------------------------------------------------
// Page root
// ---------------------------------------------------------------------------

export default function CapabilitiesAdminPage() {
  ensureStyles();

  const [searchParams, setSearchParams] = useSearchParams();
  const view = searchParams.get('view') ?? 'registry';
  const userId = searchParams.get('id') ?? 'me';
  const { token } = useAdminAuth();

  const switchToUser = useCallback(
    (id: string) => {
      setSearchParams({ view: 'user', id });
    },
    [setSearchParams]
  );

  const switchToRegistry = useCallback(() => {
    setSearchParams({ view: 'registry' });
  }, [setSearchParams]);

  if (view === 'user') {
    return (
      <div className="cap-page">
        <UserView userId={userId} token={token} onSwitchToRegistry={switchToRegistry} />
      </div>
    );
  }

  return (
    <div className="cap-page">
      <div className="cap-page__header">
        <div>
          <h1 className="cap-page__title">Capabilities registry</h1>
          <p className="cap-page__subtitle">
            All {CAPABILITIES.length} platform capability strings. Click "View holders" to inspect
            grants for a specific user.
          </p>
        </div>
      </div>
      <RegistryView onViewUser={switchToUser} />
    </div>
  );
}
