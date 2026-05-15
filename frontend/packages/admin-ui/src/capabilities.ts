/**
 * Capabilities mirror `admin_core::Capability` (Rust). Keep this list in
 * sync — the backend `/admin/capabilities/registry` endpoint is the source
 * of truth at runtime, but TypeScript needs a closed enum for `useCapability`
 * type-safety.
 */

export const CAPABILITIES = [
  'agencies_read',
  'agencies_write',
  'agencies_suspend',
  'users_read',
  'users_write',
  'users_impersonate',
  'memberships_grant',
  'memberships_revoke',
  'site_settings_read',
  'site_settings_write',
  'mobile_config_write',
  'feature_flags_write',
  'tenant_export',
  'tenant_purge',
  'tenant_restore',
  'audit_read',
  'principal_kind_escalate',
] as const;

export type Capability = (typeof CAPABILITIES)[number];
