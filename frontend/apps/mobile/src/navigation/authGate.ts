/**
 * Navigation auth guard (AC-5, 82-2 Navigation and Routing).
 *
 * The Property Management mobile app does not use a URL router; navigation is
 * a top-level branch in `App.tsx`'s `MainApp`. The auth guard is therefore the
 * decision that picks which of three mutually-exclusive views the app renders:
 *
 *   - `loading`   — auth state is still being restored from SecureStore.
 *   - `auth`      — the user is NOT authenticated, so every protected route is
 *                   replaced by the login flow (`AuthFlow`). This is the
 *                   "redirect unauthenticated users to login" behaviour.
 *   - `protected` — the user is authenticated; protected screens may render.
 *
 * Keeping this as a pure function makes the guard contract independently
 * testable (no RN tree mount required) and gives `MainApp` a single source of
 * truth for the redirect decision.
 */

/** The subset of auth state the navigation guard depends on. */
export interface AuthGateState {
  /** True while the provider is still restoring tokens from SecureStore. */
  isLoading: boolean;
  /** True once a valid session has been established. */
  isAuthenticated: boolean;
}

/**
 * The view the navigation layer must render for a given auth state.
 *
 * `auth` is the guarded outcome: when it is returned, NO protected screen is
 * reachable — the user is sent to the login flow instead.
 */
export type AuthGateDecision = 'loading' | 'auth' | 'protected';

/**
 * Resolve which top-level view the app should render.
 *
 * Order matters: a still-loading session must show the splash/loading view and
 * must NOT leak protected content while we don't yet know whether the user is
 * authenticated. Only an explicitly authenticated session unlocks protected
 * routes; everything else falls through to the login flow.
 */
export function resolveAuthGate(state: AuthGateState): AuthGateDecision {
  if (state.isLoading) {
    return 'loading';
  }
  if (!state.isAuthenticated) {
    return 'auth';
  }
  return 'protected';
}

/**
 * Convenience predicate: may a protected route render for this state?
 *
 * Equivalent to `resolveAuthGate(state) === 'protected'`; exported so call
 * sites that only need a boolean gate read clearly.
 */
export function canAccessProtectedRoute(state: AuthGateState): boolean {
  return resolveAuthGate(state) === 'protected';
}
