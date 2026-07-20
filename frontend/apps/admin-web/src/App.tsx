import { CapabilityProvider } from '@ppt/admin-ui';
import type { ReactNode } from 'react';
import { Route, Routes } from 'react-router-dom';

import { AdminAuthProvider } from './auth/AdminAuthContext';
import { usePrincipalCapabilities } from './auth/usePrincipalCapabilities';
import { AdminLayout } from './components/AdminLayout';
import { ImpersonationWrapper } from './components/ImpersonationWrapper';
import { MfaWindowProvider } from './components/MfaWindowChip';
import { MfaWrapper } from './components/MfaWrapper';
import { ProtectedRoute } from './components/ProtectedRoute';
import { ToastProvider } from './components/Toast';
import LayoutEditorPage from './features/layout-editor/LayoutEditorPage';
import LayoutManifestsPage from './features/layout-editor/LayoutManifestsPage';
import AgencyDetailPage from './pages/AgencyDetailPage';
import AgenciesPage from './pages/agencies';
import AuditPage from './pages/audit';
import CapabilitiesAdminPage from './pages/CapabilitiesAdminPage';
import { Dashboard } from './pages/Dashboard';
import FeatureFlagsPage from './pages/feature-flags';
import ImpersonationListPage from './pages/ImpersonationListPage';
import { LoginPage } from './pages/LoginPage';
import MembershipsPage from './pages/MembershipsPage';
import MobileConfigPage from './pages/MobileConfigPage';
import OAuthClientsPage from './pages/OAuthClientsPage';
import OAuthConsentPage from './pages/OAuthConsentPage';
import OnboardingToursPage from './pages/OnboardingToursPage';
import OrganizationsPage from './pages/OrganizationsPage';
import PlatformHealthPage from './pages/PlatformHealthPage';
import PlatformPage from './pages/platform';
import SupportDataPage from './pages/SupportDataPage';
import SystemAnnouncementsPage from './pages/SystemAnnouncementsPage';
import TenantLifecyclePage from './pages/TenantLifecyclePage';
import UsersPage from './pages/users';

/**
 * Feeds the live capability set + platform-principal flag (from
 * `/admin/capabilities/me`) into `@ppt/admin-ui`'s `CapabilityProvider`.
 *
 * Without this wrapper, `useCapability()` inside `<ResourceTable>` and
 * `<SettingsForm>` sees the default `{ capabilities: [], isPlatformPrincipal:
 *  false }` and gates every action button to "hidden". Wrapping the admin
 * tree restores the per-control gating the legacy admin router had.
 */
function AdminCapabilityScope({ children }: { children: ReactNode }) {
  const { capabilities, isPlatformPrincipal } = usePrincipalCapabilities();
  return (
    <CapabilityProvider value={{ capabilities, isPlatformPrincipal }}>
      {children}
    </CapabilityProvider>
  );
}

export function App() {
  return (
    <AdminAuthProvider>
      <MfaWindowProvider>
        <ToastProvider>
          <MfaWrapper>
            <ImpersonationWrapper />
            <Routes>
              <Route path="/login" element={<LoginPage />} />
              {/* OAuth 2.0 consent screen — standalone (no admin chrome). Auth
                  is enforced inside the page (Bearer token required), but no
                  capability gate: it is a per-user authorization decision. */}
              <Route path="/oauth/authorize" element={<OAuthConsentPage />} />
              <Route
                element={
                  <ProtectedRoute>
                    <AdminCapabilityScope>
                      <AdminLayout />
                    </AdminCapabilityScope>
                  </ProtectedRoute>
                }
              >
                {/* Overview — no cap required */}
                <Route index element={<Dashboard />} />

                {/* TENANTS */}
                <Route
                  path="tenants/agencies"
                  element={
                    <ProtectedRoute requiredCapability="agencies_read">
                      <AgenciesPage />
                    </ProtectedRoute>
                  }
                />
                <Route
                  path="tenants/agencies/:id"
                  element={
                    <ProtectedRoute requiredCapability="agencies_read">
                      <AgencyDetailPage />
                    </ProtectedRoute>
                  }
                />
                <Route
                  path="tenants/lifecycle"
                  element={
                    <ProtectedRoute
                      requiredCapability={['tenant_export', 'tenant_purge', 'tenant_restore']}
                    >
                      <TenantLifecyclePage />
                    </ProtectedRoute>
                  }
                />

                {/* IDENTITY */}
                <Route
                  path="identity/users"
                  element={
                    <ProtectedRoute requiredCapability={['users_read', 'principal_kind_escalate']}>
                      <UsersPage />
                    </ProtectedRoute>
                  }
                />
                <Route
                  path="identity/memberships"
                  element={
                    <ProtectedRoute
                      requiredCapability={['memberships_grant', 'memberships_revoke']}
                    >
                      <MembershipsPage />
                    </ProtectedRoute>
                  }
                />
                <Route
                  path="identity/capabilities"
                  element={
                    <ProtectedRoute
                      // Either capability is sufficient to reach the page —
                      // grant operators see the Grant drawer; revoke-only
                      // operators can still hit the Revoke buttons. Per-button
                      // capability gates inside the page keep each action
                      // correctly scoped.
                      requiredCapability={['memberships_grant', 'memberships_revoke']}
                    >
                      <CapabilitiesAdminPage />
                    </ProtectedRoute>
                  }
                />
                <Route
                  path="identity/oauth-clients"
                  element={
                    <ProtectedRoute requiredCapability="oauth_client_write">
                      <OAuthClientsPage />
                    </ProtectedRoute>
                  }
                />

                {/* OPERATIONS */}
                <Route
                  path="ops/audit"
                  element={
                    <ProtectedRoute requiredCapability="audit_read">
                      <AuditPage />
                    </ProtectedRoute>
                  }
                />
                <Route
                  path="ops/impersonation"
                  element={
                    <ProtectedRoute requiredCapability="users_impersonate">
                      <ImpersonationListPage />
                    </ProtectedRoute>
                  }
                />
                <Route
                  path="ops/feature-flags"
                  element={
                    <ProtectedRoute requiredCapability="feature_flags_write">
                      <FeatureFlagsPage />
                    </ProtectedRoute>
                  }
                />

                {/* PLATFORM */}
                <Route
                  path="platform/organizations"
                  element={
                    <ProtectedRoute requiredCapability="agencies_read">
                      <OrganizationsPage />
                    </ProtectedRoute>
                  }
                />
                <Route
                  path="platform/settings"
                  element={
                    <ProtectedRoute requiredCapability="site_settings_write">
                      <PlatformPage />
                    </ProtectedRoute>
                  }
                />
                <Route
                  path="platform/mobile"
                  element={
                    <ProtectedRoute requiredCapability="mobile_config_write">
                      <MobileConfigPage />
                    </ProtectedRoute>
                  }
                />
                <Route
                  path="platform/health"
                  element={
                    <ProtectedRoute requiredCapability="audit_read">
                      <PlatformHealthPage />
                    </ProtectedRoute>
                  }
                />
                <Route
                  path="platform/announcements"
                  element={
                    <ProtectedRoute requiredCapability="site_settings_write">
                      <SystemAnnouncementsPage />
                    </ProtectedRoute>
                  }
                />
                <Route
                  path="platform/support-data"
                  element={
                    <ProtectedRoute requiredCapability="audit_read">
                      <SupportDataPage />
                    </ProtectedRoute>
                  }
                />
                <Route
                  path="platform/onboarding"
                  element={
                    <ProtectedRoute requiredCapability="site_settings_write">
                      <OnboardingToursPage />
                    </ProtectedRoute>
                  }
                />
                {/* LAYOUT — platform-principal gate; no capability registered yet
                    (layout_editor_* capability is a backend follow-up) */}
                <Route
                  path="platform/layout"
                  element={
                    <ProtectedRoute>
                      <LayoutEditorPage />
                    </ProtectedRoute>
                  }
                />
                <Route
                  path="platform/layout/manifests"
                  element={
                    <ProtectedRoute>
                      <LayoutManifestsPage />
                    </ProtectedRoute>
                  }
                />
              </Route>
            </Routes>
          </MfaWrapper>
        </ToastProvider>
      </MfaWindowProvider>
    </AdminAuthProvider>
  );
}
