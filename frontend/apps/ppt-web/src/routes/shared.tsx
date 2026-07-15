/**
 * Shared route-wrapper helpers.
 *
 * Small utilities used by more than one feature route-group module. Kept here
 * (rather than duplicated per group) so the per-feature route files stay
 * conflict-isolated while still sharing the few genuinely cross-feature bits.
 */
import type { Building as ApiBuilding } from '@ppt/api-client';

/** Format an API Building address into a single display string. */
export function formatBuildingAddress(building: ApiBuilding): string {
  const { street, city, postalCode } = building.address;
  return `${street}, ${postalCode} ${city}`;
}

/** Transform an API Building into the `{ id, name, address }` shape UI pages expect. */
export function transformBuildingForUI(building: ApiBuilding): {
  id: string;
  name: string;
  address: string;
} {
  return {
    id: building.id,
    name: building.name,
    address: formatBuildingAddress(building),
  };
}

/**
 * Roles permitted to perform manager-level (operator) actions.
 *
 * Single source of truth shared by {@link isManagerRole} (nav/inline gating)
 * and `<ProtectedRoute requiredRoles={...}>` (route-level gating) so the two
 * cannot drift apart.
 */
export const MANAGER_ROLES = [
  'manager',
  'org_admin',
  'super_admin',
  'technical_manager',
  'property_manager',
] as const;

/**
 * Manager-equivalent role check shared by detail/workspace route wrappers.
 * Returns true for any role permitted to perform manager-level actions.
 */
export function isManagerRole(role: string | undefined): boolean {
  return role != null && (MANAGER_ROLES as readonly string[]).includes(role);
}
