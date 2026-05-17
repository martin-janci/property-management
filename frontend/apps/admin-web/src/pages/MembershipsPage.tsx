/**
 * MembershipsPage — `/identity/memberships`
 *
 * Renders a table of memberships for a given org. Org is selected by slug
 * via a text input + Load button.
 *
 * Wired to:
 *   GET  /api/v1/admin/memberships?org_id=<id>  (uses org slug lookup stub)
 *   DELETE /api/v1/admin/memberships/{user_id}   → requires MFA + memberships_revoke cap
 */

import { useCapability } from '@ppt/admin-ui';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useAdminAuth } from '../auth/AdminAuthContext';
import { useToast } from '../components/Toast';

interface MembershipRow {
  user_id: string;
  email: string;
  role: string;
  invited_at: string;
  status: string;
}

interface MembershipsResponse {
  items: MembershipRow[];
  total: number;
}

function formatDate(iso: string): string {
  try {
    return new Intl.DateTimeFormat('en-GB', { dateStyle: 'short' }).format(new Date(iso));
  } catch {
    return iso;
  }
}

export default function MembershipsPage() {
  const { t } = useTranslation();
  const { token } = useAdminAuth();
  const { showToast } = useToast();
  const queryClient = useQueryClient();
  const canRevoke = useCapability('memberships_revoke');

  const [orgSlug, setOrgSlug] = useState('');
  const [loadedSlug, setLoadedSlug] = useState('');

  const { data, isLoading, isError, error } = useQuery<MembershipsResponse>({
    queryKey: ['admin', 'memberships', loadedSlug],
    queryFn: async () => {
      if (!loadedSlug) return { items: [], total: 0 };
      const headers: Record<string, string> = {};
      if (token) headers.Authorization = `Bearer ${token}`;
      const params = new URLSearchParams({ org_id: loadedSlug });
      const res = await fetch(`/api/v1/admin/memberships?${params.toString()}`, {
        headers,
        credentials: 'include',
      });
      if (!res.ok) {
        if (res.status === 404) {
          console.warn(
            'GET /api/v1/admin/memberships returned 404 — endpoint may not be available'
          );
          return { items: [], total: 0 };
        }
        throw new Error(`Memberships fetch failed: ${res.status}`);
      }
      return res.json() as Promise<MembershipsResponse>;
    },
    enabled: loadedSlug !== '',
    staleTime: 30_000,
    retry: 1,
  });

  const revokeMutation = useMutation({
    mutationFn: async (userId: string) => {
      const headers: Record<string, string> = { 'X-MFA-Recent': '1' };
      if (token) headers.Authorization = `Bearer ${token}`;
      const res = await fetch(`/api/v1/admin/memberships/${userId}`, {
        method: 'DELETE',
        headers,
        credentials: 'include',
      });
      if (!res.ok) throw new Error(`Revoke failed: ${res.status}`);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['admin', 'memberships', loadedSlug] });
      showToast({
        type: 'success',
        title: t('admin.memberships.toast.revokedTitle', { defaultValue: 'Membership revoked' }),
        message: t('admin.memberships.toast.revokedMessage', {
          defaultValue: 'The membership has been revoked.',
        }),
      });
    },
    onError: (err) => {
      showToast({
        type: 'error',
        title: t('admin.memberships.toast.revokeFailedTitle', { defaultValue: 'Revoke failed' }),
        message: err instanceof Error ? err.message : 'Unknown error',
      });
    },
  });

  const handleLoad = useCallback(() => {
    const slug = orgSlug.trim();
    if (slug) setLoadedSlug(slug);
  }, [orgSlug]);

  const items = data?.items ?? [];

  return (
    <section style={{ padding: '24px 32px' }}>
      <h1
        style={{
          fontSize: '1.375rem',
          fontWeight: 700,
          color: 'var(--ppt-fg-primary, #111827)',
          margin: '0 0 20px',
        }}
      >
        {t('admin.memberships.title', { defaultValue: 'Memberships' })}
      </h1>

      {/* Org selector */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 20 }}>
        <label
          htmlFor="memberships-org-slug"
          style={{ fontSize: 13, fontWeight: 500, color: 'var(--ppt-fg-secondary, #374151)' }}
        >
          {t('admin.memberships.orgSlugLabel', { defaultValue: 'Organisation slug' })}
        </label>
        <input
          id="memberships-org-slug"
          type="text"
          value={orgSlug}
          onChange={(e) => setOrgSlug(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') handleLoad();
          }}
          placeholder={t('admin.memberships.orgSlugPlaceholder', {
            defaultValue: 'e.g. dunajska-residences',
          })}
          style={{
            fontSize: 13,
            padding: '6px 10px',
            border: '1px solid var(--ppt-border-default, #e5e7eb)',
            borderRadius: 8,
            background: 'var(--ppt-bg-input, #fff)',
            color: 'var(--ppt-fg-primary, #111827)',
            outline: 'none',
            width: 260,
          }}
        />
        <button
          type="button"
          onClick={handleLoad}
          style={{
            padding: '6px 14px',
            borderRadius: 8,
            border: '1px solid var(--ppt-border-default, #e5e7eb)',
            background: 'transparent',
            cursor: 'pointer',
            fontSize: 13,
            fontWeight: 500,
          }}
        >
          {t('admin.common.load', { defaultValue: 'Load' })}
        </button>
      </div>

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
            : t('admin.memberships.loadError', { defaultValue: 'Failed to load memberships.' })}
        </div>
      )}

      {!isLoading && !isError && loadedSlug && (
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
                  t('admin.memberships.columns.user', { defaultValue: 'User' }),
                  t('admin.memberships.columns.role', { defaultValue: 'Role' }),
                  t('admin.memberships.columns.invitedAt', { defaultValue: 'Invited at' }),
                  t('admin.memberships.columns.status', { defaultValue: 'Status' }),
                  ...(canRevoke ? [''] : []),
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
                    colSpan={canRevoke ? 5 : 4}
                    style={{
                      padding: '24px',
                      textAlign: 'center',
                      color: 'var(--ppt-fg-muted, #6b7280)',
                    }}
                  >
                    {t('admin.memberships.empty', { defaultValue: 'No memberships found.' })}
                  </td>
                </tr>
              )}
              {items.map((row) => (
                <tr key={row.user_id}>
                  <td
                    style={{
                      padding: '10px 14px',
                      borderBottom: '1px solid var(--ppt-border-subtle, #f3f4f6)',
                    }}
                  >
                    {row.email}
                  </td>
                  <td
                    style={{
                      padding: '10px 14px',
                      borderBottom: '1px solid var(--ppt-border-subtle, #f3f4f6)',
                    }}
                  >
                    {row.role}
                  </td>
                  <td
                    style={{
                      padding: '10px 14px',
                      borderBottom: '1px solid var(--ppt-border-subtle, #f3f4f6)',
                      whiteSpace: 'nowrap',
                    }}
                  >
                    {formatDate(row.invited_at)}
                  </td>
                  <td
                    style={{
                      padding: '10px 14px',
                      borderBottom: '1px solid var(--ppt-border-subtle, #f3f4f6)',
                    }}
                  >
                    {row.status}
                  </td>
                  {canRevoke && (
                    <td
                      style={{
                        padding: '10px 14px',
                        borderBottom: '1px solid var(--ppt-border-subtle, #f3f4f6)',
                      }}
                    >
                      <button
                        type="button"
                        disabled={revokeMutation.isPending}
                        onClick={() => revokeMutation.mutate(row.user_id)}
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
                        {t('admin.memberships.actions.revoke', { defaultValue: 'Revoke' })}
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
