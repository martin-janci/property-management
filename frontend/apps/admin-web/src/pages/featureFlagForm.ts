/**
 * Form model for the global feature-flag toggles.
 *
 * The page used to render three hard-coded keys (`beta.new_listings_search`,
 * `beta.ai_fault_triage`, `building.disabled`) that do not exist in the
 * `feature_flags` table, so every toggle ended in "Unknown feature flag".
 * The form is now built from the flags the server actually returns.
 */

export interface BackendFeatureFlag {
  id: string;
  key: string;
  name?: string;
  is_enabled: boolean;
}

export type FeatureFlagValues = Record<string, boolean>;

export interface FeatureFlagField {
  kind: 'boolean';
  key: string;
  label: string;
}

/** One boolean field per server flag, sorted by key for a stable order. */
export function flagFields(flags: readonly BackendFeatureFlag[]): FeatureFlagField[] {
  return [...flags]
    .sort((a, b) => a.key.localeCompare(b.key))
    .map((f) => ({
      kind: 'boolean' as const,
      key: f.key,
      label: f.name?.trim() ? `${f.name} (${f.key})` : f.key,
    }));
}

/** Current server state, keyed by flag key. */
export function flagInitialValues(flags: readonly BackendFeatureFlag[]): FeatureFlagValues {
  const values: FeatureFlagValues = {};
  for (const f of flags) values[f.key] = f.is_enabled;
  return values;
}

/** Flags whose submitted value differs from the server baseline. */
export function changedFlags(
  flags: readonly BackendFeatureFlag[],
  submitted: Record<string, unknown>
): BackendFeatureFlag[] {
  return flags.filter((f) => (submitted[f.key] === true) !== f.is_enabled);
}
