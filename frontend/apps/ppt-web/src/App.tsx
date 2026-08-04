import { MfaChallengeProvider } from '@ppt/admin-ui';
import { setMfaChallengeHandler } from '@ppt/api-client';
import { AccessibilityProvider, SkipNavigation } from '@ppt/ui-kit';
import { type ReactNode, Suspense } from 'react';
import { useTranslation } from 'react-i18next';
import { BrowserRouter, Link, useLocation, useNavigate } from 'react-router-dom';
import {
  AnnouncerProvider,
  ConnectionStatus,
  LanguageSwitcher,
  OfflineIndicator,
  ToastProvider,
} from './components';
import { ErrorBoundary } from './components/ErrorBoundary';
import './styles/accessibility.css';
import './features/settings/styles/accessibility.css';
import { AuthProvider, OrganizationProvider, useAuth, WebSocketProvider } from './contexts';
import {
  CommandPaletteDialog,
  CommandPaletteProvider,
  useNavigationCommands,
} from './features/command-palette';
import { AppRoutes } from './routes/AppRoutes';
import { isManagerRole } from './routes/shared';

/**
 * MFA challenge wrapper (N9).
 *
 * Mounts `<MfaChallengeProvider>` so the api-client's mfa_required
 * interceptor has somewhere to send the challenge. The provider needs
 * two things from this layer that the package itself cannot reach:
 *
 *   * `setMfaChallengeHandler` — the api-client global registration
 *     point. Passed in so admin-ui has no @ppt/api-client dependency.
 *   * `verify(code)` — POST {code} to /api/v1/auth/mfa/verify with the
 *     current bearer token. Built from `AuthContext.getAccessToken`.
 *
 * The verify endpoint is `/api/v1/auth/mfa/verify` (api-server's
 * routes/mfa.rs). On 200 the user is freshly MFA-verified for the
 * RECENT_MFA_WINDOW (15 min); the api-client retries the original
 * mutation transparently.
 *
 * Must be rendered inside AuthProvider so getAccessToken is available.
 */
function MfaWrapper({ children }: { children: ReactNode }) {
  const { getAccessToken } = useAuth();
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
      verify={async (code: string) => {
        const token = getAccessToken();
        const headers: Record<string, string> = { 'Content-Type': 'application/json' };
        if (token) headers.Authorization = `Bearer ${token}`;
        try {
          const response = await fetch('/api/v1/auth/mfa/verify', {
            method: 'POST',
            headers,
            body: JSON.stringify({ code }),
          });
          return response.ok;
        } catch {
          return false;
        }
      }}
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
  return (
    <WebSocketProvider
      auth={{
        accessToken: getAccessToken(),
        isAuthenticated,
      }}
    >
      {children}
    </WebSocketProvider>
  );
}

function AppNavigation() {
  const { t } = useTranslation();
  const { isAuthenticated, logout, user } = useAuth();
  const navigate = useNavigate();
  const isOperator = isManagerRole(user?.role);

  const handleLogout = async () => {
    await logout();
    navigate('/login', { replace: true });
  };

  return (
    <nav className="app-nav" aria-label="Main navigation">
      <Link to="/">{t('nav.home')}</Link>
      <Link to="/buildings">{t('nav.buildings')}</Link>
      <Link to="/documents">{t('nav.documents')}</Link>
      <Link to="/ai-assistant">{t('nav.aiChat')}</Link>
      <Link to="/news">{t('nav.news')}</Link>
      <Link to="/emergency">{t('nav.emergency')}</Link>
      <Link to="/disputes">{t('nav.disputes')}</Link>
      <Link to="/leases">{t('nav.leases', { defaultValue: 'Leases' })}</Link>
      <Link to="/outages">{t('nav.outages')}</Link>
      <Link to="/iot">{t('nav.iot', { defaultValue: 'Smart Building' })}</Link>
      <Link to="/iot/alerts">{t('nav.iotAlerts', { defaultValue: 'Sensor Alerts' })}</Link>
      <Link to="/rentals">{t('nav.rentals', { defaultValue: 'Rentals' })}</Link>
      {isOperator && (
        <Link to="/notifications/analytics">
          {t('nav.notificationAnalytics', { defaultValue: 'Notification Analytics' })}
        </Link>
      )}
      <Link to="/settings/integrations">
        {t('nav.integrations', { defaultValue: 'Integrations' })}
      </Link>
      {isOperator && (
        <Link to="/settings/portal-webhooks">
          {t('nav.portalWebhooks', { defaultValue: 'Portal Webhooks' })}
        </Link>
      )}
      <Link to="/settings/accessibility">{t('nav.accessibility')}</Link>
      <Link to="/settings/privacy">{t('nav.privacy')}</Link>
      <Link to="/settings/sessions">{t('nav.sessions', { defaultValue: 'Sessions' })}</Link>
      {isAuthenticated && (
        <Link to="/notifications/triggers">
          {t('nav.notificationTriggers', { defaultValue: 'Notification Triggers' })}
        </Link>
      )}
      {/* Super-admin moved to admin.rlt.sk (separate SPA) — no nav link here. */}
      <div className="ml-auto flex items-center gap-3">
        {isAuthenticated && <ConnectionStatus />}
        <LanguageSwitcher />
        {isAuthenticated && (
          <button
            type="button"
            className="btn-outline-token px-3 py-1.5 rounded-lg text-sm font-medium"
            onClick={handleLogout}
            aria-label={t('auth.signOut', { defaultValue: 'Sign out' })}
          >
            {t('auth.signOut', { defaultValue: 'Sign out' })}
          </button>
        )}
        {!isAuthenticated && (
          <Link
            to="/login"
            className="btn-primary-token px-3 py-1.5 rounded-lg text-sm font-medium"
          >
            {t('auth.signIn')}
          </Link>
        )}
      </div>
    </nav>
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
 * sibling *below* `<AppNavigation>`, keeps the shell (nav, language switcher,
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
                        <div className="app">
                          {/* Phase 5 impersonation banner moved with the
                              admin SPA to admin.rlt.sk. ppt-web tenants
                              never see impersonation tokens directly. */}
                          <AppNavigation />
                          <main id="main-content">
                            <RouteErrorBoundary>
                              <Suspense fallback={<RouteLoading />}>
                                <AppRoutes />
                              </Suspense>
                            </RouteErrorBoundary>
                          </main>
                        </div>
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
