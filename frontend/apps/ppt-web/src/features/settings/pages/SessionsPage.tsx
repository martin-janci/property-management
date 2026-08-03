/**
 * Active Sessions Page (Story 1.5 / UC-14).
 *
 * Lists the authenticated user's active sessions (device/browser, IP,
 * created/last-active timestamps) and lets them revoke an individual session
 * or all sessions other than the current one.
 *
 * Backed by the auth API (`GET /auth/sessions`, `POST /auth/sessions/revoke`,
 * `POST /auth/sessions/revoke-all`) exposed through `getAuthApi()`.
 */

import type { SessionInfo } from '@ppt/api-client';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import type React from 'react';
import { useTranslation } from 'react-i18next';
import { getAuthApi } from '../../auth/authApiClient';
import '../styles/sessions.css';

/** Query key for the active-sessions list. */
const SESSIONS_QUERY_KEY = ['auth', 'sessions'] as const;

/** Derive a friendly device/browser label from a user-agent string. */
function deviceLabel(session: SessionInfo, unknownLabel: string): string {
  if (session.deviceInfo) return session.deviceInfo;
  const ua = session.userAgent;
  if (!ua) return unknownLabel;

  let browser = unknownLabel;
  if (/edg/i.test(ua)) browser = 'Edge';
  else if (/opr|opera/i.test(ua)) browser = 'Opera';
  else if (/chrome|crios/i.test(ua)) browser = 'Chrome';
  else if (/firefox|fxios/i.test(ua)) browser = 'Firefox';
  else if (/safari/i.test(ua)) browser = 'Safari';

  let os = '';
  if (/windows/i.test(ua)) os = 'Windows';
  else if (/iphone|ipad|ios/i.test(ua)) os = 'iOS';
  else if (/mac os x|macintosh/i.test(ua)) os = 'macOS';
  else if (/android/i.test(ua)) os = 'Android';
  else if (/linux/i.test(ua)) os = 'Linux';

  return os ? `${browser} · ${os}` : browser;
}

function formatTimestamp(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString();
}

export const SessionsPage: React.FC = () => {
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  const sessionsQuery = useQuery({
    queryKey: SESSIONS_QUERY_KEY,
    queryFn: () => getAuthApi().listSessions(),
  });

  const revokeSession = useMutation({
    mutationFn: (sessionId: string) => getAuthApi().revokeSession(sessionId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: SESSIONS_QUERY_KEY });
    },
  });

  const revokeAllOthers = useMutation({
    mutationFn: () => getAuthApi().revokeAllOtherSessions(),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: SESSIONS_QUERY_KEY });
    },
  });

  const sessions = sessionsQuery.data?.sessions ?? [];
  const hasOtherSessions = sessions.some((s) => !s.isCurrent);

  return (
    <div className="sessions-page">
      <header className="sessions-header">
        <h1>{t('settings.sessions.title')}</h1>
        <p>{t('settings.sessions.subtitle')}</p>
      </header>

      {sessionsQuery.isLoading && (
        <p className="sessions-status">{t('settings.sessions.loading')}</p>
      )}

      {sessionsQuery.isError && (
        <div className="sessions-error" role="alert">
          <span>{t('settings.sessions.loadError')}</span>
          <button type="button" className="sessions-btn" onClick={() => sessionsQuery.refetch()}>
            {t('settings.sessions.retry')}
          </button>
        </div>
      )}

      {revokeSession.isError && (
        <div className="sessions-error" role="alert">
          {t('settings.sessions.revokeError')}
        </div>
      )}
      {revokeAllOthers.isError && (
        <div className="sessions-error" role="alert">
          {t('settings.sessions.revokeAllError')}
        </div>
      )}

      {!sessionsQuery.isLoading && !sessionsQuery.isError && sessions.length === 0 && (
        <p className="sessions-status">{t('settings.sessions.empty')}</p>
      )}

      {sessions.length > 0 && (
        <>
          <div className="sessions-toolbar">
            <button
              type="button"
              className="sessions-btn sessions-btn--danger"
              disabled={!hasOtherSessions || revokeAllOthers.isPending}
              onClick={() => revokeAllOthers.mutate()}
            >
              {revokeAllOthers.isPending
                ? t('settings.sessions.revoking')
                : t('settings.sessions.revokeAll')}
            </button>
          </div>

          <ul className="sessions-list">
            {sessions.map((session) => (
              <li
                key={session.id}
                className={`session-card${session.isCurrent ? ' session-card--current' : ''}`}
              >
                <div className="session-info">
                  <div className="session-device">
                    <span className="session-device-name">
                      {deviceLabel(session, t('settings.sessions.unknownDevice'))}
                    </span>
                    {session.isCurrent && (
                      <span className="session-current-badge">
                        {t('settings.sessions.currentBadge')}
                      </span>
                    )}
                  </div>
                  <dl className="session-meta">
                    <div className="session-meta-row">
                      <dt>{t('settings.sessions.ipAddress')}</dt>
                      <dd>{session.ipAddress || t('settings.sessions.unknownIp')}</dd>
                    </div>
                    <div className="session-meta-row">
                      <dt>{t('settings.sessions.lastActive')}</dt>
                      <dd>{formatTimestamp(session.lastUsedAt)}</dd>
                    </div>
                    <div className="session-meta-row">
                      <dt>{t('settings.sessions.created')}</dt>
                      <dd>{formatTimestamp(session.createdAt)}</dd>
                    </div>
                  </dl>
                </div>

                <div className="session-actions">
                  <button
                    type="button"
                    className="sessions-btn sessions-btn--danger"
                    disabled={
                      session.isCurrent ||
                      (revokeSession.isPending && revokeSession.variables === session.id)
                    }
                    onClick={() => revokeSession.mutate(session.id)}
                  >
                    {session.isCurrent
                      ? t('settings.sessions.currentSession')
                      : t('settings.sessions.revoke')}
                  </button>
                </div>
              </li>
            ))}
          </ul>
        </>
      )}
    </div>
  );
};

SessionsPage.displayName = 'SessionsPage';

export default SessionsPage;
