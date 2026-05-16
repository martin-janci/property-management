/**
 * Typed feature-flag accessors for reality-web (Phase 3).
 *
 * Two surfaces:
 *
 * - `getFeatureFlag(key)`  — server-side. Reads from the per-request
 *                            `getTenantConfig()` cache.
 * - `useFeatureFlag(key)`  — client-side. Reads from the embedded
 *                            `window.__TENANT_CONFIG__` JSON the SSR
 *                            layer injects on every render.
 *
 * Special-cased keys consumed by the layout:
 *
 * - `building_disabled` — when true, the layout renders a 503 page
 *   instead of the normal app. This is the per-tenant kill switch
 *   (defends operational leak #22).
 */

import { getFeatureFlag as serverGetFeatureFlag } from './tenant-config';

export type FeatureFlagState = {
  enabled: boolean;
  value?: unknown;
};

/** Reserved kill-switch flag keys. Add to this enum as new ones land. */
export const FeatureFlags = {
  BUILDING_DISABLED: 'building_disabled',
} as const;

export type FeatureFlagKey = (typeof FeatureFlags)[keyof typeof FeatureFlags] | string;

/** Server-side read — re-export to keep one canonical import path. */
export const getFeatureFlag = serverGetFeatureFlag;

/**
 * Server-side narrow: is this kill switch on?
 *
 * Falls back to `false` (feature enabled / app reachable) on any error,
 * so a transient api-server outage does not look like a global outage.
 */
export async function isKillSwitchOn(key: FeatureFlagKey): Promise<boolean> {
  const f = await serverGetFeatureFlag(key);
  return Boolean(f.enabled);
}

// ---------------------------------------------------------------------------
// Client-side
// ---------------------------------------------------------------------------

declare global {
  interface Window {
    __TENANT_CONFIG__?: {
      feature_flags?: Record<string, FeatureFlagState>;
      tenant_id?: string | null;
    };
  }
}

/**
 * Client-side feature-flag hook.
 *
 * Reads from `window.__TENANT_CONFIG__` which the SSR layer embeds in
 * every page render via a small inline `<script>` (see
 * `app/[locale]/layout.tsx`). No fetch happens client-side — the server
 * already paid for it.
 */
export function useFeatureFlag(key: FeatureFlagKey): FeatureFlagState {
  if (typeof window === 'undefined') {
    return { enabled: false };
  }
  return window.__TENANT_CONFIG__?.feature_flags?.[key] ?? { enabled: false };
}
