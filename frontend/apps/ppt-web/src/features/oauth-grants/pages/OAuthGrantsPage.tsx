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
import { useTranslation } from 'react-i18next';

/** Internal state for the revoke confirmation dialog. */
interface ConfirmState {
  clientId: string;
  clientName: string;
}

export const OAuthGrantsPage: React.FC = () => {
  const { t } = useTranslation();
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
        title: 'Access revoked',
        message: `${clientName} can no longer access your account.`,
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: 'Failed to revoke access',
        message: err instanceof Error ? err.message : 'An unexpected error occurred.',
      });
    }
  };

  const handleCancelRevoke = () => setConfirmState(null);

  if (isLoading) {
    return (
      <div className="oauth-grants-container">
        <h1>Authorized Applications</h1>
        <div className="oauth-grants-loading" role="status" aria-live="polite" aria-busy="true">
          Loading authorized applications&hellip;
        </div>
      </div>
    );
  }

  if (isError) {
    return (
      <div className="oauth-grants-container">
        <h1>Authorized Applications</h1>
        <div className="oauth-grants-error" role="alert">
          <p>
            {error instanceof Error
              ? error.message
              : 'Failed to load authorized applications. Please try again.'}
          </p>
          <button type="button" onClick={() => refetch()}>
            Retry
          </button>
        </div>
      </div>
    );
  }

  const grantList: UserGrant[] = grants ?? [];

  return (
    <div className="oauth-grants-container">
      <h1>Authorized Applications</h1>
      <p>
        These third-party applications have been granted access to your account. You can revoke
        access at any time &mdash; the application will need to request authorization again to
        regain access.
      </p>

      {grantList.length === 0 ? (
        <div className="oauth-grants-empty" role="status">
          You have no authorized third-party applications.
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
                    <strong>Authorized:</strong>{' '}
                    {new Date(grant.grantedAt).toLocaleDateString(undefined, {
                      year: 'numeric',
                      month: 'long',
                      day: 'numeric',
                    })}
                  </span>
                  {grant.scopes.length > 0 && (
                    <div className="oauth-grant-scopes">
                      <strong>Permissions:</strong>
                      <span
                        className="oauth-grant-scope-list"
                        aria-label={`Scopes: ${grant.scopes.join(', ')}`}
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
                aria-label={`Revoke access for ${grant.clientName}`}
              >
                Revoke access
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
            <h2 id="revoke-dialog-title">Revoke access?</h2>
            <p id="revoke-dialog-desc">
              Revoking access for <strong>{confirmState.clientName}</strong> will invalidate all
              active tokens. The application will no longer be able to access your account until you
              grant access again.
            </p>
            <div className="oauth-grants-confirm-actions">
              <button type="button" className="cancel-btn" onClick={handleCancelRevoke}>
                Cancel
              </button>
              <button
                type="button"
                className="confirm-revoke-btn"
                onClick={handleConfirmRevoke}
                disabled={revokeGrant.isPending}
                aria-busy={revokeGrant.isPending}
              >
                {revokeGrant.isPending ? 'Revoking…' : 'Revoke access'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

OAuthGrantsPage.displayName = 'OAuthGrantsPage';
