import type { ReactNode } from 'react';
import { OrganizationContext } from '../hooks/useOrganization';
import { useAuth } from './AuthContext';

export function OrganizationProvider({ children }: { children: ReactNode }) {
  const { user } = useAuth();

  const value = user?.organizationId
    ? { organizationId: user.organizationId, organizationName: user.organizationName }
    : null;

  return <OrganizationContext.Provider value={value}>{children}</OrganizationContext.Provider>;
}
