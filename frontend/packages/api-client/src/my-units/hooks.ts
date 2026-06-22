/**
 * My Unit (resident-facing) hooks — Epic 3, Story 3.6.
 */

import { useQuery } from '@tanstack/react-query';
import * as api from './api';

export const myUnitsKeys = {
  all: ['my-units'] as const,
  list: () => [...myUnitsKeys.all, 'list'] as const,
};

/** Fetch the authenticated resident's unit(s) for the "My Unit" view. */
export function useMyUnits() {
  return useQuery({
    queryKey: myUnitsKeys.list(),
    queryFn: () => api.getMyUnits(),
  });
}
