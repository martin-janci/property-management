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
import type React from 'react';
import { useCallback, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useAdminAuth } from '../auth/AdminAuthContext';
import { AuditReasonPrompt, useAuditReasonValid } from '../components/AuditReasonPrompt';
import { useToast } from '../components/Toast';

interface FeatureFlagValues extends Record<string, unknown> {
  'beta.new_listings_search': boolean;
  'beta.ai_fault_triage': boolean;
  'building.disabled': boolean;
}

const initialValues: FeatureFlagValues = {
  'beta.new_listings_search': false,
  'beta.ai_fault_triage': false,
  'building.disabled': false,
};

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
    async (reason: string) => {
      const values = pendingValuesRef.current;
      if (!values) return;
      setDialogPending(true);
      const headers: Record<string, string> = { 'Content-Type': 'application/json' };
      if (token) headers.Authorization = `Bearer ${token}`;

      let anySuccess = false;
      let anyFailure = false;

      for (const [flagKey] of Object.entries(values)) {
        try {
          const res = await fetch(
            `/api/v1/platform-admin/feature-flags/${encodeURIComponent(flagKey)}/toggle`,
            {
              method: 'POST',
              headers,
              credentials: 'include',
              body: JSON.stringify({ reason }),
            }
          );
          if (res.status === 404) {
            // TODO: wire to real flag IDs once GET /platform-admin/feature-flags is used to
            // resolve flag UUIDs from keys. Endpoint: POST /platform-admin/feature-flags/{id}/toggle
            showToast({
              type: 'warning',
              title: 'Save not yet implemented',
              message: `Flag "${flagKey}" not found on server (404). Wire real flag IDs from GET /platform-admin/feature-flags.`,
            });
            anyFailure = true;
          } else if (!res.ok) {
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

      if (anySuccess && !anyFailure) {
        showToast({
          type: 'success',
          title: t('admin.featureFlags.toast.savedTitle', { defaultValue: 'Flags saved' }),
          message: t('admin.featureFlags.toast.savedMessage', {
            defaultValue: 'Feature flags updated successfully.',
          }),
        });
        resolveSubmitRef.current?.();
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
    [token, showToast, t]
  );

  const handleDialogCancel = useCallback(() => {
    setDialogOpen(false);
    rejectSubmitRef.current?.(new Error('Cancelled'));
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
