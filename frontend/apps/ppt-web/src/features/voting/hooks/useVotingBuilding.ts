/**
 * Building scope for the voting pages (Epic 5, screens `ppt/voting`,
 * `ppt/vote-create`).
 *
 * Votes are building-scoped, so the list and create pages need to know which
 * building the user is working in. Previously both pages silently used the
 * first building of the organization. This hook:
 *
 * - merges the organization's buildings with the buildings the caller
 *   actually lives in (`GET /api/v1/users/me/units`), listing the caller's
 *   own buildings first so residents land on their building by default;
 * - reads/writes the selection in the `?building=` URL search param so the
 *   choice survives reloads, back/forward and links to `/voting/new`;
 * - ignores a `?building=` value that is not one of the available buildings.
 */
import { useBuildings, useMyUnits } from '@ppt/api-client';
import { useCallback, useMemo } from 'react';
import { useSearchParams } from 'react-router-dom';

/** URL search param holding the selected building id. */
export const VOTING_BUILDING_PARAM = 'building';

export interface VotingBuildingOption {
  id: string;
  label: string;
  /** True when the caller has an active residency in this building. */
  isMine: boolean;
}

interface BuildingLike {
  id: string;
  name?: string | null;
  street?: string;
  city?: string;
  address?: { street?: string; city?: string };
}

function labelFor(b: BuildingLike): string {
  const name = b.name?.trim();
  if (name) return name;
  const street = b.address?.street ?? b.street ?? '';
  const city = b.address?.city ?? b.city ?? '';
  return [street, city].filter(Boolean).join(', ') || b.id;
}

/**
 * Pure merge of org buildings + the caller's own buildings. The caller's
 * buildings come first (in org-list order, then any the org list did not
 * return — e.g. when a resident may not list all org buildings).
 */
export function buildVotingBuildingOptions(
  orgBuildings: BuildingLike[],
  myBuildings: BuildingLike[]
): VotingBuildingOption[] {
  const mine = new Set(myBuildings.map((b) => b.id));
  const seen = new Set<string>();
  const options: VotingBuildingOption[] = [];
  const push = (b: BuildingLike) => {
    if (!b?.id || seen.has(b.id)) return;
    seen.add(b.id);
    options.push({ id: b.id, label: labelFor(b), isMine: mine.has(b.id) });
  };
  for (const b of orgBuildings) if (mine.has(b.id)) push(b);
  for (const b of myBuildings) push(b);
  for (const b of orgBuildings) push(b);
  return options;
}

export function useVotingBuilding() {
  const [searchParams, setSearchParams] = useSearchParams();
  const { data: buildingsData, isLoading: isLoadingBuildings } = useBuildings();
  const { data: myUnits, isLoading: isLoadingMyUnits } = useMyUnits();

  const options = useMemo(
    () =>
      buildVotingBuildingOptions(
        buildingsData?.items ?? [],
        (myUnits ?? []).map((u) => u.building).filter(Boolean)
      ),
    [buildingsData, myUnits]
  );

  const requested = searchParams.get(VOTING_BUILDING_PARAM) ?? '';
  const isLoading = isLoadingBuildings || isLoadingMyUnits;
  // While the lists load, trust the URL so the votes query does not flip
  // from "first building" to the requested one once data arrives.
  const buildingId = options.some((o) => o.id === requested)
    ? requested
    : isLoading
      ? requested
      : (options[0]?.id ?? '');

  const setBuildingId = useCallback(
    (id: string) => {
      setSearchParams(
        (prev) => {
          const next = new URLSearchParams(prev);
          if (id) next.set(VOTING_BUILDING_PARAM, id);
          else next.delete(VOTING_BUILDING_PARAM);
          return next;
        },
        { replace: true }
      );
    },
    [setSearchParams]
  );

  return { buildingId, options, setBuildingId, isLoading };
}
