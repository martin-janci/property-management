/**
 * Load + patch a singleton settings document (`GET` / `PATCH` on one URL).
 *
 * Backs the platform-settings and mobile-config pages, which used to render
 * hard-coded defaults read-only although both endpoints exist on api-server.
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useAdminAuth } from '../auth/AdminAuthContext';

async function readError(res: Response, fallback: string): Promise<string> {
  try {
    const body = (await res.json()) as { message?: unknown; error?: unknown };
    if (typeof body.message === 'string' && body.message) return body.message;
    if (typeof body.error === 'string' && body.error) return body.error;
  } catch {
    // Non-JSON error body: fall through to the status-based message.
  }
  return `${fallback} (${res.status})`;
}

export function useSettingsResource<TDoc, TPatch>(url: string, queryKey: readonly unknown[]) {
  const { token } = useAdminAuth();
  const queryClient = useQueryClient();

  const query = useQuery<TDoc>({
    queryKey,
    queryFn: async () => {
      const headers: Record<string, string> = {};
      if (token) headers.Authorization = `Bearer ${token}`;
      const res = await fetch(url, { headers, credentials: 'include' });
      if (!res.ok) throw new Error(await readError(res, 'Failed to load settings'));
      return (await res.json()) as TDoc;
    },
    staleTime: 15_000,
    retry: 1,
  });

  const save = useMutation<TDoc, Error, TPatch>({
    mutationFn: async (patch) => {
      const headers: Record<string, string> = { 'Content-Type': 'application/json' };
      if (token) headers.Authorization = `Bearer ${token}`;
      const res = await fetch(url, {
        method: 'PATCH',
        headers,
        credentials: 'include',
        body: JSON.stringify(patch),
      });
      if (!res.ok) throw new Error(await readError(res, 'Failed to save settings'));
      return (await res.json()) as TDoc;
    },
    onSuccess: (doc) => {
      queryClient.setQueryData(queryKey, doc);
    },
  });

  return { query, save };
}
