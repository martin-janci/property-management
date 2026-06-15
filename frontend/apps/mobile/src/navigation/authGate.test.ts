/**
 * Navigation auth-guard tests (AC-5, 82-2 Navigation and Routing).
 *
 * Evidence that the app guards protected routes: an unauthenticated session is
 * always sent to the login flow (`auth`) and can never reach a protected
 * screen, while still-loading sessions never leak protected content. This is
 * the redirect-unauthenticated-users-to-login behaviour required by AC-5.
 *
 * NOTE: this suite intentionally does NOT import
 * `@testing-library/react-native`. The repo has a `react-test-renderer`
 * 19.2.0-vs-19.2.6 peer-dep mismatch that fails every suite importing that
 * package (see `useDeepLinkRouting.test.tsx`). The guard decision is a pure
 * function shared with `App.tsx`'s `MainApp`, so we exercise the real contract
 * directly without mounting the RN tree.
 */

import { type AuthGateState, canAccessProtectedRoute, resolveAuthGate } from './authGate';

describe('navigation auth guard (AC-5)', () => {
  describe('resolveAuthGate', () => {
    it('shows the loading view while the session is being restored', () => {
      // isAuthenticated must be ignored while loading — we do not yet know it.
      expect(resolveAuthGate({ isLoading: true, isAuthenticated: false })).toBe('loading');
      expect(resolveAuthGate({ isLoading: true, isAuthenticated: true })).toBe('loading');
    });

    it('redirects an unauthenticated user to the login flow', () => {
      // The guard: no protected route renders; the user lands on `auth`.
      expect(resolveAuthGate({ isLoading: false, isAuthenticated: false })).toBe('auth');
    });

    it('unlocks protected routes only for an authenticated session', () => {
      expect(resolveAuthGate({ isLoading: false, isAuthenticated: true })).toBe('protected');
    });

    it('never returns `protected` for any unauthenticated state', () => {
      const unauthenticatedStates: AuthGateState[] = [
        { isLoading: true, isAuthenticated: false },
        { isLoading: false, isAuthenticated: false },
        { isLoading: true, isAuthenticated: true }, // loading wins until settled
      ];
      for (const state of unauthenticatedStates) {
        expect(resolveAuthGate(state)).not.toBe('protected');
      }
    });
  });

  describe('canAccessProtectedRoute', () => {
    it('blocks protected routes when unauthenticated', () => {
      expect(canAccessProtectedRoute({ isLoading: false, isAuthenticated: false })).toBe(false);
    });

    it('blocks protected routes while loading even if a stale flag is set', () => {
      expect(canAccessProtectedRoute({ isLoading: true, isAuthenticated: true })).toBe(false);
    });

    it('allows protected routes once authenticated and settled', () => {
      expect(canAccessProtectedRoute({ isLoading: false, isAuthenticated: true })).toBe(true);
    });
  });
});
