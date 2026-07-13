/**
 * Core route group.
 *
 * Auth, dashboard, settings, emergency, AI assistant, workflow automation, and
 * error/state surfaces — plus the `Home` landing page. These routes have no
 * per-feature data wrappers (pages self-resolve params/auth), so they are
 * grouped together. Extracted from App.tsx to keep the aggregator slim.
 */
import { lazy } from 'react';
import { useTranslation } from 'react-i18next';
import { Link, Navigate, Route, useNavigate } from 'react-router-dom';
import { ProtectedRoute } from '../../components';
import { useAuth } from '../../contexts';
import { ManagerDashboardPage, ResidentDashboardPage } from '../../features/dashboard';
import {
  AccessibilitySettingsPage,
  AdvancedNotificationSettingsPage,
  AiChatPage,
  AuthCallbackPage,
  AutomationRulesPage,
  ChangePasswordPage,
  CreateRulePage,
  EditRulePage,
  EmergencyContactDirectoryPage,
  ExecutionMonitoringPage,
  ForbiddenPage,
  ForgotPasswordPage,
  LoginPage,
  NotFoundPage,
  NotificationSettingsPage,
  OAuthGrantsPage,
  PrivacySettingsPage,
  ProfileEditPage,
  RegisterPage,
  ResetPasswordPage,
  ServerErrorPage,
  SessionExpiredPage,
  TemplateLibraryPage,
  TwoFactorAuthPage,
} from '../lazyRoutes';

// Sessions management page (#966) — lazy-loaded.
const SessionsPage = lazy(() =>
  import('../../features/settings/pages/SessionsPage').then((m) => ({ default: m.SessionsPage }))
);

// Integrations settings page (Gap 83-1 — Airbnb OAuth & Sync) — lazy-loaded.
const IntegrationsPage = lazy(() =>
  import('../../features/settings').then((m) => ({ default: m.IntegrationsPage }))
);

// Portal Webhooks status page (Gap 83-3 — Real Estate Portal Webhooks) — lazy-loaded.
const PortalWebhooksPage = lazy(() =>
  import('../../features/settings').then((m) => ({ default: m.PortalWebhooksPage }))
);

export function Home() {
  const { t } = useTranslation();
  const { isAuthenticated, user } = useAuth();
  const navigate = useNavigate();

  return (
    <div className="space-y-10">
      <section className="text-center py-10">
        <h1 className="text-3xl md:text-4xl font-bold mb-3">{t('home.title')}</h1>
        <p className="text-lg max-w-2xl mx-auto" style={{ color: 'var(--color-text-secondary)' }}>
          {t('home.welcome')}
        </p>
        {!isAuthenticated && (
          <div className="mt-6 flex flex-wrap justify-center gap-3">
            <button
              type="button"
              onClick={() => navigate('/login')}
              className="btn-primary-token px-5 py-2.5 rounded-lg font-medium"
            >
              {t('auth.signIn')}
            </button>
            <button
              type="button"
              onClick={() => navigate('/register')}
              className="btn-outline-token px-5 py-2.5 rounded-lg font-medium"
            >
              {t('auth.register', { defaultValue: 'Register' })}
            </button>
          </div>
        )}
        {isAuthenticated && (
          <div className="mt-6">
            <button
              type="button"
              onClick={() =>
                navigate(user?.role === 'manager' ? '/dashboard/manager' : '/dashboard/resident')
              }
              className="btn-primary-token px-5 py-2.5 rounded-lg font-medium"
            >
              {t('nav.dashboard', { defaultValue: 'Open dashboard' })}
            </button>
          </div>
        )}
      </section>

      <section className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
        {[
          { to: '/documents', titleKey: 'nav.documents' },
          { to: '/news', titleKey: 'nav.news' },
          { to: '/disputes', titleKey: 'nav.disputes' },
          { to: '/outages', titleKey: 'nav.outages' },
          { to: '/emergency', titleKey: 'nav.emergency' },
          { to: '/settings/accessibility', titleKey: 'nav.accessibility' },
        ].map((card) => (
          <Link
            key={card.to}
            to={card.to}
            className="block p-5 rounded-xl"
            style={{
              backgroundColor: 'var(--color-surface)',
              border: '1px solid var(--color-border)',
            }}
          >
            <span className="font-semibold" style={{ color: 'var(--color-text)' }}>
              {t(card.titleKey)}
            </span>
          </Link>
        ))}
      </section>
    </div>
  );
}

/** Auth + profile routes (UC-14). */
export function authRoutes() {
  return (
    <>
      <Route path="/login" element={<LoginPage />} />
      {/* SSO / OAuth callback route (gap-79-2).
          Handles provider redirect: reads ?code=…&state=…,
          exchanges for PPT JWT tokens via tokenProvider,
          then redirects to /dashboard or stored return URL. */}
      <Route path="/auth/callback" element={<AuthCallbackPage />} />
      <Route path="/register" element={<RegisterPage />} />
      <Route path="/forgot-password" element={<ForgotPasswordPage />} />
      <Route path="/reset-password" element={<ResetPasswordPage />} />
      <Route path="/settings/password" element={<ChangePasswordPage />} />
      <Route path="/settings/two-factor" element={<TwoFactorAuthPage />} />
      <Route path="/settings/profile" element={<ProfileEditPage />} />
    </>
  );
}

/** Dashboard routes (Epic 124). */
export function dashboardRoutes() {
  return (
    <>
      {/* Bare /dashboard 404'd previously — redirect it to the manager
          dashboard so users typing the obvious URL get something useful (the
          ProtectedRoute / role check on the target page handles auth +
          role-based fan-out). */}
      <Route path="/dashboard" element={<Navigate to="/dashboard/manager" replace />} />
      <Route path="/dashboard/manager" element={<ManagerDashboardPage />} />
      <Route path="/dashboard/resident" element={<ResidentDashboardPage />} />
    </>
  );
}

/** Settings, emergency, AI, automation, and error/state surface routes. */
export function settingsRoutes() {
  return (
    <>
      {/* Emergency contacts route (Epic 62) */}
      <Route
        path="/emergency"
        element={
          <ProtectedRoute>
            <EmergencyContactDirectoryPage />
          </ProtectedRoute>
        }
      />
      {/* Accessibility settings route (Epic 60) */}
      <Route path="/settings/accessibility" element={<AccessibilitySettingsPage />} />
      {/* Privacy settings route (Epic 63) */}
      <Route path="/settings/privacy" element={<PrivacySettingsPage />} />
      {/* OAuth Grants management route (Epic 10A, Story 10A-3) */}
      <Route
        path="/settings/oauth-grants"
        element={
          <ProtectedRoute>
            <OAuthGrantsPage />
          </ProtectedRoute>
        }
      />
      {/* Active sessions management (#966) */}
      <Route
        path="/settings/sessions"
        element={
          <ProtectedRoute>
            <SessionsPage />
          </ProtectedRoute>
        }
      />
      {/* External integrations — Airbnb OAuth & Sync (Gap 83-1) */}
      <Route
        path="/settings/integrations"
        element={
          <ProtectedRoute>
            <IntegrationsPage />
          </ProtectedRoute>
        }
      />
      {/* Portal webhook delivery status — Real Estate Portal Webhooks (Gap 83-3) */}
      <Route
        path="/settings/portal-webhooks"
        element={
          <ProtectedRoute>
            <PortalWebhooksPage />
          </ProtectedRoute>
        }
      />
      {/* Notification preferences (Epic 8A, Story 8A.1) */}
      <Route
        path="/settings/notifications"
        element={
          <ProtectedRoute>
            <NotificationSettingsPage />
          </ProtectedRoute>
        }
      />
      {/* Advanced notification preferences (Epic 40) */}
      <Route
        path="/settings/notifications/advanced"
        element={
          <ProtectedRoute>
            <AdvancedNotificationSettingsPage />
          </ProtectedRoute>
        }
      />
      {/* AI Assistant chat (Epic 13, Story 13.1) — gap-sweep */}
      <Route
        path="/ai-assistant"
        element={
          <ProtectedRoute>
            <AiChatPage />
          </ProtectedRoute>
        }
      />
      {/* Workflow Automation (Epic 13) — gap-sweep.
        Paths match the pages' hard-coded navigate() targets. */}
      <Route
        path="/automations/rules"
        element={
          <ProtectedRoute>
            <AutomationRulesPage />
          </ProtectedRoute>
        }
      />
      <Route
        path="/automations/rules/new"
        element={
          <ProtectedRoute>
            <CreateRulePage />
          </ProtectedRoute>
        }
      />
      <Route
        path="/automations/rules/:id/edit"
        element={
          <ProtectedRoute>
            <EditRulePage />
          </ProtectedRoute>
        }
      />
      <Route
        path="/automations/templates"
        element={
          <ProtectedRoute>
            <TemplateLibraryPage />
          </ProtectedRoute>
        }
      />
      <Route
        path="/automations/executions"
        element={
          <ProtectedRoute>
            <ExecutionMonitoringPage />
          </ProtectedRoute>
        }
      />
    </>
  );
}

/** Error / state surface routes. */
export function errorRoutes() {
  return (
    <>
      <Route path="/forbidden" element={<ForbiddenPage />} />
      <Route path="/server-error" element={<ServerErrorPage />} />
      <Route path="/session-expired" element={<SessionExpiredPage />} />
      <Route path="*" element={<NotFoundPage />} />
    </>
  );
}
