-- Refresh the capability descriptions seeded by migration 00152.
--
-- Migration 00152 used `ON CONFLICT (capability) DO NOTHING`, which meant a
-- re-run of the seed could not update stale descriptions or risk levels —
-- only newly-added capabilities took effect. The repo convention (see
-- migration 00141_fix_listings_soft_delete.sql) is to add a follow-up
-- migration rather than edit an applied migration in place, so this file
-- re-runs the INSERT with `DO UPDATE` to refresh the canonical text.
--
-- Idempotent: running this multiple times converges on the same final state.
-- Adding a new capability here is the supported way to ship description
-- changes going forward — bump `updated_at` so registry readers can detect
-- staleness if/when they start caching.

INSERT INTO capability_descriptions (capability, description, risk_level) VALUES
  ('agencies_read',             'View agencies and their basic profile data.',                         'low'),
  ('agencies_write',            'Edit agency profile, branding, contact info.',                        'medium'),
  ('agencies_suspend',          'Suspend or reactivate an agency. Affects all members.',              'high'),
  ('users_read',                'View global user search results across all tenants.',                 'low'),
  ('users_write',               'Edit user profile fields, status, locale.',                           'medium'),
  ('users_impersonate',         'Assume the identity of any user for support purposes.',               'high'),
  ('memberships_grant',         'Add users to organizations and assign roles.',                        'medium'),
  ('memberships_revoke',        'Remove users from organizations.',                                    'medium'),
  ('site_settings_read',        'Read platform-wide settings and configuration.',                      'low'),
  ('site_settings_write',       'Modify platform-wide settings including maintenance mode.',           'high'),
  ('mobile_config_write',       'Push mobile app config: force-update floor, feature flags.',          'high'),
  ('feature_flags_write',       'Toggle feature flags per tenant or globally.',                        'medium'),
  ('tenant_export',             'Export a tenant''s full data as encrypted tarball.',                  'medium'),
  ('tenant_purge',              'Permanently delete a tenant. Irreversible.',                          'critical'),
  ('tenant_restore',            'Restore a previously exported tenant tarball.',                       'high'),
  ('audit_read',                'View the full audit log across all operators and capabilities.',      'low'),
  ('principal_kind_escalate',   'Promote a user to platform principal. Bypasses tenant isolation.',    'critical')
ON CONFLICT (capability) DO UPDATE
    SET description = EXCLUDED.description,
        risk_level  = EXCLUDED.risk_level,
        updated_at  = NOW();
