/**
 * ImpersonationListPage — `/ops/impersonation`
 *
 * Renders active impersonation sessions from
 * GET /api/v1/admin/impersonation/active (falls back to empty list on 404).
 *
 * Stop button calls DELETE /api/v1/admin/impersonation/{token_id}.
 * Gate: users_impersonate capability.
 */

import { useCapability } from '@ppt/admin-ui';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { useAdminAuth } from '../auth/AdminAuthContext';
import { useToast } from '../components/Toast';
import { HelpTooltip } from '../features/help';

interface ActiveSession {
  token_id: string;
  target_user_id: string;
  target_email: string;
  started_by: string;
  expires_at: string;
}

interface ActiveSessionsResponse {
  items: ActiveSession[];
}

function formatDate(iso: string): string {
  try {
    return new Intl.DateTimeFormat('en-GB', { dateStyle: 'short', timeStyle: 'short' }).format(
      new Date(iso)
    );
  } catch {
    return iso;
  }
}

export default function ImpersonationListPage() {
  const { t } = useTranslation();
  const { token } = useAdminAuth();
  const { showToast } = useToast();
  const queryClient = useQueryClient();
  const canImpersonate = useCapability('users_impersonate');

  const { data, isLoading, isError, error } = useQuery<ActiveSessionsResponse>({
    queryKey: ['admin', 'impersonation', 'active'],
    queryFn: async () => {
      const headers: Record<string, string> = {};
      if (token) headers.Authorization = `Bearer ${token}`;
      const res = await fetch('/api/v1/admin/impersonation/active', {
        headers,
        credentials: 'include',
      });
      if (!res.ok) {
        throw new Error(`Failed to load impersonation sessions: ${res.status}`);
      }
      return res.json() as Promise<ActiveSessionsResponse>;
    },
    staleTime: 15_000,
    retry: 1,
    refetchInterval: 30_000,
  });

  const stopMutation = useMutation({
    mutationFn: async (tokenId: string) => {
      const headers: Record<string, string> = {};
      if (token) headers.Authorization = `Bearer ${token}`;
      const res = await fetch(`/api/v1/admin/impersonation/${tokenId}`, {
        method: 'DELETE',
        headers,
        credentials: 'include',
      });
      if (!res.ok) throw new Error(`Stop impersonation failed: ${res.status}`);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['admin', 'impersonation', 'active'] });
      showToast({
        type: 'success',
        title: t('admin.impersonation.toast.stoppedTitle', { defaultValue: 'Session stopped' }),
        message: t('admin.impersonation.toast.stoppedMessage', {
          defaultValue: 'Impersonation session ended.',
        }),
      });
    },
    onError: (err) => {
      showToast({
        type: 'error',
        title: t('admin.impersonation.toast.stopFailedTitle', { defaultValue: 'Stop failed' }),
        message: err instanceof Error ? err.message : 'Unknown error',
      });
    },
  });

  const items = data?.items ?? [];

  return (
    <section style={{ padding: '24px 32px' }}>
      <h1
        style={{
          fontSize: '1.375rem',
          fontWeight: 700,
          color: 'var(--ppt-fg-primary, #111827)',
          margin: '0 0 20px',
          display: 'flex',
          alignItems: 'center',
          gap: 8,
        }}
      >
        {t('admin.impersonation.title', { defaultValue: 'Active Impersonation Sessions' })}
        <HelpTooltip text={t('admin.impersonation.helpTooltip')} />
      </h1>

      {isLoading && (
        <div
          role="status"
          aria-live="polite"
          style={{ color: 'var(--ppt-fg-muted, #6b7280)', fontSize: 13 }}
        >
          {t('admin.common.loading', { defaultValue: 'Loading…' })}
        </div>
      )}

      {isError && (
        <div
          role="alert"
          style={{
            color: 'var(--ppt-danger-700, #b91c1c)',
            fontSize: 13,
            padding: '10px 14px',
            background: 'var(--ppt-danger-50, #fef2f2)',
            borderRadius: 8,
            border: '1px solid var(--ppt-danger-300, #fca5a5)',
          }}
        >
          {error instanceof Error
            ? error.message
            : t('admin.impersonation.loadError', { defaultValue: 'Failed to load sessions.' })}
        </div>
      )}

      {!isLoading && !isError && (
        <div
          style={{
            overflowX: 'auto',
            background: 'var(--ppt-bg-surface, #fff)',
            border: '1px solid var(--ppt-border-default, #e5e7eb)',
            borderRadius: 12,
          }}
        >
          <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 13 }}>
            <thead style={{ background: 'var(--ppt-bg-subtle, #f3f4f6)' }}>
              <tr>
                {[
                  t('admin.impersonation.columns.targetUser', { defaultValue: 'Target user' }),
                  t('admin.impersonation.columns.startedBy', { defaultValue: 'Started by' }),
                  t('admin.impersonation.columns.expiresAt', { defaultValue: 'Expires at' }),
                  ...(canImpersonate ? [''] : []),
                ].map((header) => (
                  <th
                    key={header}
                    style={{
                      padding: '10px 14px',
                      textAlign: 'left',
                      fontSize: 11,
                      fontWeight: 600,
                      textTransform: 'uppercase',
                      letterSpacing: '0.06em',
                      color: 'var(--ppt-fg-muted, #6b7280)',
                      borderBottom: '1px solid var(--ppt-border-default, #e5e7eb)',
                      whiteSpace: 'nowrap',
                    }}
                  >
                    {header}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {items.length === 0 && (
                <tr>
                  <td
                    colSpan={canImpersonate ? 4 : 3}
                    style={{
                      padding: '24px',
                      textAlign: 'center',
                      color: 'var(--ppt-fg-muted, #6b7280)',
                    }}
                  >
                    {t('admin.impersonation.empty', {
                      defaultValue: 'No active impersonation sessions.',
                    })}
                  </td>
                </tr>
              )}
              {items.map((session) => (
                <tr key={session.token_id}>
                  <td
                    style={{
                      padding: '10px 14px',
                      borderBottom: '1px solid var(--ppt-border-subtle, #f3f4f6)',
                    }}
                  >
                    {session.target_email}
                  </td>
                  <td
                    style={{
                      padding: '10px 14px',
                      borderBottom: '1px solid var(--ppt-border-subtle, #f3f4f6)',
                    }}
                  >
                    {session.started_by}
                  </td>
                  <td
                    style={{
                      padding: '10px 14px',
                      borderBottom: '1px solid var(--ppt-border-subtle, #f3f4f6)',
                      whiteSpace: 'nowrap',
                    }}
                  >
                    {formatDate(session.expires_at)}
                  </td>
                  {canImpersonate && (
                    <td
                      style={{
                        padding: '10px 14px',
                        borderBottom: '1px solid var(--ppt-border-subtle, #f3f4f6)',
                      }}
                    >
                      <button
                        type="button"
                        disabled={stopMutation.isPending}
                        onClick={() => stopMutation.mutate(session.token_id)}
                        style={{
                          padding: '4px 10px',
                          borderRadius: 8,
                          border: '1px solid var(--ppt-danger-300, #fca5a5)',
                          background: 'transparent',
                          color: 'var(--ppt-danger-600, #dc2626)',
                          cursor: 'pointer',
                          fontSize: 12,
                          fontWeight: 500,
                        }}
                      >
                        {t('admin.impersonation.actions.stop', { defaultValue: 'Stop' })}
                      </button>
                    </td>
                  )}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  );
}
