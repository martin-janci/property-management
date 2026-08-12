/**
 * Protected Route Component.
 *
 * Wraps routes that require authentication.
 * Redirects to login if the user is not authenticated,
 * storing the current location for redirect after login.
 *
 * @see Story 79.2 - Authentication Flow Implementation
 */

import { setReturnUrl } from '@ppt/shared';
import type React from 'react';
import { useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { Navigate, useLocation } from 'react-router-dom';
import { useAuth } from '../contexts/AuthContext';
import './ProtectedRoute.css';

// ============================================================================
// Constants
// ============================================================================

const LOGIN_PATH = '/login';

// ============================================================================
// Types
// ============================================================================

export interface ProtectedRouteProps {
  /** The content to render when authenticated */
  children: React.ReactNode;
  /** Optional redirect path override (defaults to /login) */
  redirectTo?: string;
  /** Optional roles required to access the route */
  requiredRoles?: string[];
}

// ============================================================================
// Helper Functions
// ============================================================================

/**
 * Stores the current location as the return URL via @ppt/shared setReturnUrl.
 */
function storeReturnUrl(pathname: string, search: string): void {
  const returnUrl = `${pathname}${search}`;
  // Don't store the login page as return URL
  if (returnUrl !== LOGIN_PATH && !returnUrl.startsWith(`${LOGIN_PATH}?`)) {
    setReturnUrl(returnUrl);
  }
}

// ============================================================================
// Component
// ============================================================================

/**
 * Protected Route Component.
 *
 * Checks authentication status and redirects to login if not authenticated.
 * Shows a loading spinner while checking authentication state.
 *
 * @example
 * ```tsx
 * <Route
 *   path="/dashboard"
 *   element={
 *     <ProtectedRoute>
 *       <DashboardPage />
 *     </ProtectedRoute>
 *   }
 * />
 * ```
 *
 * @example With role requirement
 * ```tsx
 * <Route
 *   path="/admin"
 *   element={
 *     <ProtectedRoute requiredRoles={['admin']}>
 *       <AdminPage />
 *     </ProtectedRoute>
 *   }
 * />
 * ```
 */
export function ProtectedRoute({
  children,
  redirectTo = LOGIN_PATH,
  requiredRoles,
}: ProtectedRouteProps) {
  const { isAuthenticated, isLoading, user } = useAuth();
  const location = useLocation();
  const { t } = useTranslation();

  // Store return URL when redirecting to login
  useEffect(() => {
    if (!isLoading && !isAuthenticated) {
      storeReturnUrl(location.pathname, location.search);
    }
  }, [isLoading, isAuthenticated, location.pathname, location.search]);

  // Show loading spinner while checking auth state
  if (isLoading) {
    return (
      <div className="protected-route-loading">
        <div
          className="protected-route-spinner"
          aria-label={t('accessibility.checkingAuthentication', {
            defaultValue: 'Checking authentication',
          })}
        />
        <span className="protected-route-loading-text">{t('common.loading')}</span>
      </div>
    );
  }

  // Redirect to login if not authenticated
  if (!isAuthenticated) {
    return <Navigate to={redirectTo} replace />;
  }

  // Check role requirements if specified.
  // Deny-on-missing-role: if requiredRoles is set we MUST have a role to compare
  // against. Skipping when user.role is absent would be fail-open.
  if (requiredRoles && requiredRoles.length > 0) {
    const hasRequiredRole = user?.role != null && requiredRoles.includes(user.role);
    if (!hasRequiredRole) {
      // User is authenticated but lacks required role (or role not yet populated).
      return (
        <div className="protected-route-unauthorized">
          <h1>{t('errors.accessDenied', { defaultValue: 'Access Denied' })}</h1>
          <p>{t('errors.unauthorized')}</p>
        </div>
      );
    }
  }

  // User is authenticated (and has required role if specified)
  return <>{children}</>;
}

ProtectedRoute.displayName = 'ProtectedRoute';
