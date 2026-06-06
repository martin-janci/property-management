import { useEffect } from 'react';
import { deepLinkManager } from '../qrcode';
import { resolveDeepLinkTarget } from '../services/deepLinkRouting';

/**
 * Deep-link wiring (gap-85-3).
 *
 * Extracted from `App.tsx` so the glue between the {@link deepLinkManager} and
 * the app's `handleNavigate` lives in one focused, churn-isolated module.
 * Future deep-link additions touch this hook (and `resolveDeepLinkTarget`)
 * instead of the App monolith.
 *
 * Behaviour is identical to the two effects it replaces:
 *  1. Register a handler that maps incoming parsed links → screen+params and
 *     forwards them to `onNavigate`, then kick off `deepLinkManager.initialize()`
 *     (cold-start initial URL + runtime `url` events). Cleanup unsubscribes.
 *  2. Keep the manager's auth gate in sync so auth-required links queued while
 *     logged out are dispatched the moment the user authenticates.
 *
 * @param onNavigate stable navigation callback (App memoises this with `useCallback`)
 * @param isAuthenticated current auth state, forwarded to the manager's gate
 */
export function useDeepLinkRouting(
  onNavigate: (screen: string, params?: Record<string, unknown>) => void,
  isAuthenticated: boolean
): void {
  useEffect(() => {
    const unsubscribe = deepLinkManager.addHandler((link) => {
      const target = resolveDeepLinkTarget(link);
      if (target) {
        onNavigate(target.screen, target.params);
      }
    });
    void deepLinkManager.initialize();
    return unsubscribe;
  }, [onNavigate]);

  // Keep the manager's auth gate in sync so auth-required links queued while
  // logged out are dispatched the moment the user authenticates.
  useEffect(() => {
    deepLinkManager.setAuthenticated(isAuthenticated);
  }, [isAuthenticated]);
}
