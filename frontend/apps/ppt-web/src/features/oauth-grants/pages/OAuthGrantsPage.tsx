/**
 * OAuth Grants Management Page (Epic 10A, Story 10A-3).
 *
 * Allows the authenticated user to view and revoke third-party app
 * authorizations. Calls:
 *   GET    /api/v1/oauth/grants             — via useUserGrants()
 *   DELETE /api/v1/oauth/grants/{clientId}  — via useRevokeUserGrant()
 */

import { type UserGrant, useRevokeUserGrant, useUserGrants } from '@ppt/api-client';
import type React from 'react';
import { useState } from 'react';
import { useToast } from '../../../components';
import './OAuthGrantsPage.css';
import { Trans, useTranslation } from 'react-i18next';

/** Internal state for the revoke confirmation dialog. */
interface ConfirmState {
  clientId: string;
  clientName: string;
}

export const OAuthGrantsPage: React.FC = () => {
  const { t, i18n } = useTranslation();
  const { showToast } = useToast();
  const { data: grants, isLoading, isError, error, refetch } = useUserGrants();
  const revokeGrant = useRevokeUserGrant();
  const [confirmState, setConfirmState] = useState<ConfirmState | null>(null);

  const handleRevokeClick = (clientId: string, clientName: string) => {
    setConfirmState({ clientId, clientName });
  };

  const handleConfirmRevoke = async () => {
    if (!confirmState) return;
    const { clientId, clientName } = confirmState;
    setConfirmState(null);
    try {
      await revokeGrant.mutateAsync(clientId);
      showToast({
        type: 'success',
        title: t('oauthGrants.revoked'),
        message: t('oauthGrants.revokedMessage', { name: clientName }),
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: t('oauthGrants.revokeFailed'),
        message: err instanceof Error ? err.message : t('oauthGrants.unexpected'),
      });
    }
  };

  const handleCancelRevoke = () => setConfirmState(null);

  if (isLoading) {
    return (
      <div className="oauth-grants-container">
        <h1>{t('oauthGrants.title')}</h1>
        <div className="oauth-grants-loading" role="status" aria-live="polite" aria-busy="true">
          {t('oauthGrants.loading')}
        </div>
      </div>
    );
  }

  if (isError) {
    return (
      <div className="oauth-grants-container">
        <h1>{t('oauthGrants.title')}</h1>
        <div className="oauth-grants-error" role="alert">
          <p>{error instanceof Error ? error.message : t('oauthGrants.loadFailed')}</p>
          <button type="button" onClick={() => refetch()}>
            {t('oauthGrants.retry')}
          </button>
        </div>
      </div>
    );
  }

  const grantList: UserGrant[] = grants ?? [];

  return (
    <div className="oauth-grants-container">
      <h1>{t('oauthGrants.title')}</h1>
      <p>{t('oauthGrants.description')}</p>

      {grantList.length === 0 ? (
        <div className="oauth-grants-empty" role="status">
          {t('oauthGrants.empty')}
        </div>
      ) : (
        <ul className="oauth-grants-list" aria-label={t('aria.authorizedApplications')}>
          {grantList.map((grant: UserGrant) => (
            <li key={grant.id} className="oauth-grant-card">
              <div className="oauth-grant-info">
                <h2 className="oauth-grant-name">{grant.clientName}</h2>
                {grant.clientDescription && (
                  <p className="oauth-grant-description">{grant.clientDescription}</p>
                )}
                <div className="oauth-grant-meta">
                  <span className="oauth-grant-date">
                    <strong>{t('oauthGrants.authorized')}</strong>{' '}
                    {new Date(grant.grantedAt).toLocaleDateString(i18n.language, {
                      year: 'numeric',
                      month: 'long',
                      day: 'numeric',
                    })}
                  </span>
                  {grant.scopes.length > 0 && (
                    <div className="oauth-grant-scopes">
                      <strong>{t('oauthGrants.permissions')}</strong>
                      <span
                        className="oauth-grant-scope-list"
                        aria-label={t('oauthGrants.scopesLabel', {
                          scopes: grant.scopes.join(', '),
                        })}
                      >
                        {grant.scopes.map((scope: string) => (
                          <span key={scope} className="oauth-grant-scope-badge">
                            {scope}
                          </span>
                        ))}
                      </span>
                    </div>
                  )}
                </div>
              </div>
              <button
                type="button"
                className="oauth-grant-revoke-btn"
                onClick={() => handleRevokeClick(grant.clientId, grant.clientName)}
                disabled={revokeGrant.isPending}
                aria-label={t('oauthGrants.revokeFor', { name: grant.clientName })}
              >
                {t('oauthGrants.revoke')}
              </button>
            </li>
          ))}
        </ul>
      )}

      {confirmState && (
        <div
          className="oauth-grants-confirm-overlay"
          role="dialog"
          aria-modal="true"
          aria-labelledby="revoke-dialog-title"
          aria-describedby="revoke-dialog-desc"
        >
          <div className="oauth-grants-confirm-dialog">
            <h2 id="revoke-dialog-title">{t('oauthGrants.confirmTitle')}</h2>
            <p id="revoke-dialog-desc">
              <Trans
                i18nKey="oauthGrants.confirmMessage"
                values={{ name: confirmState.clientName }}
                components={{ strong: <strong /> }}
              />
            </p>
            <div className="oauth-grants-confirm-actions">
              <button type="button" className="cancel-btn" onClick={handleCancelRevoke}>
                {t('oauthGrants.cancel')}
              </button>
              <button
                type="button"
                className="confirm-revoke-btn"
                onClick={handleConfirmRevoke}
                disabled={revokeGrant.isPending}
                aria-busy={revokeGrant.isPending}
              >
                {revokeGrant.isPending ? t('oauthGrants.revoking') : t('oauthGrants.revoke')}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

OAuthGrantsPage.displayName = 'OAuthGrantsPage';
