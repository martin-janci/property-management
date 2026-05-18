-- Canonical descriptions + risk levels for the 17 capability strings.
-- Owned by the platform team; super-admins read this via
-- GET /admin/capabilities/registry. Stub editing UI is out of scope.

CREATE TABLE IF NOT EXISTS capability_descriptions (
  capability  VARCHAR(64) PRIMARY KEY,
  description TEXT NOT NULL,
  risk_level  VARCHAR(16) NOT NULL CHECK (risk_level IN ('low','medium','high','critical')),
  updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

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
ON CONFLICT (capability) DO NOTHING;
