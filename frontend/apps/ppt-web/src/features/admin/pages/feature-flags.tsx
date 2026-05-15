/**
 * Phase 5 — `/admin/feature-flags` page.
 *
 * Stub: write-only-from-admin-console feature flag management. Phase 3 owns
 * the per-tenant `tenant_feature_flags` table; Phase 5 only exposes the UI.
 */

import { SettingsForm, type SettingsField } from '@ppt/admin-ui';
import type React from 'react';
import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';

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

const FeatureFlagsPage: React.FC = () => {
  const { t } = useTranslation();

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
    [t],
  );

  return (
    <section>
      <h1>{t('admin.featureFlags.title')}</h1>
      <p
        role="alert"
        className="ppt-admin-warning"
        style={{ color: '#b00020', fontWeight: 600 }}
      >
        {t('admin.featureFlags.warning')}
      </p>
      <SettingsForm<FeatureFlagValues>
        fields={fields}
        initialValues={initialValues}
        capability="feature_flags_write"
        onSubmit={(next) => {
          console.warn('TODO: PATCH /admin/agencies/:id/feature-flags', next);
        }}
      />
    </section>
  );
};

export default FeatureFlagsPage;
