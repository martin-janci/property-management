/**
 * Phase 5 — `/admin/platform` page.
 *
 * Platform-wide settings (operator-facing, never tenant-facing). Today this
 * is a thin SettingsForm shell; future waves will add infrastructure dashboards,
 * runtime feature toggles, and capability-grant management.
 */

import { SettingsForm, type SettingsField } from '@ppt/admin-ui';
import type React from 'react';

interface PlatformValues extends Record<string, unknown> {
  'platform.maintenance_mode': boolean;
  'platform.signup_enabled': boolean;
  'platform.support_email': string;
}

const fields: ReadonlyArray<SettingsField> = [
  { kind: 'boolean', key: 'platform.maintenance_mode', label: 'Maintenance mode' },
  { kind: 'boolean', key: 'platform.signup_enabled', label: 'Signup enabled' },
  {
    kind: 'text',
    key: 'platform.support_email',
    label: 'Support email',
    placeholder: 'support@example.com',
  },
];

const initialValues: PlatformValues = {
  'platform.maintenance_mode': false,
  'platform.signup_enabled': true,
  'platform.support_email': '',
};

const PlatformPage: React.FC = () => {
  return (
    <section>
      <h1>Platform Settings</h1>
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
