/**
 * Phase 5 — `/admin/platform` page.
 *
 * Platform-wide settings (operator-facing, never tenant-facing).
 *
 * Reads and writes the singleton `platform_settings` row through
 * `GET` / `PATCH /api/v1/platform-admin/settings`. The wire keys are the
 * dotted form keys (`platform.maintenance_mode`, ...), so the form values are
 * sent as-is. The values are stored and audited (`updated_at` / `updated_by`)
 * but nothing on the platform enforces them yet, which the notice says.
 */

import { type SettingsField, SettingsForm } from '@ppt/admin-ui';
import type React from 'react';
import { useCallback, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { useToast } from '../components/Toast';
import { HelpTooltip } from '../features/help';
import { useSettingsResource } from '../lib/settingsResource';

interface PlatformValues extends Record<string, unknown> {
  'platform.maintenance_mode': boolean;
  'platform.signup_enabled': boolean;
  'platform.support_email': string;
}

interface PlatformSettingsDoc extends PlatformValues {
  updated_at?: string;
}

const SETTINGS_URL = '/api/v1/platform-admin/settings';
const SETTINGS_KEY = ['admin', 'platform', 'settings'] as const;

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

const PlatformPage: React.FC = () => {
  const { t } = useTranslation();
  const { showToast } = useToast();
  const { query, save } = useSettingsResource<PlatformSettingsDoc, PlatformValues>(
    SETTINGS_URL,
    SETTINGS_KEY
  );

  const handleSubmit = useCallback(
    async (values: PlatformValues) => {
      await save.mutateAsync({
        'platform.maintenance_mode': values['platform.maintenance_mode'],
        'platform.signup_enabled': values['platform.signup_enabled'],
        'platform.support_email': values['platform.support_email'].trim(),
      });
      showToast({
        type: 'success',
        title: t('admin.platform.saved', { defaultValue: 'Platform settings saved.' }),
      });
    },
    [save, showToast, t]
  );

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
      {t('admin.platform.storedOnlyNotice', {
        defaultValue:
          'These settings are saved and audited, but the platform does not enforce them yet: turning on maintenance mode or turning off sign-up here has no effect on users.',
      })}
    </span>
  );

  return (
    <section>
      <h1 style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
        {t('admin.platform.title')}
        <HelpTooltip text={t('admin.platform.helpTooltip')} />
      </h1>
      {query.isLoading ? (
        <p role="status">{t('admin.platform.loading', { defaultValue: 'Loading settings…' })}</p>
      ) : query.isError || !query.data ? (
        <p role="alert">
          {t('admin.platform.loadError', { defaultValue: 'Could not load platform settings.' })}{' '}
          {query.error?.message}
        </p>
      ) : (
        <SettingsForm<PlatformValues>
          // Remount on every stored version so the form shows what was saved.
          key={query.data.updated_at ?? 'initial'}
          fields={fields}
          initialValues={{
            'platform.maintenance_mode': !!query.data['platform.maintenance_mode'],
            'platform.signup_enabled': !!query.data['platform.signup_enabled'],
            'platform.support_email': query.data['platform.support_email'] ?? '',
          }}
          capability="site_settings_write"
          notice={notice}
          onSubmit={handleSubmit}
        />
      )}
    </section>
  );
};

export default PlatformPage;
