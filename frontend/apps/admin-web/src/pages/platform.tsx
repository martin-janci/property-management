/**
 * Phase 5 — `/admin/platform` page.
 *
 * Platform-wide settings (operator-facing, never tenant-facing).
 *
 * The maintenance_mode_on field uses AuditReasonPrompt (minLength=20) before
 * submit. Other fields submit directly.
 *
 * Backend endpoint: no PATCH /platform-admin/settings exists yet.
 * TODO: wire to PATCH /api/v1/platform-admin/settings once implemented.
 * For now, shows toast "Save not yet implemented" to give clear feedback.
 */

import { type SettingsField, SettingsForm } from '@ppt/admin-ui';
import type React from 'react';
import { useCallback, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { AuditReasonPrompt, useAuditReasonValid } from '../components/AuditReasonPrompt';
import { useToast } from '../components/Toast';
import { useFocusTrap } from '../components/useFocusTrap';
import { HelpTooltip } from '../features/help';

interface PlatformValues extends Record<string, unknown> {
  'platform.maintenance_mode': boolean;
  'platform.signup_enabled': boolean;
  'platform.support_email': string;
}

const initialValues: PlatformValues = {
  'platform.maintenance_mode': false,
  'platform.signup_enabled': true,
  'platform.support_email': '',
};

// ---------------------------------------------------------------------------
// Audit reason dialog for high-risk settings (maintenance mode)
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
  const titleId = 'platform-audit-dialog-title';

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
            color: 'var(--ppt-warning-800, #92400e)',
          }}
        >
          Save platform settings
        </h2>
        <p style={{ margin: 0, fontSize: 13, color: 'var(--ppt-fg-secondary, #374151)' }}>
          Maintenance mode is enabled — describe the maintenance being performed.
        </p>
        <AuditReasonPrompt
          action="maintenance_mode_on"
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
            {isPending ? 'Saving…' : 'Save settings'}
          </button>
        </div>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

const PlatformPage: React.FC = () => {
  const { t } = useTranslation();
  const { showToast } = useToast();

  const pendingValuesRef = useRef<PlatformValues | null>(null);
  const resolveSubmitRef = useRef<(() => void) | null>(null);
  const rejectSubmitRef = useRef<((err: Error) => void) | null>(null);

  const [dialogOpen, setDialogOpen] = useState(false);

  const fields = useMemo<ReadonlyArray<SettingsField>>(
    () => [
      {
        kind: 'boolean',
        key: 'platform.maintenance_mode',
        label: t('admin.platform.fields.maintenanceMode'),
      },
      {
        kind: 'boolean',
        key: 'platform.signup_enabled',
        label: t('admin.platform.fields.signupEnabled'),
      },
      {
        kind: 'text',
        key: 'platform.support_email',
        label: t('admin.platform.fields.supportEmail'),
        placeholder: t('admin.platform.fields.supportEmailPlaceholder'),
      },
    ],
    [t]
  );

  const handleSubmit = useCallback(
    (values: PlatformValues): Promise<void> => {
      pendingValuesRef.current = values;
      const maintenanceModeEnabled = values['platform.maintenance_mode'] === true;

      if (maintenanceModeEnabled) {
        // Require audit reason for high-risk maintenance mode toggle
        setDialogOpen(true);
        return new Promise<void>((resolve, reject) => {
          resolveSubmitRef.current = resolve;
          rejectSubmitRef.current = reject;
        });
      }

      // Non-high-risk submit — no real endpoint yet
      // TODO: wire to PATCH /api/v1/platform-admin/settings once implemented
      showToast({
        type: 'warning',
        title: 'Save not yet implemented',
        message: 'PATCH /api/v1/platform-admin/settings endpoint is not yet available.',
      });
      return Promise.resolve();
    },
    [showToast]
  );

  const handleConfirm = useCallback(
    (_reason: string) => {
      // TODO: wire to PATCH /api/v1/platform-admin/settings once implemented
      // Body should include all pendingValuesRef.current fields + reason
      setDialogOpen(false);
      showToast({
        type: 'warning',
        title: 'Save not yet implemented',
        message: 'PATCH /api/v1/platform-admin/settings endpoint is not yet available.',
      });
      resolveSubmitRef.current?.();
    },
    [showToast]
  );

  const handleDialogCancel = useCallback(() => {
    // Resolve the form's submit Promise on cancel so SettingsForm does not
    // render "Cancelled" as a form error — the user simply backs out.
    setDialogOpen(false);
    resolveSubmitRef.current?.();
  }, []);

  return (
    <>
      {dialogOpen && (
        <AuditReasonDialog
          onConfirm={handleConfirm}
          onCancel={handleDialogCancel}
          isPending={false}
        />
      )}
      <section>
        <h1 style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          {t('admin.platform.title')}
          <HelpTooltip text={t('admin.platform.helpTooltip')} />
        </h1>
        <SettingsForm<PlatformValues>
          fields={fields}
          initialValues={initialValues}
          capability="site_settings_write"
          onSubmit={handleSubmit}
        />
      </section>
    </>
  );
};

export default PlatformPage;
