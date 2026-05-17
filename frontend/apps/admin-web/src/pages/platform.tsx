/**
 * Phase 5 — `/admin/platform` page.
 *
 * Platform-wide settings (operator-facing, never tenant-facing). Today this
 * is a thin SettingsForm shell; future waves will add infrastructure dashboards,
 * runtime feature toggles, and capability-grant management.
 */

import { type SettingsField, SettingsForm } from '@ppt/admin-ui';
import type React from 'react';
import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';

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

  return (
    <section>
      <h1>{t('admin.platform.title')}</h1>
      <SettingsForm<PlatformValues>
        fields={fields}
        initialValues={initialValues}
        capability="site_settings_write"
        onSubmit={(next) => {
          console.warn('TODO: PATCH /admin/platform/settings', next);
        }}
      />
    </section>
  );
};

export default PlatformPage;
