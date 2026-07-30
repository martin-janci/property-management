/**
 * Phase 5 — `/admin/platform` page.
 *
 * Platform-wide settings (operator-facing, never tenant-facing).
 *
 * NOTE: there is no backend write endpoint for these settings yet — no
 * `PATCH /api/v1/platform-admin/settings` exists. The form is therefore
 * rendered read-only via `SettingsForm`'s `readOnly` prop with an explanatory
 * notice, rather than presenting a Save button that fires at a non-existent
 * endpoint and silently no-ops. Re-enable writes (and restore the
 * maintenance-mode audit-reason flow) once the endpoint lands.
 */

import { type SettingsField, SettingsForm } from '@ppt/admin-ui';
import type React from 'react';
import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
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

const noop = () => {};

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

const PlatformPage: React.FC = () => {
  const { t } = useTranslation();

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

  const notice = (
    <span
      style={{
        display: 'block',
        padding: '10px 12px',
        borderRadius: 8,
        border: '1px solid var(--ppt-warning-300, #fcd34d)',
        background: 'var(--ppt-warning-50, #fffbeb)',
        color: 'var(--ppt-warning-800, #92400e)',
        fontSize: 13,
      }}
    >
      {t('admin.platform.readOnlyNotice', {
        defaultValue:
          'Saving is not available yet — the platform settings write endpoint is not implemented. These values are shown for reference only.',
      })}
    </span>
  );

  return (
    <section>
      <h1 style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
        {t('admin.platform.title')}
        <HelpTooltip text={t('admin.platform.helpTooltip')} />
      </h1>
      <SettingsForm<PlatformValues>
        fields={fields}
        initialValues={initialValues}
        capability="site_settings_write"
        readOnly
        notice={notice}
        onSubmit={noop}
      />
    </section>
  );
};

export default PlatformPage;
