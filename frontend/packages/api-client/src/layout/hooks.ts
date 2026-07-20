/**
 * Layout Query Hooks
 *
 * TanStack Query hooks for resolved screen layouts.
 */

import { useQuery } from '@tanstack/react-query';
import { fetchResolvedLayout, type ResolvedScreen } from './api';

export const layoutKeys = {
  all: ['layout'] as const,
  resolved: (screen: string, platform: string) =>
    [...layoutKeys.all, 'resolved', screen, platform] as const,
};

export function useResolvedLayout(screen: string, platform: 'web' | 'mobile' = 'web') {
  return useQuery<ResolvedScreen>({
    queryKey: layoutKeys.resolved(screen, platform),
    queryFn: () => fetchResolvedLayout(screen, platform),
    staleTime: 60_000,
    retry: 1,
  });
}
