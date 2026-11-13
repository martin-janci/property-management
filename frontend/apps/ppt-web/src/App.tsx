import { MfaChallengeProvider } from '@ppt/admin-ui';
import { setMfaChallengeHandler } from '@ppt/api-client';
import { AccessibilityProvider, SkipNavigation } from '@ppt/ui-kit';
import { useQueryClient } from '@tanstack/react-query';
import { type ReactNode, Suspense, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { BrowserRouter, useLocation } from 'react-router-dom';
import { AnnouncerProvider, OfflineIndicator, ToastProvider } from './components';
import { AppShell } from './components/AppShell/AppShell';
import { ErrorBoundary } from './components/ErrorBoundary';
import './styles/accessibility.css';
import './features/settings/styles/accessibility.css';
import { AuthProvider, OrganizationProvider, useAuth, WebSocketProvider } from './contexts';
import { verifyMfaCode } from './features/auth/mfaVerify';
import {
  CommandPaletteDialog,
  CommandPaletteProvider,
  useNavigationCommands,
} from './features/command-palette';
import { AppRoutes } from './routes/AppRoutes';

/**
 * MFA challenge wrapper (N9).
 *
 * Mounts `<MfaChallengeProvider>` so the api-client's mfa_required
 * interceptor has somewhere to send the challenge. The provider needs
 * two things from this layer that the package itself cannot reach:
 *
 *   * `setMfaChallengeHandler` — the api-client global registration
 *     point. Passed in so admin-ui has no @ppt/api-client dependency.
 *   * `verify(code)` — delegated to `verifyMfaCode`, which POSTs {code} to
 *     the MFA verify endpoint via the shared authenticated client so the
 *     request hits the configured API base (not the web origin) and carries
 *     the bearer token from the registered auth providers.
 *
 * The verify endpoint is `/api/v1/auth/mfa/verify` (api-server's
 * routes/mfa.rs). On 200 the user is freshly MFA-verified for the
 * RECENT_MFA_WINDOW (15 min); the api-client retries the original
 * mutation transparently.
 */
function MfaWrapper({ children }: { children: ReactNode }) {
  const { t } = useTranslation();
  // Resolve labels once per render so the modal sees stable strings even
  // when the active locale changes (the next render swaps them out).
  const mfaLabels = {
    title: t('admin.mfa.title'),
    description: t('admin.mfa.description'),
    descriptionForAction: t('admin.mfa.descriptionForAction', { action: '{action}' }),
    codeLabel: t('admin.mfa.label'),
    verify: t('admin.mfa.verify'),
    verifying: t('admin.mfa.verifying'),
    cancel: t('admin.mfa.cancel'),
    invalidFormat: t('admin.mfa.invalidFormat'),
    invalidCode: t('admin.mfa.invalidCode'),
    verificationFailed: t('admin.mfa.verificationFailed'),
  };
  return (
    <MfaChallengeProvider
      registerHandler={setMfaChallengeHandler}
      labels={mfaLabels}
      verify={verifyMfaCode}
    >
      {children}
    </MfaChallengeProvider>
  );
}

/**
 * WebSocket wrapper that bridges AuthContext to WebSocketProvider.
 * Must be rendered inside AuthProvider to access auth state.
 */
function WebSocketWrapper({ children }: { children: ReactNode }) {
  const { getAccessToken, isAuthenticated } = useAuth();
  const queryClient = useQueryClient();

  // Bridge realtime `domain.action` events to cache invalidation. The provider
  // resolves each event to its affected query roots (see `eventToQueryKeys` /
  // `categoryToQueryKeys`); here we invalidate them so the affected lists
  // refetch. Without this wiring the WebSocket delivers events that never touch
  // the cache — the realtime sync is dead end-to-end.
  const handleEntityEvent = useCallback(
    (_eventType: string, queryKeys: string[]) => {
      for (const key of queryKeys) {
        queryClient.invalidateQueries({ queryKey: [key] });
      }
    },
    [queryClient]
  );

  return (
    <WebSocketProvider
      auth={{
        accessToken: getAccessToken(),
        isAuthenticated,
      }}
      onEntityEvent={handleEntityEvent}
    >
      {children}
    </WebSocketProvider>
  );
}

function RouteLoading() {
  const { t } = useTranslation();
  return (
    <div
      className="flex items-center justify-center py-16"
      role="status"
      aria-live="polite"
      aria-busy="true"
    >
      <div
        className="h-8 w-8 animate-spin rounded-full border-2 border-blue-600 border-t-transparent"
        aria-hidden="true"
      />
      <span className="sr-only">{t('common.loading')}</span>
    </div>
  );
}

/**
 * Route-outlet-level error boundary.
 *
 * Scopes render/lazy failures to the route outlet so they can't unmount the
 * whole application shell. Without this, a single stale-chunk `lazy()`
 * rejection (or any uncaught render error in one route) propagates all the way
 * to the top-level `<ErrorBoundary>` in `main.tsx`, which wraps the *entire*
 * `<App />` — nav, providers and all — and replaces the whole UI with the
 * global fallback. Placing a boundary here, as a child of `<main>` and a
 * sibling *below* the `<AppShell>` header/sidebar, keeps the shell (nav, language switcher,
 * connection status) mounted and renders the fallback only inside the content
 * region. The reused `ErrorBoundary` fallback already offers a reload
 * affordance ("Reload Page") plus an in-place "Try Again".
 *
 * The boundary is keyed on `pathname` so that navigating to a different route
 * after a failure resets it automatically — the user recovers by clicking a
 * nav link, without a full page reload. Must be rendered inside
 * `BrowserRouter` for `useLocation` to resolve.
 */
export function RouteErrorBoundary({ children }: { children: ReactNode }) {
  const { pathname } = useLocation();
  return <ErrorBoundary key={pathname}>{children}</ErrorBoundary>;
}

/**
 * Command Palette wrapper that registers navigation commands.
 * Must be rendered inside BrowserRouter for useNavigate to work.
 */
function CommandPaletteWrapper({ children }: { children: ReactNode }) {
  useNavigationCommands();
  return (
    <>
      <CommandPaletteDialog />
      {children}
    </>
  );
}

function App() {
  return (
    <AccessibilityProvider>
      <AuthProvider>
        <OrganizationProvider>
          <ToastProvider>
            <AnnouncerProvider>
              <MfaWrapper>
                <WebSocketWrapper>
                  <BrowserRouter>
                    <CommandPaletteProvider>
                      <CommandPaletteWrapper>
                        <SkipNavigation mainContentId="main-content" />
                        <OfflineIndicator />
                        {/* Phase 5 impersonation banner moved with the
                            admin SPA to admin.rlt.sk. ppt-web tenants
                            never see impersonation tokens directly. */}
                        <AppShell>
                          <main id="main-content">
                            <RouteErrorBoundary>
                              <Suspense fallback={<RouteLoading />}>
                                <AppRoutes />
                              </Suspense>
                            </RouteErrorBoundary>
                          </main>
                        </AppShell>
                      </CommandPaletteWrapper>
                    </CommandPaletteProvider>
                  </BrowserRouter>
                </WebSocketWrapper>
              </MfaWrapper>
            </AnnouncerProvider>
          </ToastProvider>
        </OrganizationProvider>
      </AuthProvider>
    </AccessibilityProvider>
  );
}

export default App;
