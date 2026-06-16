import { useAuth } from '../../../contexts/AuthContext';

/**
 * Hook to provide authentication headers for accounting API calls.
 *
 * @returns An object containing the headers with Authorization and X-Tenant-ID,
 *          or null if the user is not authenticated or has no active organization.
 */
export function useAccountingAuth() {
  const { getAccessToken, user } = useAuth();
  const token = getAccessToken();
  const tenantId = user?.organizationId;

  if (!token || !tenantId) return null;

  return {
    headers: {
      Authorization: `Bearer ${token}`,
      'X-Tenant-ID': tenantId,
    },
  };
}
