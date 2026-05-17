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

import { type SettingsField, SettingsForm } from '@ppt/admin-ui';
import { useQuery } from '@tanstack/react-query';
import type React from 'react';
import { useCallback, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useAdminAuth } from '../auth/AdminAuthContext';
import { AuditReasonPrompt, useAuditReasonValid } from '../components/AuditReasonPrompt';
import { useToast } from '../components/Toast';
import { useFocusTrap } from '../components/useFocusTrap';

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
// AuditReason dialog — shown before actual API call
// ---------------------------------------------------------------------------

interface AuditReasonDialogProps {
  onConfirm: (reason: string) => void;
  onCancel: () => void;
  isPending: boolean;
}

function AuditReasonDialog({ onConfirm, onCancel, isPending }: AuditReasonDialogProps) {
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
          Save feature flags
        </h2>
        <AuditReasonPrompt
          action="feature_flag_toggle"
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
            {isPending ? 'Saving…' : 'Save flags'}
          </button>
        </div>
      </div>
    </div>
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
        <h1>{t('admin.featureFlags.title')}</h1>
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
      </section>
    </>
  );
};

export default FeatureFlagsPage;
