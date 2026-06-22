import { useEffect, useRef } from 'react';
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
  // Hold the latest `onNavigate` in a ref so the wiring effect can read the
  // current callback without listing it as a dependency. This keeps the effect
  // stable: the handler is registered and `initialize()` runs exactly once for
  // the hook's lifetime, even when the parent passes a fresh `onNavigate`
  // identity on re-render (avoids duplicate handlers / re-initialisation).
  const onNavigateRef = useRef(onNavigate);
  onNavigateRef.current = onNavigate;

  useEffect(() => {
    const unsubscribe = deepLinkManager.addHandler((link) => {
      const target = resolveDeepLinkTarget(link);
      if (target) {
        onNavigateRef.current(target.screen, target.params);
      }
    });
    // `initialize()` rejecting must not surface as an unhandled promise
    // rejection; deep-link cold-start failures are non-fatal for the app.
    deepLinkManager.initialize().catch((error) => {
      console.warn('[useDeepLinkRouting] deepLinkManager.initialize() failed', error);
    });
    return unsubscribe;
  }, []);

  // Keep the manager's auth gate in sync so auth-required links queued while
  // logged out are dispatched the moment the user authenticates.
  useEffect(() => {
    deepLinkManager.setAuthenticated(isAuthenticated);
  }, [isAuthenticated]);
}
