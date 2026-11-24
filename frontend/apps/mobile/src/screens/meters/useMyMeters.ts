/**
 * Meters of the signed-in resident's own units.
 *
 * The login response and token claims carry no unit id, so `user.unitId` is
 * normally unset. The units come from `GET /api/v1/users/me/units` (camelCase
 * `MyUnit[]`, the same source PersonMonthsScreen uses) and each unit's meters
 * from `GET /api/v1/meters/units/{unit_id}` (bare `Vec<Meter>`). A resident
 * with several units sees the meters of all of them.
 */

import { useQuery } from '@tanstack/react-query';
import { useAuth } from '../../contexts/AuthContext';
import { apiRequest } from '../../hooks/useApi';

/** Pull the unit ids out of a `MyUnit[]` payload, tolerating malformed rows. */
export function parseMyUnitIds(data: unknown): string[] {
  if (!Array.isArray(data)) return [];
  const ids: string[] = [];
  for (const row of data) {
    const unit = (row as { unit?: { id?: unknown } } | null)?.unit;
    if (unit && typeof unit.id === 'string' && unit.id && !ids.includes(unit.id)) {
      ids.push(unit.id);
    }
  }
  return ids;
}

export interface MyMetersResult {
  /** Units the meters were loaded for; empty when the user has no unit. */
  unitIds: string[];
  /** Raw meter rows of all units, in unit order. */
  meters: unknown[];
}

export async function fetchMyMeters(
  knownUnitId: string | undefined,
  signal?: AbortSignal
): Promise<MyMetersResult> {
  const unitIds = knownUnitId
    ? [knownUnitId]
    : parseMyUnitIds(
        await apiRequest<unknown>('/api/v1/users/me/units', { method: 'GET' }, { signal })
      );
  const lists = await Promise.all(
    unitIds.map((id) =>
      apiRequest<unknown>(`/api/v1/meters/units/${id}`, { method: 'GET' }, { signal })
    )
  );
  const meters = lists.flatMap((list) => {
    if (Array.isArray(list)) return list;
    const wrapped = (list as { meters?: unknown } | null)?.meters;
    return Array.isArray(wrapped) ? wrapped : [];
  });
  return { unitIds, meters };
}

export function useMyMeters() {
  const { user } = useAuth();
  const knownUnitId = user?.unitId;
  return useQuery<MyMetersResult, Error>({
    queryKey: ['meters', 'mine', user?.id ?? null, knownUnitId ?? null],
    queryFn: ({ signal }) => fetchMyMeters(knownUnitId, signal),
    staleTime: 30_000,
    enabled: !!user,
  });
}
