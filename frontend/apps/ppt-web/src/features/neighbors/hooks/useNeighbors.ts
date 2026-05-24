/**
 * Neighbor Feature Hooks
 *
 * Wires @ppt/api-client neighbor + privacy-settings routes to the
 * feature-layer NeighborView / PrivacySettings types used by the
 * presentational components (Epic 6, Story 6.6).
 *
 * API NeighborView  →  feature NeighborView mapping:
 *   userId       → id
 *   displayName  → displayName
 *   unitLabel    → unitNumber
 *   isVisible    → (used for filtering)
 *   email        → email
 *   phone        → phone
 *   residentType → isOwner (owner → true)
 */

import type {
  NeighborView as ApiNeighborView,
  PrivacySettings as ApiPrivacySettings,
  UpdatePrivacySettingsRequest as ApiUpdatePrivacySettingsRequest,
} from '@ppt/api-client';
import { createNeighborHooks, createNeighborsApi, getToken } from '@ppt/api-client';
import { useMemo } from 'react';
import type { NeighborView, PrivacySettings } from '../types';

// ---------------------------------------------------------------------------
// API client factory
// ---------------------------------------------------------------------------

const API_BASE_URL = import.meta.env.VITE_API_BASE_URL ?? '';

function getNeighborsApiClient() {
  return createNeighborsApi({
    baseUrl: API_BASE_URL,
    accessToken: getToken() ?? undefined,
  });
}

// ---------------------------------------------------------------------------
// Type adapters: API → feature layer
// ---------------------------------------------------------------------------

/**
 * Map API NeighborView to the feature-layer NeighborView type.
 * Privacy-aware: hidden neighbors have no email/phone.
 */
export function mapApiNeighborToFeature(n: ApiNeighborView): NeighborView {
  return {
    id: n.userId,
    displayName: n.displayName,
    unitNumber: n.unitLabel,
    email: n.email ?? undefined,
    phone: n.phone ?? undefined,
    isOwner: n.residentType === 'owner',
  };
}

/**
 * Map API PrivacySettings to the feature-layer PrivacySettings type.
 * The API uses a simplified `profile_visibility` + `show_contact_info`
 * model; we map it to the more granular feature-layer visibility levels.
 */
export function mapApiPrivacyToFeature(api: ApiPrivacySettings): PrivacySettings {
  // 'visible' → 'building', 'hidden' → 'private', 'contacts_only' → 'neighbors'
  const visibilityMap: Record<string, PrivacySettings['showName']> = {
    visible: 'building',
    contacts_only: 'neighbors',
    hidden: 'private',
  };
  const visibility = visibilityMap[api.profileVisibility] ?? 'building';
  const contactVisibility = api.showContactInfo ? 'neighbors' : 'private';

  return {
    showName: visibility,
    showEmail: contactVisibility,
    showPhone: contactVisibility,
    showUnit: visibility,
    showAvatar: visibility,
    showBio: visibility,
    showMoveInDate: 'private',
    listedInDirectory: api.profileVisibility !== 'hidden',
  };
}

/**
 * Map feature-layer PrivacySettings to the API UpdatePrivacySettingsRequest.
 * We collapse the granular visibility flags back to the three-value API enum.
 */
export function mapFeaturePrivacyToApi(settings: PrivacySettings): ApiUpdatePrivacySettingsRequest {
  let profileVisibility: ApiUpdatePrivacySettingsRequest['profileVisibility'];
  if (!settings.listedInDirectory || settings.showName === 'private') {
    profileVisibility = 'hidden';
  } else if (settings.showName === 'neighbors') {
    profileVisibility = 'contacts_only';
  } else {
    profileVisibility = 'visible';
  }

  const showContactInfo =
    settings.showEmail === 'neighbors' ||
    settings.showEmail === 'building' ||
    settings.showEmail === 'public';

  return { profileVisibility, showContactInfo };
}

// ---------------------------------------------------------------------------
// Hooks
// ---------------------------------------------------------------------------

/**
 * List neighbors in a building, mapping to feature-layer types.
 * Only includes neighbors who have not hidden their profile.
 */
export function useNeighbors(buildingId: string, enabled = true) {
  const api = useMemo(() => getNeighborsApiClient(), []);
  const hooks = useMemo(() => createNeighborHooks(api), [api]);
  const query = hooks.useNeighbors(buildingId, enabled && !!buildingId);

  const neighbors: NeighborView[] = useMemo(
    () => (query.data?.neighbors ?? []).filter((n) => n.isVisible).map(mapApiNeighborToFeature),
    [query.data]
  );

  return {
    neighbors,
    total: query.data?.total ?? 0,
    isLoading: query.isLoading,
    error: query.error instanceof Error ? query.error.message : null,
  };
}

/**
 * Get + update current user's privacy settings.
 */
export function usePrivacySettings() {
  const api = useMemo(() => getNeighborsApiClient(), []);
  const hooks = useMemo(() => createNeighborHooks(api), [api]);

  const query = hooks.usePrivacySettings();
  const mutation = hooks.useUpdatePrivacySettings();

  const settings: PrivacySettings | undefined = query.data
    ? mapApiPrivacyToFeature(query.data.settings)
    : undefined;

  const updateSettings = (newSettings: PrivacySettings) => {
    return mutation.mutateAsync(mapFeaturePrivacyToApi(newSettings));
  };

  return {
    settings,
    isLoading: query.isLoading,
    isSubmitting: mutation.isPending,
    error:
      (query.error instanceof Error ? query.error.message : null) ??
      (mutation.error instanceof Error ? mutation.error.message : null),
    updateSettings,
  };
}
