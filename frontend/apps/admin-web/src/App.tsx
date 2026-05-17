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
import AgenciesPage from './pages/agencies';
import AuditPage from './pages/audit';
import CapabilitiesAdminPage from './pages/CapabilitiesAdminPage';
import { Dashboard } from './pages/Dashboard';
import FeatureFlagsPage from './pages/feature-flags';
import { LoginPage } from './pages/LoginPage';
import PlatformPage from './pages/platform';
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

/** Placeholder for pages being implemented in Round 2. */
function PlaceholderPage({ title }: { title: string }) {
  return (
    <section style={{ padding: '32px' }}>
      <h1 style={{ fontSize: '1.5rem', fontWeight: 700, color: 'var(--ppt-fg-primary, #111827)' }}>
        {title}
      </h1>
      <p style={{ color: 'var(--ppt-fg-muted, #6b7280)', marginTop: '8px' }}>
        {title} page coming in Round 2
      </p>
    </section>
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
                  path="tenants/lifecycle"
                  element={
                    <ProtectedRoute
                      requiredCapability={['tenant_export', 'tenant_purge', 'tenant_restore']}
                    >
                      <PlaceholderPage title="Lifecycle" />
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
                      <PlaceholderPage title="Memberships" />
                    </ProtectedRoute>
                  }
                />
                <Route
                  path="identity/capabilities"
                  element={
                    <ProtectedRoute requiredCapability="memberships_grant">
                      <CapabilitiesAdminPage />
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
                      <PlaceholderPage title="Impersonation" />
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
                      <PlaceholderPage title="Mobile config" />
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
