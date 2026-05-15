/**
 * Phase 5 — `/admin/feature-flags` page.
 *
 * Stub: write-only-from-admin-console feature flag management. Phase 3 owns
 * the per-tenant `tenant_feature_flags` table; Phase 5 only exposes the UI.
 */

import { SettingsForm, type SettingsField } from '@ppt/admin-ui';
import type React from 'react';

interface FeatureFlagValues extends Record<string, unknown> {
  'beta.new_listings_search': boolean;
  'beta.ai_fault_triage': boolean;
}

const fields: ReadonlyArray<SettingsField> = [
  { kind: 'boolean', key: 'beta.new_listings_search', label: 'New listings search (beta)' },
  { kind: 'boolean', key: 'beta.ai_fault_triage', label: 'AI fault triage (beta)' },
];

const initialValues: FeatureFlagValues = {
  'beta.new_listings_search': false,
  'beta.ai_fault_triage': false,
};

const FeatureFlagsPage: React.FC = () => {
  return (
    <section>
      <h1>Feature Flags</h1>
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
