import { CapabilityProvider } from '@ppt/admin-ui';
import type { ReactNode } from 'react';
import { Route, Routes } from 'react-router-dom';

import { AdminAuthProvider } from './auth/AdminAuthContext';
import { usePrincipalCapabilities } from './auth/usePrincipalCapabilities';
import { AdminLayout } from './components/AdminLayout';
import { ImpersonationWrapper } from './components/ImpersonationWrapper';
import { MfaWrapper } from './components/MfaWrapper';
import { ProtectedRoute } from './components/ProtectedRoute';
import { ToastProvider } from './components/Toast';
import AgenciesPage from './pages/agencies';
import AuditPage from './pages/audit';
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

export function App() {
  return (
    <AdminAuthProvider>
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
              <Route index element={<Dashboard />} />
              <Route path="agencies" element={<AgenciesPage />} />
              <Route path="users" element={<UsersPage />} />
              <Route path="audit" element={<AuditPage />} />
              <Route path="feature-flags" element={<FeatureFlagsPage />} />
              <Route path="platform" element={<PlatformPage />} />
            </Route>
          </Routes>
        </MfaWrapper>
      </ToastProvider>
    </AdminAuthProvider>
  );
}
