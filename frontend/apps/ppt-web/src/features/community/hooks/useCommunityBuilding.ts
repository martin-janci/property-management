/**
 * Building context for the Community section.
 *
 * Every Community list on the server is scoped to a building
 * (`/api/v1/community/buildings/{building_id}/...`). The selectable buildings
 * are the caller's own unit buildings (`GET /users/me/units`, residents) plus,
 * for manager roles, the organization's buildings (`GET /buildings`). The
 * selection is persisted in the `?building=` query param so it survives
 * navigation between the Community tabs and page reloads; an unknown or
 * missing param falls back to the first available building.
 */

import { buildingKeys, listBuildings, useMyUnits } from '@ppt/api-client';
import { useQuery } from '@tanstack/react-query';
import { useCallback, useMemo } from 'react';
import { useSearchParams } from 'react-router-dom';
import { useAuth } from '../../../contexts';
import { isManagerRole } from '../../../routes/shared';

export interface CommunityBuildingOption {
  id: string;
  name: string;
}

export interface CommunityBuildingContext {
  buildings: CommunityBuildingOption[];
  /** The active building id, or `undefined` while loading / when none exist. */
  buildingId: string | undefined;
  setBuildingId: (id: string) => void;
  isLoading: boolean;
  error: unknown;
}

const BUILDING_PARAMS = { pageSize: 100 } as const;

export function useCommunityBuilding(): CommunityBuildingContext {
  const { user } = useAuth();
  const isManager = isManagerRole(user?.role);
  const [searchParams, setSearchParams] = useSearchParams();

  const units = useMyUnits();
  const orgBuildings = useQuery({
    queryKey: buildingKeys.list(BUILDING_PARAMS),
    queryFn: ({ signal }) => listBuildings(BUILDING_PARAMS, signal),
    enabled: isManager,
    staleTime: 30_000,
  });

  const buildings = useMemo(() => {
    const seen = new Map<string, CommunityBuildingOption>();
    for (const u of units.data ?? []) {
      if (!u.building?.id || seen.has(u.building.id)) continue;
      const name = u.building.name || `${u.building.street}, ${u.building.city}`;
      seen.set(u.building.id, { id: u.building.id, name });
    }
    for (const b of orgBuildings.data?.items ?? []) {
      if (seen.has(b.id)) continue;
      seen.set(b.id, { id: b.id, name: b.name || b.address?.street || b.id });
    }
    return [...seen.values()];
  }, [units.data, orgBuildings.data]);

  const requested = searchParams.get('building');
  const buildingId =
    requested && buildings.some((b) => b.id === requested) ? requested : buildings[0]?.id;

  const setBuildingId = useCallback(
    (id: string) => {
      setSearchParams(
        (prev) => {
          const next = new URLSearchParams(prev);
          next.set('building', id);
          return next;
        },
        { replace: true }
      );
    },
    [setSearchParams]
  );

  const isLoading = units.isLoading || (isManager && orgBuildings.isLoading);
  // Only an error when nothing could be loaded at all — a resident whose org
  // listing fails still has their unit buildings.
  const error = buildings.length === 0 ? (units.error ?? orgBuildings.error ?? null) : null;

  return { buildings, buildingId, setBuildingId, isLoading, error };
}

/** Append `?building=<id>` to a Community path so the selection carries over. */
export function withBuilding(path: string, buildingId: string | undefined): string {
  if (!buildingId) return path;
  const sep = path.includes('?') ? '&' : '?';
  return `${path}${sep}building=${encodeURIComponent(buildingId)}`;
}
