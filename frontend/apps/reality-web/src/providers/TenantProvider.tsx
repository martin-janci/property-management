'use client';

/**
 * TenantProvider — client-side React context for tenant config (Phase 6: C1).
 *
 * The SSR layer (app/[locale]/layout.tsx) serialises a projection of
 * the tenant config into `window.__TENANT_CONFIG__` via an inline script.
 * This provider reads that bootstrap value on first render and makes the
 * data available via React context — so client components don't need to
 * fetch the config themselves.
 *
 * Only `tenant_id` and `feature_flags` are included in the bootstrap
 * payload (branding is applied as CSS variables directly on `<html>` by
 * the SSR layout and needs no client-side representation).
 *
 * Usage (client component):
 *
 *   const { tenantId, featureFlags } = useTenantContext();
 *
 * The provider is mounted inside the locale layout, so it is available
 * everywhere below that boundary. Server components can call
 * `getTenantConfig()` from `@/lib/tenant-config` directly instead.
 */

import { createContext, type ReactNode, useContext, useMemo } from 'react';

export type ClientTenantConfig = {
  tenant_id: string | null;
  feature_flags: Record<string, { enabled: boolean; value?: unknown }>;
};

type TenantContextValue = {
  tenantId: string | null;
  featureFlags: Record<string, { enabled: boolean; value?: unknown }>;
  isWhiteLabel: boolean;
};

const TenantContext = createContext<TenantContextValue>({
  tenantId: null,
  featureFlags: {},
  isWhiteLabel: false,
});

function readBootstrap(): ClientTenantConfig {
  if (typeof window === 'undefined') {
    return { tenant_id: null, feature_flags: {} };
  }
  const raw = (window as Window & { __TENANT_CONFIG__?: ClientTenantConfig }).__TENANT_CONFIG__;
  if (!raw) return { tenant_id: null, feature_flags: {} };
  return raw;
}

interface TenantProviderProps {
  children: ReactNode;
  /**
   * Optional override for SSR — pass the serialised tenant config from
   * the server layout so the provider is hydration-safe even on first
   * paint (before the inline script runs).
   */
  initial?: ClientTenantConfig;
}

export function TenantProvider({ children, initial }: TenantProviderProps) {
  const config = initial ?? readBootstrap();

  const value = useMemo<TenantContextValue>(
    () => ({
      tenantId: config.tenant_id,
      featureFlags: config.feature_flags ?? {},
      isWhiteLabel: config.tenant_id !== null,
    }),
    // Stable after hydration — the bootstrap blob never changes during the
    // client session. If the tenant branding is updated the page must be
    // refreshed to pick up a new SSR render. Use the object ref of
    // feature_flags instead of JSON.stringify (avoids re-serialization on
    // every render and key-order false-positives — review R7).
    [config.tenant_id, config.feature_flags]
  );

  return <TenantContext.Provider value={value}>{children}</TenantContext.Provider>;
}

/**
 * Access the tenant context from any client component.
 *
 * Returns sane defaults (`tenantId: null`, empty `featureFlags`) when
 * called outside the provider tree (e.g. in Storybook or isolated tests).
 */
export function useTenantContext(): TenantContextValue {
  return useContext(TenantContext);
}
