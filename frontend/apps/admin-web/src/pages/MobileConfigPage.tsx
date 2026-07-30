/**
 * MobileConfigPage — `/platform/mobile`
 *
 * Two SettingsForm sections:
 *   1. Minimum app version (force-update floor)
 *   2. React Native feature flags
 *
 * NOTE: there is no backend write endpoint for mobile config yet — no
 * `PATCH /api/v1/admin/mobile-config` exists. Both forms are therefore
 * rendered read-only via `SettingsForm`'s `readOnly` prop with an explanatory
 * notice, rather than presenting a Save button that fires at a non-existent
 * endpoint and silently no-ops. Re-enable writes (and restore the
 * force-update audit-reason flow) once the endpoint lands.
 */

import { type SettingsField, SettingsForm } from '@ppt/admin-ui';
import { type ReactNode, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { HelpTooltip } from '../features/help';

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

const noop = () => {};

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

export default function MobileConfigPage() {
  const { t } = useTranslation();

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

  const notice = useMemo<ReactNode>(
    () => (
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
        {t('admin.mobile.readOnlyNotice', {
          defaultValue:
            'Saving is not available yet — the mobile config write endpoint is not implemented. These values are shown for reference only.',
        })}
      </span>
    ),
    [t]
  );

  return (
    <section style={{ padding: '24px 32px', display: 'flex', flexDirection: 'column', gap: 32 }}>
      <div>
        <h1
          style={{
            fontSize: '1.375rem',
            fontWeight: 700,
            color: 'var(--ppt-fg-primary, #111827)',
            margin: '0 0 8px',
            display: 'flex',
            alignItems: 'center',
            gap: 8,
          }}
        >
          {t('admin.mobile.title', { defaultValue: 'Mobile configuration' })}
          <HelpTooltip text={t('admin.mobile.helpTooltip')} />
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
          readOnly
          notice={notice}
          onSubmit={noop}
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
          readOnly
          notice={notice}
          onSubmit={noop}
        />
      </div>
    </section>
  );
}
