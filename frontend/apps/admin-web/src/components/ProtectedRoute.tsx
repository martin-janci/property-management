import type { ReactNode } from 'react';
import { Navigate, useLocation } from 'react-router-dom';

import { useAdminAuth } from '../auth/AdminAuthContext';
import { usePrincipalCapabilities } from '../auth/usePrincipalCapabilities';

/**
 * Guards the admin tree:
 *   - Unauthenticated → redirect `/login`.
 *   - Authenticated but not a platform principal (tenant user with a valid
 *     access token) → 403 page. Reaching the admin host with org-scoped
 *     credentials must not render the admin shell, even read-only.
 *   - Authenticated platform principal → render `children`.
 *
 * The platform-principal check waits for `usePrincipalCapabilities` to
 * resolve; until then it shows a minimal loading state to avoid flashing
 * the dashboard at non-admins on first paint.
 */
export function ProtectedRoute({ children }: { children: ReactNode }) {
  const auth = useAdminAuth();
  const location = useLocation();
  const { isPlatformPrincipal, isLoading } = usePrincipalCapabilities();

  if (!auth.isAuthenticated) {
    return <Navigate to="/login" state={{ from: location.pathname }} replace />;
  }
  if (isLoading) {
    return <p aria-live="polite">Verifying access…</p>;
  }
  if (!isPlatformPrincipal) {
    return (
      <section role="alert">
        <h1>403 — Not a platform principal</h1>
        <p>
          Your account is authenticated but does not have super-admin capabilities. If you believe
          this is wrong, contact the platform operator.
        </p>
      </section>
    );
  }
  return <>{children}</>;
}
