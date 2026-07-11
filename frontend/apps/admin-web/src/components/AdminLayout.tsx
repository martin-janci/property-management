import { useCapability } from '@ppt/admin-ui';
import type { ReactNode } from 'react';
import React from 'react';
import { useTranslation } from 'react-i18next';
import { Link, Outlet, useLocation } from 'react-router-dom';

import { useAdminAuth } from '../auth/AdminAuthContext';
import { HelpSidebar, useContextualHelp } from '../features/help';
import './AdminLayout.css';
import { ConnectedMfaWindowChip } from './MfaWindowChip';

interface SidebarGroupProps {
  label: string;
  children: ReactNode;
}

/**
 * Renders group header + children only when at least one child is non-null.
 * Prevents orphan headers when all children are gated out.
 */
function SidebarGroup({ label, children }: SidebarGroupProps) {
  const nonNull = React.Children.toArray(children).filter(Boolean);
  if (nonNull.length === 0) return null;
  return (
    <div style={{ marginTop: '20px' }}>
      <div
        style={{
          fontSize: '11px',
          fontWeight: 600,
          letterSpacing: 'var(--ppt-tracking-caps, 0.06em)',
          textTransform: 'uppercase',
          color: 'var(--ppt-fg-muted, #6b7280)',
          padding: '0 12px',
          marginBottom: '4px',
        }}
      >
        {label}
      </div>
      {nonNull}
    </div>
  );
}

interface NavItemProps {
  to: string;
  label: string;
}

function NavItem({ to, label }: NavItemProps) {
  const location = useLocation();
  const isActive = to === '/' ? location.pathname === '/' : location.pathname.startsWith(to);

  return (
    <Link
      to={to}
      style={{
        display: 'block',
        padding: '6px 12px',
        borderRadius: 'var(--ppt-radius-md, 8px)',
        textDecoration: 'none',
        fontSize: '14px',
        background: isActive ? 'var(--ppt-accent-soft-bg, #eff6ff)' : 'transparent',
        color: isActive ? 'var(--ppt-brand-700, #1d4ed8)' : 'var(--ppt-fg-secondary, #374151)',
        fontWeight: isActive ? 500 : 400,
      }}
    >
      {label}
    </Link>
  );
}

export function AdminLayout() {
  const { t } = useTranslation();
  const auth = useAdminAuth();
  const help = useContextualHelp();

  // TENANTS
  const canAgenciesRead = useCapability('agencies_read');
  const canTenantExport = useCapability('tenant_export');
  const canTenantPurge = useCapability('tenant_purge');
  const canTenantRestore = useCapability('tenant_restore');

  // IDENTITY
  const canUsersRead = useCapability('users_read');
  const canPrincipalKindEscalate = useCapability('principal_kind_escalate');
  const canMembershipsGrant = useCapability('memberships_grant');
  const canMembershipsRevoke = useCapability('memberships_revoke');

  // OPERATIONS
  const canAuditRead = useCapability('audit_read');
  const canUsersImpersonate = useCapability('users_impersonate');
  const canFeatureFlagsWrite = useCapability('feature_flags_write');

  // PLATFORM
  const canSiteSettingsWrite = useCapability('site_settings_write');
  const canMobileConfigWrite = useCapability('mobile_config_write');
  const canHealthRead = useCapability('audit_read');
  const canSupportDataRead = useCapability('audit_read');

  // DEVELOPER
  const canOauthClientWrite = useCapability('oauth_client_write');

  return (
    <div className="admin-shell">
      <aside>
        <h2>{t('admin.title', 'Admin')}</h2>
        <nav style={{ display: 'flex', flexDirection: 'column', gap: '2px' }}>
          {/* No group header — overview always visible */}
          <NavItem to="/" label={t('admin.nav.overview', 'Overview')} />

          <SidebarGroup label="TENANTS">
            {canAgenciesRead ? (
              <NavItem to="/tenants/agencies" label={t('admin.agencies.title', 'Agencies')} />
            ) : null}
            {canTenantExport || canTenantPurge || canTenantRestore ? (
              <NavItem to="/tenants/lifecycle" label={t('admin.tenants.lifecycle', 'Lifecycle')} />
            ) : null}
          </SidebarGroup>

          <SidebarGroup label="IDENTITY">
            {canUsersRead || canPrincipalKindEscalate ? (
              <NavItem to="/identity/users" label={t('admin.users.title', 'Users')} />
            ) : null}
            {canMembershipsGrant || canMembershipsRevoke ? (
              <NavItem
                to="/identity/memberships"
                label={t('admin.memberships.title', 'Memberships')}
              />
            ) : null}
            {canMembershipsGrant ? (
              <NavItem
                to="/identity/capabilities"
                label={t('admin.capabilities.title', 'Capabilities')}
              />
            ) : null}
          </SidebarGroup>

          <SidebarGroup label="OPERATIONS">
            {canAuditRead ? (
              <NavItem to="/ops/audit" label={t('admin.audit.title', 'Audit log')} />
            ) : null}
            {canUsersImpersonate ? (
              <NavItem
                to="/ops/impersonation"
                label={t('admin.impersonation.title', 'Impersonation')}
              />
            ) : null}
            {canFeatureFlagsWrite ? (
              <NavItem
                to="/ops/feature-flags"
                label={t('admin.featureFlags.title', 'Feature flags')}
              />
            ) : null}
          </SidebarGroup>

          <SidebarGroup label="PLATFORM">
            {canAgenciesRead ? (
              <NavItem
                to="/platform/organizations"
                label={t('admin.organizations.navLabel', 'Organizations')}
              />
            ) : null}
            {canSiteSettingsWrite ? (
              <NavItem to="/platform/settings" label={t('admin.platform.title', 'Settings')} />
            ) : null}
            {canMobileConfigWrite ? (
              <NavItem to="/platform/mobile" label={t('admin.platform.mobile', 'Mobile config')} />
            ) : null}
            {canHealthRead ? (
              <NavItem to="/platform/health" label={t('admin.health.navLabel', 'Health')} />
            ) : null}
            {canSiteSettingsWrite ? (
              <NavItem
                to="/platform/announcements"
                label={t('admin.announcements.navLabel', 'Announcements')}
              />
            ) : null}
            {canSupportDataRead ? (
              <NavItem
                to="/platform/support-data"
                label={t('admin.supportData.navLabel', 'Support data')}
              />
            ) : null}
            {canSiteSettingsWrite ? (
              <NavItem
                to="/platform/onboarding"
                label={t('admin.onboarding.navLabel', 'Onboarding')}
              />
            ) : null}
          </SidebarGroup>

          <SidebarGroup label="DEVELOPER">
            {canOauthClientWrite ? (
              <NavItem
                to="/identity/oauth-clients"
                label={t('admin.oauthClients.navLabel', 'OAuth Clients')}
              />
            ) : null}
          </SidebarGroup>
        </nav>
        <button type="button" onClick={auth.logout}>
          {t('admin.actions.signOut', 'Sign out')}
        </button>
      </aside>
      <main style={{ display: 'flex', flexDirection: 'column', flex: 1, minWidth: 0 }}>
        {/* Topbar */}
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            padding: '8px 22px',
            borderBottom: '1px solid var(--ppt-border-default, #e5e7eb)',
            background: 'var(--ppt-bg-surface, #ffffff)',
            gap: '8px',
          }}
        >
          <span style={{ flex: 1 }} />
          <ConnectedMfaWindowChip />
          {/* Help toggle — styles live in AdminLayout.css (.ppt-topbar-help-btn) */}
          <button
            type="button"
            onClick={help.toggle}
            aria-expanded={help.isOpen}
            aria-label={t('admin.help.openHelp', 'Open contextual help')}
            className="ppt-topbar-help-btn"
          >
            ?
          </button>
          <button type="button" onClick={auth.logout} style={{ fontSize: '13px' }}>
            {t('admin.actions.signOut', 'Sign out')}
          </button>
        </div>
        <Outlet />
        {help.isOpen && <HelpSidebar article={help.article} onClose={help.close} />}
      </main>
    </div>
  );
}
