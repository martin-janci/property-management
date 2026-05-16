import type { ReactNode } from 'react';
import { Navigate, useLocation } from 'react-router-dom';

import { useAdminAuth } from '../auth/AdminAuthContext';

export function ProtectedRoute({ children }: { children: ReactNode }) {
  const auth = useAdminAuth();
  const location = useLocation();
  if (!auth.isAuthenticated) {
    return <Navigate to="/login" state={{ from: location.pathname }} replace />;
  }
  return <>{children}</>;
}
