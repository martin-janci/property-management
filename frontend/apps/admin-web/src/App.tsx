import { Route, Routes } from 'react-router-dom';

import { AdminAuthProvider } from './auth/AdminAuthContext';
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
                  <AdminLayout />
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
