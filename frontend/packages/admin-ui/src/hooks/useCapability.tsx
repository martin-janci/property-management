/**
 * Capability checker hook + provider.
 *
 * The provider is fed the current user's capability set (typically loaded
 * from `GET /admin/capabilities/users/:me` at app boot). The hook returns
 * `true` if the user holds the queried capability, `false` otherwise.
 *
 * UI affordances should be HIDDEN (not just disabled) when a capability is
 * missing — Phase 5's brainstorming session calls out leak #21: showing a
 * disabled "delete tenant" button to a non-platform user is information
 * disclosure.
 */

import { createContext, type ReactNode, useContext, useMemo } from 'react';
import type { Capability } from '../capabilities';

export interface CapabilityCheckerOptions {
  /**
   * The set of capabilities the user holds. Empty set = no admin
   * affordances visible.
   */
  capabilities: ReadonlyArray<Capability>;
  /**
   * Whether the user is a platform-kind principal at all. If false, the
   * admin tree should not be rendered. The `<RequirePlatformPrincipal>`
   * gate in the router uses this.
   */
  isPlatformPrincipal: boolean;
}

const CapabilityContext = createContext<CapabilityCheckerOptions>({
  capabilities: [],
  isPlatformPrincipal: false,
});

export function CapabilityProvider({
  value,
  children,
}: {
  value: CapabilityCheckerOptions;
  children: ReactNode;
}) {
  // useMemo so consumers don't re-render every parent re-render.
  const memo = useMemo(
    () => ({
      capabilities: value.capabilities,
      isPlatformPrincipal: value.isPlatformPrincipal,
    }),
    [value.capabilities, value.isPlatformPrincipal]
  );
  return <CapabilityContext.Provider value={memo}>{children}</CapabilityContext.Provider>;
}

/** Returns the full checker context. Useful for `<RequirePlatformPrincipal>`. */
export function useCapabilityChecker(): CapabilityCheckerOptions {
  return useContext(CapabilityContext);
}

/** Returns true iff the current user holds `capability`. */
export function useCapability(capability: Capability): boolean {
  const ctx = useContext(CapabilityContext);
  if (!ctx.isPlatformPrincipal) return false;
  return ctx.capabilities.includes(capability);
}
