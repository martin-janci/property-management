import { type ReactNode, useMemo } from 'react';
import { OrganizationContext, type OrganizationContextValue } from '../hooks/useOrganization';
import { useAuth } from './AuthContext';

export function OrganizationProvider({ children }: { children: ReactNode }) {
  const { user } = useAuth();
  const organizationId = user?.organizationId;
  const organizationName = user?.organizationName;

  // Memoize the context value so consumers only re-render when the
  // organization identity actually changes — not on every AuthProvider
  // render. Mirrors AuthProvider's memoized contextValue.
  const value = useMemo<OrganizationContextValue | null>(
    () => (organizationId ? { organizationId, organizationName } : null),
    [organizationId, organizationName]
  );

  return <OrganizationContext.Provider value={value}>{children}</OrganizationContext.Provider>;
}
