/**
 * Phase 5 — admin section router.
 *
 * Mounts under `/admin/*` inside ppt-web's `<BrowserRouter>`. Wrap the entire
 * tree with `<CapabilityProvider>` so child pages and `<ResourceTable>`
 * actions can hide affordances the user lacks capabilities for.
 *
 * The router does not perform a hard auth check itself — that is the host
 * app's job (`ProtectedRoute`). It does, however, refuse to render anything
 * if the principal is not a platform-kind user (leak #21).
 */

import { CapabilityProvider, useCapabilityChecker } from '@ppt/admin-ui';
import type { ReactNode } from 'react';
import { Navigate, Route, Routes } from 'react-router-dom';
import {
  AgenciesPage,
  AuditPage,
  FeatureFlagsPage,
  PlatformPage,
  UsersPage,
} from './pages';

interface AdminRouterProps {
  /** Capabilities the current principal holds (loaded at app boot). */
  capabilities: ReadonlyArray<import('@ppt/admin-ui').Capability>;
  /** Whether the current principal is platform-kind. */
  isPlatformPrincipal: boolean;
}

export function AdminRouter({ capabilities, isPlatformPrincipal }: AdminRouterProps) {
  return (
    <CapabilityProvider value={{ capabilities, isPlatformPrincipal }}>
      <RequirePlatformPrincipal>
        <Routes>
          <Route index element={<Navigate to="agencies" replace />} />
          <Route path="agencies" element={<AgenciesPage />} />
          <Route path="users" element={<UsersPage />} />
          <Route path="feature-flags" element={<FeatureFlagsPage />} />
          <Route path="audit" element={<AuditPage />} />
          <Route path="platform" element={<PlatformPage />} />
          <Route path="*" element={<Navigate to="agencies" replace />} />
        </Routes>
      </RequirePlatformPrincipal>
    </CapabilityProvider>
  );
}

function RequirePlatformPrincipal({ children }: { children: ReactNode }) {
  const { isPlatformPrincipal } = useCapabilityChecker();
  if (!isPlatformPrincipal) {
    // Render nothing rather than a "you do not have permission" page —
    // information disclosure is leak #21.
    return <Navigate to="/" replace />;
  }
  return <>{children}</>;
}
