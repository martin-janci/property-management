import { useCapabilityChecker } from '@ppt/admin-ui';
import type { ReactNode } from 'react';
import { Navigate, useLocation } from 'react-router-dom';

import { useAdminAuth } from '../auth/AdminAuthContext';
import { usePrincipalCapabilities } from '../auth/usePrincipalCapabilities';
import { ForbiddenPage } from './ForbiddenPage';

interface ProtectedRouteProps {
  children: ReactNode;
  /**
   * Optional capability gate. If provided, the user must hold AT LEAST ONE
   * of the listed capabilities; otherwise a 403 page is shown.
   * The platform-principal check always runs regardless of this prop.
   */
  requiredCapability?: string | string[];
}

/**
 * Guards the admin tree:
 *   - Unauthenticated → redirect `/login`.
 *   - Authenticated but not a platform principal (tenant user with a valid
 *     access token) → 403 page. Reaching the admin host with org-scoped
 *     credentials must not render the admin shell, even read-only.
 *   - Authenticated platform principal → render `children`.
 *   - `requiredCapability` provided but user lacks ALL of them → 403 page.
 *
 * The platform-principal check waits for `usePrincipalCapabilities` to
 * resolve; until then it shows a minimal loading state to avoid flashing
 * the dashboard at non-admins on first paint.
 */
export function ProtectedRoute({ children, requiredCapability }: ProtectedRouteProps) {
  const auth = useAdminAuth();
  const location = useLocation();
  const { isPlatformPrincipal, isLoading } = usePrincipalCapabilities();
  const { capabilities } = useCapabilityChecker();

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

  // Per-route capability check: user must hold at least one of the required caps.
  if (requiredCapability !== undefined) {
    const required = Array.isArray(requiredCapability) ? requiredCapability : [requiredCapability];
    const hasAny = required.some((cap) => capabilities.includes(cap as never));
    if (!hasAny) {
      return <ForbiddenPage requiredCapability={required} />;
    }
  }

  return <>{children}</>;
}
