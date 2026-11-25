/**
 * MobileConfigPage — `/platform/mobile`
 *
 * Two SettingsForm sections:
 *   1. Minimum app version (force-update floor)
 *   2. React Native feature flags
 *
 * Reads and writes the singleton `mobile_config` row through `GET` / `PATCH
 * /api/v1/admin/mobile-config`. The versions are top-level fields; the flag
 * toggles live in the nested `flags` object, which PATCH merges key by key.
 * No app reads this config yet (there is no public read endpoint), which the
 * notice says.
 */

import { type SettingsField, SettingsForm } from '@ppt/admin-ui';
import { type ReactNode, useCallback, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { useToast } from '../components/Toast';
import { HelpTooltip } from '../features/help';
import { useSettingsResource } from '../lib/settingsResource';

interface ForceUpdateValues extends Record<string, unknown> {
  min_ios_version: string;
  min_android_version: string;
}

interface MobileFlagValues extends Record<string, unknown> {
  'rn.new_onboarding': boolean;
  'rn.push_notifications_v2': boolean;
}

const FLAG_KEYS = ['rn.new_onboarding', 'rn.push_notifications_v2'] as const;

/** `MobileConfigResponse` from api-server. */
export interface MobileConfigDoc {
  min_ios_version: string | null;
  min_android_version: string | null;
  flags: Record<string, unknown> | null;
  updated_at?: string;
}

/** `MobileConfigPatchRequest`: only the supplied fields are applied. */
export interface MobileConfigPatch {
  min_ios_version?: string;
  min_android_version?: string;
  flags?: Record<string, boolean>;
}

/** Stored flags object → the form's flat boolean values. */
export function flagValuesFromDoc(doc: MobileConfigDoc): MobileFlagValues {
  const flags = doc.flags ?? {};
  return {
    'rn.new_onboarding': flags['rn.new_onboarding'] === true,
    'rn.push_notifications_v2': flags['rn.push_notifications_v2'] === true,
  };
}

/** The form's flat flag values → the nested `flags` patch. */
export function flagsPatch(values: MobileFlagValues): MobileConfigPatch {
  const flags: Record<string, boolean> = {};
  for (const key of FLAG_KEYS) flags[key] = values[key] === true;
  return { flags };
}

const CONFIG_URL = '/api/v1/admin/mobile-config';
const CONFIG_KEY = ['admin', 'mobile-config'] as const;

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

export default function MobileConfigPage() {
  const { t } = useTranslation();
  const { showToast } = useToast();
  const { query, save } = useSettingsResource<MobileConfigDoc, MobileConfigPatch>(
    CONFIG_URL,
    CONFIG_KEY
  );

  const notifySaved = useCallback(() => {
    showToast({
      type: 'success',
      title: t('admin.mobile.saved', { defaultValue: 'Mobile configuration saved.' }),
    });
  }, [showToast, t]);

  const handleVersionsSubmit = useCallback(
    async (values: ForceUpdateValues) => {
      // An empty string clears the stored floor.
      await save.mutateAsync({
        min_ios_version: values.min_ios_version.trim(),
        min_android_version: values.min_android_version.trim(),
      });
      notifySaved();
    },
    [save, notifySaved]
  );

  const handleFlagsSubmit = useCallback(
    async (values: MobileFlagValues) => {
      await save.mutateAsync(flagsPatch(values));
      notifySaved();
    },
    [save, notifySaved]
  );

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
        {t('admin.mobile.storedOnlyNotice', {
          defaultValue:
            'This configuration is saved and audited, but the mobile apps do not read it yet: changing a version floor or a flag here has no effect on users.',
        })}
      </span>
    ),
    [t]
  );

  const doc = query.data;
  // Remount the forms on every stored version so they show what was saved.
  const version = doc?.updated_at ?? 'initial';
  const status: ReactNode = query.isLoading ? (
    <p role="status">{t('admin.mobile.loading', { defaultValue: 'Loading configuration…' })}</p>
  ) : (
    <p role="alert">
      {t('admin.mobile.loadError', { defaultValue: 'Could not load mobile configuration.' })}{' '}
      {query.error?.message}
    </p>
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
        {doc ? (
          <SettingsForm<ForceUpdateValues>
            key={`versions-${version}`}
            fields={forceUpdateFields}
            initialValues={{
              min_ios_version: doc.min_ios_version ?? '',
              min_android_version: doc.min_android_version ?? '',
            }}
            capability="mobile_config_write"
            notice={notice}
            onSubmit={handleVersionsSubmit}
          />
        ) : (
          status
        )}
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
        {doc ? (
          <SettingsForm<MobileFlagValues>
            key={`flags-${version}`}
            fields={flagFields}
            initialValues={flagValuesFromDoc(doc)}
            capability="mobile_config_write"
            notice={notice}
            onSubmit={handleFlagsSubmit}
          />
        ) : (
          status
        )}
      </div>
    </section>
  );
}
