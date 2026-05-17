/**
 * MobileConfigPage — `/platform/mobile`
 *
 * Two SettingsForm sections:
 *   1. Minimum app version (force-update floor) — AuditReasonPrompt required (min 20 chars)
 *   2. React Native feature flags
 *
 * Wired to PATCH /api/v1/admin/mobile-config. Falls back to warning toast if endpoint is missing.
 * TODO: wire to real endpoint once PATCH /api/v1/admin/mobile-config is implemented.
 */

import { type SettingsField, SettingsForm } from '@ppt/admin-ui';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useAdminAuth } from '../auth/AdminAuthContext';
import { AuditReasonPrompt, useAuditReasonValid } from '../components/AuditReasonPrompt';
import { useToast } from '../components/Toast';

interface ForceUpdateValues extends Record<string, unknown> {
  min_ios_version: string;
  min_android_version: string;
}

interface MobileFlagValues extends Record<string, unknown> {
  'rn.new_onboarding': boolean;
  'rn.push_notifications_v2': boolean;
}

const initialForceUpdateValues: ForceUpdateValues = {
  min_ios_version: '',
  min_android_version: '',
};

const initialFlagValues: MobileFlagValues = {
  'rn.new_onboarding': false,
  'rn.push_notifications_v2': false,
};

// ---------------------------------------------------------------------------
// Audit dialog for force-update version bump
// ---------------------------------------------------------------------------

interface AuditDialogProps {
  onConfirm: (reason: string) => void;
  onCancel: () => void;
  isPending: boolean;
}

function AuditDialog({ onConfirm, onCancel, isPending }: AuditDialogProps) {
  const [reason, setReason] = useState('');
  const isValid = useAuditReasonValid(reason, 20);
  const dialogRef = useRef<HTMLDivElement | null>(null);
  const previouslyFocusedRef = useRef<HTMLElement | null>(null);
  const titleId = 'mobile-config-audit-dialog-title';

  useEffect(() => {
    previouslyFocusedRef.current = document.activeElement as HTMLElement | null;
    const node = dialogRef.current;
    if (node) {
      const focusable = node.querySelector<HTMLElement>(
        'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
      );
      (focusable ?? node).focus();
    }
    return () => {
      previouslyFocusedRef.current?.focus?.();
    };
  }, []);

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
          Update minimum app version
        </h2>
        <p style={{ margin: 0, fontSize: 13, color: 'var(--ppt-fg-secondary, #374151)' }}>
          This will force users on older app versions to update. Provide an audit reason.
        </p>
        <AuditReasonPrompt
          action="force_update_floor_bump"
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
            {isPending ? 'Saving…' : 'Save version floor'}
          </button>
        </div>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

export default function MobileConfigPage() {
  const { t } = useTranslation();
  const { token } = useAdminAuth();
  const { showToast } = useToast();

  const pendingForceUpdateRef = useRef<ForceUpdateValues | null>(null);
  const resolveRef = useRef<(() => void) | null>(null);
  const rejectRef = useRef<((err: Error) => void) | null>(null);

  const [dialogOpen, setDialogOpen] = useState(false);
  const [dialogPending, setDialogPending] = useState(false);

  const forceUpdateFields = useMemo<ReadonlyArray<SettingsField>>(
    () => [
      {
        kind: 'text',
        key: 'min_ios_version',
        label: t('admin.mobile.fields.minIosVersion', { defaultValue: 'Minimum iOS version' }),
        placeholder: t('admin.mobile.fields.minIosVersionPlaceholder', {
          defaultValue: 'e.g. 2.3.0',
        }),
      },
      {
        kind: 'text',
        key: 'min_android_version',
        label: t('admin.mobile.fields.minAndroidVersion', {
          defaultValue: 'Minimum Android version',
        }),
        placeholder: t('admin.mobile.fields.minAndroidVersionPlaceholder', {
          defaultValue: 'e.g. 2.3.0',
        }),
      },
    ],
    [t]
  );

  const flagFields = useMemo<ReadonlyArray<SettingsField>>(
    () => [
      {
        kind: 'boolean',
        key: 'rn.new_onboarding',
        label: t('admin.mobile.flags.newOnboarding', {
          defaultValue: 'New onboarding flow (React Native)',
        }),
      },
      {
        kind: 'boolean',
        key: 'rn.push_notifications_v2',
        label: t('admin.mobile.flags.pushNotificationsV2', {
          defaultValue: 'Push notifications v2 (React Native)',
        }),
      },
    ],
    [t]
  );

  const handleForceUpdateSubmit = useCallback((values: ForceUpdateValues): Promise<void> => {
    pendingForceUpdateRef.current = values;
    setDialogOpen(true);
    return new Promise<void>((resolve, reject) => {
      resolveRef.current = resolve;
      rejectRef.current = reject;
    });
  }, []);

  const handleDialogConfirm = useCallback(
    async (reason: string) => {
      const values = pendingForceUpdateRef.current;
      if (!values) return;
      setDialogPending(true);
      try {
        const headers: Record<string, string> = { 'Content-Type': 'application/json' };
        if (token) headers.Authorization = `Bearer ${token}`;
        const res = await fetch('/api/v1/admin/mobile-config', {
          method: 'PATCH',
          headers,
          credentials: 'include',
          body: JSON.stringify({ ...values, reason }),
        });
        if (res.status === 404 || res.status === 405) {
          // TODO: endpoint PATCH /api/v1/admin/mobile-config not yet implemented
          showToast({
            type: 'warning',
            title: 'Save not yet implemented',
            message: 'PATCH /api/v1/admin/mobile-config is not yet available.',
          });
          resolveRef.current?.();
        } else if (res.ok) {
          showToast({
            type: 'success',
            title: t('admin.mobile.toast.savedTitle', { defaultValue: 'Mobile config saved' }),
            message: t('admin.mobile.toast.savedMessage', {
              defaultValue: 'Minimum version floor updated.',
            }),
          });
          resolveRef.current?.();
        } else {
          const text = await res.text().catch(() => '');
          throw new Error(text || `HTTP ${res.status}`);
        }
      } catch (err) {
        showToast({
          type: 'error',
          title: t('admin.mobile.toast.saveFailedTitle', { defaultValue: 'Save failed' }),
          message: err instanceof Error ? err.message : 'Unknown error',
        });
        rejectRef.current?.(err instanceof Error ? err : new Error('Save failed'));
      } finally {
        setDialogPending(false);
        setDialogOpen(false);
      }
    },
    [token, showToast, t]
  );

  const handleDialogCancel = useCallback(() => {
    // Resolve cleanly on cancel so SettingsForm does not render the
    // "Cancelled" rejection string as a form error.
    setDialogOpen(false);
    resolveRef.current?.();
  }, []);

  const handleFlagSubmit = useCallback(
    async (_values: MobileFlagValues) => {
      // TODO: wire to PATCH /api/v1/admin/mobile-config once implemented
      showToast({
        type: 'warning',
        title: 'Save not yet implemented',
        message: 'PATCH /api/v1/admin/mobile-config is not yet available.',
      });
    },
    [showToast]
  );

  return (
    <>
      {dialogOpen && (
        <AuditDialog
          onConfirm={handleDialogConfirm}
          onCancel={handleDialogCancel}
          isPending={dialogPending}
        />
      )}
      <section style={{ padding: '24px 32px', display: 'flex', flexDirection: 'column', gap: 32 }}>
        <div>
          <h1
            style={{
              fontSize: '1.375rem',
              fontWeight: 700,
              color: 'var(--ppt-fg-primary, #111827)',
              margin: '0 0 8px',
            }}
          >
            {t('admin.mobile.title', { defaultValue: 'Mobile configuration' })}
          </h1>
          <p style={{ fontSize: 13, color: 'var(--ppt-fg-muted, #6b7280)', margin: 0 }}>
            {t('admin.mobile.subtitle', {
              defaultValue: 'Configure force-update floors and React Native feature flags.',
            })}
          </p>
        </div>

        <div>
          <h2
            style={{
              fontSize: '1rem',
              fontWeight: 600,
              color: 'var(--ppt-fg-primary, #111827)',
              margin: '0 0 12px',
            }}
          >
            {t('admin.mobile.forceUpdateTitle', { defaultValue: 'Force-update floor' })}
          </h2>
          <SettingsForm<ForceUpdateValues>
            fields={forceUpdateFields}
            initialValues={initialForceUpdateValues}
            capability="mobile_config_write"
            onSubmit={handleForceUpdateSubmit}
          />
        </div>

        <div>
          <h2
            style={{
              fontSize: '1rem',
              fontWeight: 600,
              color: 'var(--ppt-fg-primary, #111827)',
              margin: '0 0 12px',
            }}
          >
            {t('admin.mobile.flagsTitle', { defaultValue: 'React Native feature flags' })}
          </h2>
          <SettingsForm<MobileFlagValues>
            fields={flagFields}
            initialValues={initialFlagValues}
            capability="mobile_config_write"
            onSubmit={handleFlagSubmit}
          />
        </div>
      </section>
    </>
  );
}
