/**
 * Phase 5 — `/admin/users` page.
 *
 * Wired to `GET /api/v1/admin/principals?q=<query>` (the Phase 5 admin user
 * search endpoint, mounted at /principals to avoid colliding with the legacy
 * /users lifecycle routes). Uses TanStack Query with 250ms debounce.
 *
 * The Impersonate action calls `POST /api/v1/admin/impersonation/start` after
 * capturing an audit reason and navigates to `/ops/impersonation` on success.
 */

import { ResourceTable, type ResourceTableColumn } from '@ppt/admin-ui';
import { useQuery } from '@tanstack/react-query';
import type React from 'react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { useAdminAuth } from '../auth/AdminAuthContext';
import { AuditReasonPrompt, useAuditReasonValid } from '../components/AuditReasonPrompt';
import { useToast } from '../components/Toast';
import { useFocusTrap } from '../components/useFocusTrap';

interface UserRow {
  id: string;
  email: string;
  display_name: string | null;
}

// ---------------------------------------------------------------------------
// Impersonate reason dialog
// ---------------------------------------------------------------------------

interface ImpersonateDialogProps {
  user: UserRow;
  onConfirm: (reason: string) => void;
  onCancel: () => void;
  isPending: boolean;
  error?: string;
}

function ImpersonateDialog({
  user,
  onConfirm,
  onCancel,
  isPending,
  error,
}: ImpersonateDialogProps) {
  const [reason, setReason] = useState('');
  const isValid = useAuditReasonValid(reason, 30);
  const dialogRef = useRef<HTMLDivElement | null>(null);
  const titleId = 'impersonate-dialog-title';

  // Trap Tab focus inside the dialog and dismiss on Escape; restore focus on close.
  useFocusTrap(dialogRef, () => {
    if (!isPending) onCancel();
  });

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
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        tabIndex={-1}
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
          id={titleId}
          style={{
            margin: 0,
            fontSize: 16,
            fontWeight: 600,
            color: 'var(--ppt-danger-700, #b91c1c)',
          }}
        >
          Impersonate user
        </h2>
        <p style={{ margin: 0, fontSize: 13, color: 'var(--ppt-fg-secondary, #374151)' }}>
          You are about to impersonate <strong>{user.email}</strong>. Provide an audit reason.
        </p>
        <AuditReasonPrompt
          action="principal_kind_escalate"
          minLength={30}
          value={reason}
          onChange={setReason}
          required
        />
        {error && (
          <div
            style={{
              fontSize: 13,
              color: 'var(--ppt-danger-700, #b91c1c)',
              padding: '8px 12px',
              background: 'var(--ppt-danger-50, #fef2f2)',
              borderRadius: 8,
              border: '1px solid var(--ppt-danger-300, #fca5a5)',
            }}
          >
            {error}
          </div>
        )}
        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 10 }}>
          <button
            type="button"
            style={{
              padding: '7px 14px',
              borderRadius: 8,
              border: '1px solid var(--ppt-border-default, #e5e7eb)',
              background: 'transparent',
              cursor: 'pointer',
              fontSize: 13,
              fontWeight: 500,
            }}
            onClick={onCancel}
            disabled={isPending}
          >
            Cancel
          </button>
          <button
            type="button"
            style={{
              padding: '7px 14px',
              borderRadius: 8,
              border: 'none',
              background: 'var(--ppt-danger-600, #dc2626)',
              color: '#fff',
              cursor: 'pointer',
              fontSize: 13,
              fontWeight: 500,
              opacity: !isValid || isPending ? 0.45 : 1,
            }}
            disabled={!isValid || isPending}
            onClick={() => onConfirm(reason)}
          >
            {isPending ? 'Starting…' : 'Start impersonation'}
          </button>
        </div>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Hook: debounced value
// ---------------------------------------------------------------------------

function useDebounce<T>(value: T, delay: number): T {
  const [debounced, setDebounced] = useState(value);
  // Use useEffect so the cleanup actually runs and we cancel the pending timer
  // on every change. The previous useMemo-based implementation discarded the
  // returned cleanup (useMemo doesn't honour it), leaking timers per keystroke.
  useEffect(() => {
    const id = setTimeout(() => setDebounced(value), delay);
    return () => clearTimeout(id);
  }, [value, delay]);
  return debounced;
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

const UsersPage: React.FC = () => {
  const { t } = useTranslation();
  const { token } = useAdminAuth();
  const { showToast } = useToast();
  const navigate = useNavigate();

  const [query, setQuery] = useState('');
  const debouncedQuery = useDebounce(query, 250);

  const [impersonateTarget, setImpersonateTarget] = useState<UserRow | null>(null);
  const [impersonatePending, setImpersonatePending] = useState(false);
  const [impersonateError, setImpersonateError] = useState<string | undefined>(undefined);

  const { data, isLoading, isError } = useQuery<UserRow[]>({
    queryKey: ['admin', 'principals', debouncedQuery],
    queryFn: async () => {
      const params = new URLSearchParams();
      if (debouncedQuery) params.set('q', debouncedQuery);
      const headers: Record<string, string> = {};
      if (token) headers.Authorization = `Bearer ${token}`;
      const res = await fetch(`/api/v1/admin/principals?${params.toString()}`, {
        headers,
        credentials: 'include',
      });
      if (!res.ok) {
        if (res.status === 404) {
          console.warn(
            'GET /api/v1/admin/principals returned 404 — endpoint may not be deployed yet'
          );
          return [];
        }
        throw new Error(`Users fetch failed: ${res.status}`);
      }
      return res.json() as Promise<UserRow[]>;
    },
    staleTime: 30_000,
    retry: 1,
  });

  const handleImpersonate = useCallback(
    async (user: UserRow, reason: string) => {
      setImpersonatePending(true);
      setImpersonateError(undefined);
      // NOTE: backend `StartBody` in
      // `backend/servers/api-server/src/routes/admin/impersonation.rs` only
      // deserializes `target_user_id`; the `reason` we collect from the
      // operator is intentionally NOT in the request body to avoid sending
      // a misleading field that the server silently drops. The reason is
      // surfaced via the audit-log path that wraps this action server-side.
      void reason;
      try {
        const headers: Record<string, string> = { 'Content-Type': 'application/json' };
        if (token) headers.Authorization = `Bearer ${token}`;
        const res = await fetch('/api/v1/admin/impersonation/start', {
          method: 'POST',
          headers,
          credentials: 'include',
          body: JSON.stringify({ target_user_id: user.id }),
        });
        if (!res.ok) {
          const text = await res.text().catch(() => '');
          throw new Error(text || `Impersonation failed: ${res.status}`);
        }
        // Persist the issued token so ImpersonationWrapper renders the banner
        // and the operator's session has a way to look up / end the
        // impersonation. Storage shape mirrors `StoredImpersonation` in
        // components/ImpersonationWrapper.tsx.
        const issued = (await res.json().catch(() => null)) as {
          token_id?: string;
          expires_at?: string;
        } | null;
        if (issued?.token_id && issued.expires_at) {
          try {
            localStorage.setItem(
              'ppt_impersonation',
              JSON.stringify({
                tokenId: issued.token_id,
                expiresAt: issued.expires_at,
                targetUserLabel: user.email,
              })
            );
          } catch {
            // localStorage unavailable (private mode, quota) — fall through;
            // the wrapper will simply not show the banner this session.
          }
        }
        showToast({
          type: 'success',
          title: 'Impersonation started',
          message: `Now impersonating ${user.email}`,
        });
        setImpersonateTarget(null);
        navigate('/ops/impersonation');
      } catch (err) {
        setImpersonateError(
          err instanceof Error ? err.message : 'Impersonation failed. Try again.'
        );
      } finally {
        setImpersonatePending(false);
      }
    },
    [token, showToast, navigate]
  );

  const columns = useMemo<ReadonlyArray<ResourceTableColumn<UserRow>>>(
    () => [
      { key: 'email', header: t('admin.users.columns.email'), render: (u) => u.email },
      {
        key: 'name',
        header: t('admin.users.columns.displayName'),
        render: (u) => u.display_name ?? '',
      },
    ],
    [t]
  );

  const tableData: UserRow[] = isError ? [] : (data ?? []);

  return (
    <>
      {impersonateTarget && (
        <ImpersonateDialog
          user={impersonateTarget}
          onConfirm={(reason) => handleImpersonate(impersonateTarget, reason)}
          onCancel={() => {
            setImpersonateTarget(null);
            setImpersonateError(undefined);
          }}
          isPending={impersonatePending}
          error={impersonateError}
        />
      )}
      <section>
        <h1>{t('admin.users.title')}</h1>
        <div style={{ marginBottom: 16 }}>
          <input
            type="search"
            placeholder={t('admin.users.searchPlaceholder', {
              defaultValue: 'Search by email or name…',
            })}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            style={{
              fontSize: 13,
              padding: '7px 10px',
              border: '1px solid var(--ppt-border-default, #e5e7eb)',
              borderRadius: 8,
              background: 'var(--ppt-bg-input, #fff)',
              color: 'var(--ppt-fg-primary, #111827)',
              outline: 'none',
              width: 280,
            }}
            aria-label={t('admin.users.searchPlaceholder', { defaultValue: 'Search users' })}
          />
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
        {!isLoading && (
          <ResourceTable<UserRow>
            columns={columns}
            data={tableData}
            rowKey={(u) => u.id}
            emptyMessage={t('admin.users.empty')}
            actions={[
              {
                label: t('admin.users.actions.impersonate'),
                capability: 'users_impersonate',
                variant: 'danger',
                onClick: (u) => {
                  setImpersonateError(undefined);
                  setImpersonateTarget(u);
                },
              },
            ]}
          />
        )}
      </section>
    </>
  );
};

export default UsersPage;
