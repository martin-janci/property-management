/**
 * Phase 5 — `/admin/feature-flags` page.
 *
 * Write-only-from-admin-console feature flag management. The form's onSubmit
 * opens an AuditReasonPrompt dialog (minLength=20). On confirm, fires
 * `POST /api/v1/platform-admin/feature-flags/{id}/toggle` for each toggled
 * flag. Flag key is used as the ID since the platform_admin endpoint expects
 * the flag ID in the path.
 *
 * If the toggle endpoint returns 404 for a given flag, shows a toast
 * "Save not yet implemented" for that flag and leaves a TODO comment below.
 */

import { type SettingsField, SettingsForm, useCapability } from '@ppt/admin-ui';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import type React from 'react';
import { useCallback, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useAdminAuth } from '../auth/AdminAuthContext';
import {
  type AuditReasonAction,
  AuditReasonPrompt,
  useAuditReasonValid,
} from '../components/AuditReasonPrompt';
import { useToast } from '../components/Toast';
import { useFocusTrap } from '../components/useFocusTrap';
import { HelpTooltip } from '../features/help';

interface FeatureFlagValues extends Record<string, unknown> {
  'beta.new_listings_search': boolean;
  'beta.ai_fault_triage': boolean;
  'building.disabled': boolean;
}

const FLAG_KEYS = [
  'beta.new_listings_search',
  'beta.ai_fault_triage',
  'building.disabled',
] as const;

const defaultValues: FeatureFlagValues = {
  'beta.new_listings_search': false,
  'beta.ai_fault_triage': false,
  'building.disabled': false,
};

interface BackendFeatureFlag {
  id: string;
  key: string;
  name?: string;
  is_enabled: boolean;
}

interface ListFeatureFlagsBackendResponse {
  flags: BackendFeatureFlag[];
}

// ---------------------------------------------------------------------------
// Per-flag override types (Epic 10B.2)
//
// GET /api/v1/platform-admin/feature-flags/{id} returns
//   { flag: { flag: FeatureFlag, overrides: FeatureFlagOverride[] } }
// (FeatureFlagDetailResponse wraps FeatureFlagWithOverrides). We only need the
// overrides + the inner flag's is_enabled for the panel.
// ---------------------------------------------------------------------------

type OverrideScopeType = 'organization' | 'user' | 'role';

const OVERRIDE_SCOPE_TYPES: readonly OverrideScopeType[] = ['organization', 'user', 'role'];

interface FeatureFlagOverride {
  id: string;
  flag_id: string;
  scope_type: string;
  scope_id: string;
  is_enabled: boolean;
  created_at: string;
}

interface FeatureFlagWithOverrides {
  flag: BackendFeatureFlag & { description?: string | null };
  overrides: FeatureFlagOverride[];
}

interface FeatureFlagDetailResponse {
  flag: FeatureFlagWithOverrides;
}

// ---------------------------------------------------------------------------
// AuditReason dialog — shown before actual API call
// ---------------------------------------------------------------------------

interface AuditReasonDialogProps {
  onConfirm: (reason: string) => void;
  onCancel: () => void;
  isPending: boolean;
  /** Dialog heading. Defaults to the global save-flags title. */
  title?: string;
  /** Audit action label forwarded to AuditReasonPrompt. */
  action?: AuditReasonAction;
  /** Confirm-button label. */
  confirmLabel?: string;
}

function AuditReasonDialog({
  onConfirm,
  onCancel,
  isPending,
  title = 'Save feature flags',
  action = 'feature_flag_toggle',
  confirmLabel = 'Save flags',
}: AuditReasonDialogProps) {
  const [reason, setReason] = useState('');
  const isValid = useAuditReasonValid(reason, 20);
  const dialogRef = useRef<HTMLDivElement | null>(null);
  const titleId = 'feature-flags-audit-dialog-title';

  useFocusTrap(dialogRef, () => {
    if (!isPending) onCancel();
  });

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
            color: 'var(--ppt-fg-primary, #111827)',
          }}
        >
          {title}
        </h2>
        <AuditReasonPrompt
          action={action}
          minLength={20}
          value={reason}
          onChange={setReason}
          required
        />
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
              background: 'var(--ppt-brand-600, #2563eb)',
              color: '#fff',
              cursor: 'pointer',
              fontSize: 13,
              fontWeight: 500,
              opacity: !isValid || isPending ? 0.45 : 1,
            }}
            disabled={!isValid || isPending}
            onClick={() => onConfirm(reason)}
          >
            {isPending ? 'Saving…' : confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Per-flag override panel (Epic 10B.2)
//
// Lists the flag's existing scope overrides and lets an operator add (via the
// audit-reason dialog) or remove targeted overrides. Reuses the same bearer +
// credentials:'include' fetch pattern as the rest of the page.
// ---------------------------------------------------------------------------

interface OverridesPanelProps {
  flag: BackendFeatureFlag;
}

function FeatureFlagOverridesPanel({ flag }: OverridesPanelProps) {
  const { t } = useTranslation();
  const { token } = useAdminAuth();
  const { showToast } = useToast();
  const queryClient = useQueryClient();

  const detailKey = useMemo(
    () => ['admin', 'platform', 'feature-flags', flag.id, 'detail'] as const,
    [flag.id]
  );

  const authHeaders = useCallback(
    (json = false): Record<string, string> => {
      const headers: Record<string, string> = {};
      if (json) headers['Content-Type'] = 'application/json';
      if (token) headers.Authorization = `Bearer ${token}`;
      return headers;
    },
    [token]
  );

  const detailQuery = useQuery<FeatureFlagWithOverrides>({
    queryKey: detailKey,
    queryFn: async () => {
      const res = await fetch(
        `/api/v1/platform-admin/feature-flags/${encodeURIComponent(flag.id)}`,
        { headers: authHeaders(), credentials: 'include' }
      );
      if (!res.ok) throw new Error(`Feature flag detail fetch failed: ${res.status}`);
      const body = (await res.json()) as FeatureFlagDetailResponse;
      return body.flag;
    },
    staleTime: 15_000,
    retry: 1,
  });

  // Add-override form state.
  const [scopeType, setScopeType] = useState<OverrideScopeType>('organization');
  const [scopeId, setScopeId] = useState('');
  const [isEnabled, setIsEnabled] = useState(true);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [pending, setPending] = useState(false);

  const invalidate = useCallback(async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ['admin', 'platform', 'feature-flags'] }),
      queryClient.invalidateQueries({ queryKey: detailKey }),
    ]);
  }, [queryClient, detailKey]);

  const handleAddConfirm = useCallback(
    async (_reason: string) => {
      setPending(true);
      try {
        const res = await fetch(
          `/api/v1/platform-admin/feature-flags/${encodeURIComponent(flag.id)}/overrides`,
          {
            method: 'POST',
            headers: authHeaders(true),
            credentials: 'include',
            body: JSON.stringify({
              scope_type: scopeType,
              scope_id: scopeId.trim(),
              is_enabled: isEnabled,
            }),
          }
        );
        if (res.status === 201 || res.ok) {
          setDialogOpen(false);
          setScopeId('');
          await invalidate();
          showToast({
            type: 'success',
            title: t('admin.featureFlags.overrides.toast.addedTitle', {
              defaultValue: 'Override added',
            }),
            message: t('admin.featureFlags.overrides.toast.addedMessage', {
              defaultValue: 'The feature flag override was created.',
            }),
          });
        } else {
          const text = await res.text().catch(() => '');
          showToast({
            type: 'error',
            title: t('admin.featureFlags.overrides.toast.addFailedTitle', {
              defaultValue: 'Failed to add override',
            }),
            message: text || `HTTP ${res.status}`,
          });
        }
      } catch (e) {
        showToast({
          type: 'error',
          title: t('admin.featureFlags.overrides.toast.addFailedTitle', {
            defaultValue: 'Failed to add override',
          }),
          message: e instanceof Error ? e.message : 'Network error',
        });
      } finally {
        setPending(false);
      }
    },
    [flag.id, authHeaders, scopeType, scopeId, isEnabled, invalidate, showToast, t]
  );

  const handleRemove = useCallback(
    async (overrideId: string) => {
      try {
        const res = await fetch(
          `/api/v1/platform-admin/feature-flags/${encodeURIComponent(
            flag.id
          )}/overrides/${encodeURIComponent(overrideId)}`,
          { method: 'DELETE', headers: authHeaders(), credentials: 'include' }
        );
        if (res.status === 204 || res.ok) {
          await invalidate();
          showToast({
            type: 'success',
            title: t('admin.featureFlags.overrides.toast.removedTitle', {
              defaultValue: 'Override removed',
            }),
            message: t('admin.featureFlags.overrides.toast.removedMessage', {
              defaultValue: 'The feature flag override was removed.',
            }),
          });
        } else {
          const text = await res.text().catch(() => '');
          showToast({
            type: 'error',
            title: t('admin.featureFlags.overrides.toast.removeFailedTitle', {
              defaultValue: 'Failed to remove override',
            }),
            message: text || `HTTP ${res.status}`,
          });
        }
      } catch (e) {
        showToast({
          type: 'error',
          title: t('admin.featureFlags.overrides.toast.removeFailedTitle', {
            defaultValue: 'Failed to remove override',
          }),
          message: e instanceof Error ? e.message : 'Network error',
        });
      }
    },
    [flag.id, authHeaders, invalidate, showToast, t]
  );

  const canSubmitAdd = scopeId.trim().length > 0 && !pending;
  const overrides = detailQuery.data?.overrides ?? [];

  return (
    <div
      style={{
        borderTop: '1px solid var(--ppt-border-default, #e5e7eb)',
        marginTop: 8,
        paddingTop: 12,
        display: 'flex',
        flexDirection: 'column',
        gap: 12,
      }}
    >
      {dialogOpen && (
        <AuditReasonDialog
          title={t('admin.featureFlags.overrides.dialogTitle', {
            defaultValue: 'Add feature flag override',
          })}
          confirmLabel={t('admin.featureFlags.overrides.addAction', {
            defaultValue: 'Add override',
          })}
          isPending={pending}
          onConfirm={handleAddConfirm}
          onCancel={() => setDialogOpen(false)}
        />
      )}

      <h3 style={{ margin: 0, fontSize: 14, fontWeight: 600 }}>
        {t('admin.featureFlags.overrides.title', { defaultValue: 'Per-scope overrides' })}
      </h3>

      {detailQuery.isLoading && (
        <div role="status" aria-live="polite">
          {t('admin.featureFlags.overrides.loading', { defaultValue: 'Loading overrides…' })}
        </div>
      )}

      {detailQuery.isError && (
        <div role="alert" className="ppt-admin-error">
          {t('admin.featureFlags.overrides.loadError', {
            defaultValue: 'Failed to load overrides.',
          })}
        </div>
      )}

      {!detailQuery.isLoading && !detailQuery.isError && overrides.length === 0 && (
        <p style={{ margin: 0, color: 'var(--ppt-fg-secondary, #6b7280)' }}>
          {t('admin.featureFlags.overrides.empty', {
            defaultValue: 'No overrides — this flag uses its global value everywhere.',
          })}
        </p>
      )}

      {overrides.length > 0 && (
        <table style={{ borderCollapse: 'collapse', width: '100%', fontSize: 13 }}>
          <thead>
            <tr style={{ textAlign: 'left' }}>
              <th style={{ padding: '4px 8px' }}>
                {t('admin.featureFlags.overrides.columns.scopeType', { defaultValue: 'Scope' })}
              </th>
              <th style={{ padding: '4px 8px' }}>
                {t('admin.featureFlags.overrides.columns.scopeId', { defaultValue: 'Scope ID' })}
              </th>
              <th style={{ padding: '4px 8px' }}>
                {t('admin.featureFlags.overrides.columns.enabled', { defaultValue: 'Enabled' })}
              </th>
              <th style={{ padding: '4px 8px' }} />
            </tr>
          </thead>
          <tbody>
            {overrides.map((o) => (
              <tr key={o.id} style={{ borderTop: '1px solid var(--ppt-border-default, #f1f5f9)' }}>
                <td style={{ padding: '4px 8px' }}>{o.scope_type}</td>
                <td style={{ padding: '4px 8px', fontFamily: 'monospace' }}>{o.scope_id}</td>
                <td style={{ padding: '4px 8px' }}>
                  {o.is_enabled
                    ? t('admin.featureFlags.overrides.on', { defaultValue: 'On' })
                    : t('admin.featureFlags.overrides.off', { defaultValue: 'Off' })}
                </td>
                <td style={{ padding: '4px 8px' }}>
                  <button
                    type="button"
                    onClick={() => void handleRemove(o.id)}
                    style={{
                      padding: '3px 10px',
                      borderRadius: 6,
                      border: '1px solid var(--ppt-border-default, #e5e7eb)',
                      background: 'transparent',
                      cursor: 'pointer',
                      fontSize: 12,
                    }}
                  >
                    {t('admin.featureFlags.overrides.removeAction', { defaultValue: 'Remove' })}
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      {/* Add-override form */}
      <form
        onSubmit={(e) => {
          e.preventDefault();
          if (canSubmitAdd) setDialogOpen(true);
        }}
        style={{ display: 'flex', flexWrap: 'wrap', alignItems: 'flex-end', gap: 10 }}
      >
        <label style={{ display: 'flex', flexDirection: 'column', gap: 4, fontSize: 12 }}>
          {t('admin.featureFlags.overrides.columns.scopeType', { defaultValue: 'Scope' })}
          <select
            value={scopeType}
            onChange={(e) => setScopeType(e.target.value as OverrideScopeType)}
            style={{ padding: '5px 8px' }}
          >
            {OVERRIDE_SCOPE_TYPES.map((s) => (
              <option key={s} value={s}>
                {t(`admin.featureFlags.overrides.scope.${s}`, { defaultValue: s })}
              </option>
            ))}
          </select>
        </label>
        <label style={{ display: 'flex', flexDirection: 'column', gap: 4, fontSize: 12 }}>
          {t('admin.featureFlags.overrides.columns.scopeId', { defaultValue: 'Scope ID' })}
          <input
            type="text"
            value={scopeId}
            onChange={(e) => setScopeId(e.target.value)}
            placeholder={t('admin.featureFlags.overrides.scopeIdPlaceholder', {
              defaultValue: 'UUID',
            })}
            style={{ padding: '5px 8px', minWidth: 260 }}
          />
        </label>
        <label style={{ display: 'flex', alignItems: 'center', gap: 6, fontSize: 12 }}>
          <input
            type="checkbox"
            checked={isEnabled}
            onChange={(e) => setIsEnabled(e.target.checked)}
          />
          {t('admin.featureFlags.overrides.columns.enabled', { defaultValue: 'Enabled' })}
        </label>
        <button
          type="submit"
          disabled={!canSubmitAdd}
          style={{
            padding: '6px 14px',
            borderRadius: 8,
            border: 'none',
            background: 'var(--ppt-brand-600, #2563eb)',
            color: '#fff',
            cursor: canSubmitAdd ? 'pointer' : 'not-allowed',
            fontSize: 13,
            fontWeight: 500,
            opacity: canSubmitAdd ? 1 : 0.45,
          }}
        >
          {t('admin.featureFlags.overrides.addAction', { defaultValue: 'Add override' })}
        </button>
      </form>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Flag list with expandable override panels (Epic 10B.2)
// ---------------------------------------------------------------------------

function FeatureFlagOverridesSection({ flags }: { flags: BackendFeatureFlag[] }) {
  const { t } = useTranslation();
  const canWrite = useCapability('feature_flags_write');
  const [expanded, setExpanded] = useState<string | null>(null);

  // Gate the whole panel behind feature_flags_write, mirroring the form below.
  if (!canWrite) return null;
  if (flags.length === 0) return null;

  return (
    <section style={{ marginTop: 32 }}>
      <h2 style={{ fontSize: 18, fontWeight: 600 }}>
        {t('admin.featureFlags.overrides.sectionTitle', {
          defaultValue: 'Targeted overrides',
        })}
      </h2>
      <p style={{ color: 'var(--ppt-fg-secondary, #6b7280)', marginTop: 4 }}>
        {t('admin.featureFlags.overrides.sectionHint', {
          defaultValue:
            'Enable or disable a flag for a specific organisation, user, or role. Targeted scopes take precedence over the global value.',
        })}
      </p>
      <ul
        style={{
          listStyle: 'none',
          padding: 0,
          margin: '16px 0 0',
          display: 'flex',
          flexDirection: 'column',
          gap: 8,
        }}
      >
        {flags.map((flag) => {
          const isOpen = expanded === flag.id;
          return (
            <li
              key={flag.id}
              style={{
                border: '1px solid var(--ppt-border-default, #e5e7eb)',
                borderRadius: 'var(--ppt-radius-lg, 10px)',
                padding: 12,
              }}
            >
              <button
                type="button"
                aria-expanded={isOpen}
                onClick={() => setExpanded(isOpen ? null : flag.id)}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                  width: '100%',
                  background: 'transparent',
                  border: 'none',
                  cursor: 'pointer',
                  fontSize: 14,
                  fontWeight: 500,
                  padding: 0,
                }}
              >
                <span>
                  {flag.name ?? flag.key}{' '}
                  <code style={{ color: 'var(--ppt-fg-secondary, #6b7280)', fontWeight: 400 }}>
                    {flag.key}
                  </code>
                </span>
                <span aria-hidden>{isOpen ? '▾' : '▸'}</span>
              </button>
              {isOpen && <FeatureFlagOverridesPanel flag={flag} />}
            </li>
          );
        })}
      </ul>
    </section>
  );
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

const FeatureFlagsPage: React.FC = () => {
  const { t } = useTranslation();
  const { token } = useAdminAuth();
  const { showToast } = useToast();

  // Load real flag metadata. The backend toggle endpoint takes a UUID id, not
  // the human-readable key, so we need this list to resolve key → id and to
  // get the current `is_enabled` baseline used for diffing.
  const { data: backendFlags } = useQuery<BackendFeatureFlag[]>({
    queryKey: ['admin', 'platform', 'feature-flags'],
    queryFn: async () => {
      const headers: Record<string, string> = {};
      if (token) headers.Authorization = `Bearer ${token}`;
      const res = await fetch('/api/v1/platform-admin/feature-flags', {
        headers,
        credentials: 'include',
      });
      if (!res.ok) {
        if (res.status === 404) return [];
        throw new Error(`Feature flags fetch failed: ${res.status}`);
      }
      const body = (await res.json()) as ListFeatureFlagsBackendResponse;
      return body.flags ?? [];
    },
    staleTime: 30_000,
    retry: 1,
  });

  // Index by key for O(1) lookup of {id, is_enabled} during save.
  const flagsByKey = useMemo(() => {
    const map = new Map<string, BackendFeatureFlag>();
    for (const f of backendFlags ?? []) map.set(f.key, f);
    return map;
  }, [backendFlags]);

  // Form initial values derived from the server's current is_enabled, falling
  // back to defaults for keys the server doesn't yet know about.
  const initialValues = useMemo<FeatureFlagValues>(() => {
    const result = { ...defaultValues };
    for (const k of FLAG_KEYS) {
      const found = flagsByKey.get(k);
      if (found) (result as Record<string, unknown>)[k] = found.is_enabled;
    }
    return result;
  }, [flagsByKey]);

  // Pending values captured from SettingsForm so we can pass to dialog
  const pendingValuesRef = useRef<FeatureFlagValues | null>(null);
  // Resolve/reject for the Promise returned by onSubmit
  const resolveSubmitRef = useRef<(() => void) | null>(null);
  const rejectSubmitRef = useRef<((err: Error) => void) | null>(null);

  const [dialogOpen, setDialogOpen] = useState(false);
  const [dialogPending, setDialogPending] = useState(false);

  const fields = useMemo<ReadonlyArray<SettingsField>>(
    () => [
      {
        kind: 'boolean',
        key: 'beta.new_listings_search',
        label: t('admin.featureFlags.fields.newListingsSearch'),
      },
      {
        kind: 'boolean',
        key: 'beta.ai_fault_triage',
        label: t('admin.featureFlags.fields.aiFaultTriage'),
      },
      {
        kind: 'boolean',
        key: 'building.disabled',
        label: t('admin.featureFlags.fields.buildingDisabled'),
      },
    ],
    [t]
  );

  const handleSubmit = useCallback((values: FeatureFlagValues): Promise<void> => {
    pendingValuesRef.current = values;
    setDialogOpen(true);
    return new Promise<void>((resolve, reject) => {
      resolveSubmitRef.current = resolve;
      rejectSubmitRef.current = reject;
    });
  }, []);

  const handleConfirm = useCallback(
    async (_reason: string) => {
      const values = pendingValuesRef.current;
      if (!values) return;
      setDialogPending(true);
      const headers: Record<string, string> = { 'Content-Type': 'application/json' };
      if (token) headers.Authorization = `Bearer ${token}`;

      // Diff submitted values against the server baseline; only call toggle
      // for flags that actually changed. Avoids needless writes (and needless
      // audit-log rows) for flags the operator left untouched.
      const changedKeys = FLAG_KEYS.filter((k) => {
        const desired = values[k] === true;
        const current = flagsByKey.get(k)?.is_enabled ?? defaultValues[k];
        return desired !== current;
      });

      if (changedKeys.length === 0) {
        setDialogPending(false);
        setDialogOpen(false);
        showToast({
          type: 'info',
          title: t('admin.featureFlags.toast.noChangesTitle', { defaultValue: 'No changes' }),
          message: t('admin.featureFlags.toast.noChangesMessage', {
            defaultValue: 'No feature flags were modified.',
          }),
        });
        resolveSubmitRef.current?.();
        return;
      }

      let anySuccess = false;
      let anyFailure = false;
      const unknownKeys: string[] = [];

      for (const flagKey of changedKeys) {
        const meta = flagsByKey.get(flagKey);
        if (!meta) {
          // Flag isn't registered on the server yet — skip and warn.
          unknownKeys.push(flagKey);
          anyFailure = true;
          continue;
        }
        try {
          const res = await fetch(
            `/api/v1/platform-admin/feature-flags/${encodeURIComponent(meta.id)}/toggle`,
            {
              method: 'POST',
              headers,
              credentials: 'include',
            }
          );
          if (!res.ok) {
            anyFailure = true;
          } else {
            anySuccess = true;
          }
        } catch {
          anyFailure = true;
        }
      }

      setDialogPending(false);
      setDialogOpen(false);

      if (unknownKeys.length > 0) {
        showToast({
          type: 'warning',
          title: 'Unknown feature flag(s)',
          message: `Not registered on server: ${unknownKeys.join(', ')}. Create them via POST /platform-admin/feature-flags first.`,
        });
      }

      if (anySuccess && !anyFailure) {
        showToast({
          type: 'success',
          title: t('admin.featureFlags.toast.savedTitle', { defaultValue: 'Flags saved' }),
          message: t('admin.featureFlags.toast.savedMessage', {
            defaultValue: 'Feature flags updated successfully.',
          }),
        });
        resolveSubmitRef.current?.();
      } else if (anyFailure && anySuccess) {
        showToast({
          type: 'warning',
          title: t('admin.featureFlags.toast.partialFailedTitle', {
            defaultValue: 'Partial save',
          }),
          message: t('admin.featureFlags.toast.partialFailedMessage', {
            defaultValue: 'Some flags were saved; others could not be saved.',
          }),
        });
        rejectSubmitRef.current?.(new Error('Partial save — some flags failed.'));
      } else if (anyFailure) {
        showToast({
          type: 'error',
          title: t('admin.featureFlags.toast.saveFailedTitle', { defaultValue: 'Save failed' }),
          message: t('admin.featureFlags.toast.saveFailedMessage', {
            defaultValue: 'One or more flags could not be saved.',
          }),
        });
        rejectSubmitRef.current?.(new Error('One or more flags failed to save.'));
      } else {
        resolveSubmitRef.current?.();
      }
    },
    [token, showToast, t, flagsByKey]
  );

  // Cancel resolves the form's submit Promise instead of rejecting it, so the
  // form treats the cancel as a clean no-op rather than rendering "Cancelled"
  // as a form-level error.
  const handleDialogCancel = useCallback(() => {
    setDialogOpen(false);
    resolveSubmitRef.current?.();
  }, []);

  return (
    <>
      {dialogOpen && (
        <AuditReasonDialog
          onConfirm={handleConfirm}
          onCancel={handleDialogCancel}
          isPending={dialogPending}
        />
      )}
      <section>
        <h1 style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          {t('admin.featureFlags.title')}
          <HelpTooltip text={t('admin.featureFlags.helpTooltip')} />
        </h1>
        <p role="alert" className="ppt-admin-warning" style={{ color: '#b00020', fontWeight: 600 }}>
          {t('admin.featureFlags.warning')}
        </p>
        <SettingsForm<FeatureFlagValues>
          // key forces re-init of the SettingsForm internal state once the
          // server-provided baseline arrives, so the toggles reflect actual
          // server values instead of always-false defaults.
          key={`ff-${(backendFlags ?? []).length}`}
          fields={fields}
          initialValues={initialValues}
          capability="feature_flags_write"
          onSubmit={handleSubmit}
        />
        <FeatureFlagOverridesSection flags={backendFlags ?? []} />
      </section>
    </>
  );
};

export default FeatureFlagsPage;
