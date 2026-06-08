/**
 * OAuthConsentPage — `/oauth/authorize`
 *
 * The OAuth 2.0 authorization-code consent screen (Epic 10A). A third-party
 * client redirects the user here with the standard authorize query params
 * (`response_type`, `client_id`, `redirect_uri`, `scope`, `state`,
 * `code_challenge`, `code_challenge_method`). We validate the request against
 * the backend, show the requesting app + the scopes it wants, and let the user
 * Approve or Deny.
 *
 * Wired to (reusing the shared `@ppt/api-client` OAuth consent client/types):
 *   GET  /api/v1/oauth/authorize — validate request, fetch consent page data
 *   POST /api/v1/oauth/authorize — submit the approve/deny decision
 *
 * On Approve the backend returns an authorization `code` + echoed `state`,
 * which we hand back to the client by redirecting to its `redirect_uri`.
 * On Deny (per RFC 6749 §4.1.2.1) we redirect back with `error=access_denied`.
 *
 * Standalone route — rendered outside the AdminLayout chrome. It requires an
 * authenticated session (the consent endpoints need a Bearer token) but not a
 * specific capability: it is a per-user authorization decision, not a
 * platform-admin action.
 */

import { type AuthorizeRequestParams, useConsentPageData, useSubmitConsent } from '@ppt/api-client';
import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { Navigate, useLocation, useSearchParams } from 'react-router-dom';

import { useAdminAuth } from '../auth/AdminAuthContext';

// ============================================================
// Inline styles (injected once) — matches the OAuthClientsPage idiom
// ============================================================

const STYLE_ID = 'ppt-oauth-consent-styles';

function ensureStyles() {
  if (typeof document === 'undefined') return;
  if (document.getElementById(STYLE_ID)) return;
  const el = document.createElement('style');
  el.id = STYLE_ID;
  el.textContent = `
    .ppt-ocs-wrap {
      min-height: 100vh; display: flex; align-items: center; justify-content: center;
      padding: 24px; background: var(--ppt-bg-app,#f9fafb); box-sizing: border-box;
    }
    .ppt-ocs-card {
      max-width: 440px; width: 100%;
      background: var(--ppt-bg-surface,#fff);
      border: 1px solid var(--ppt-border-default,#e5e7eb);
      border-radius: 14px; padding: 32px 30px 26px;
      box-shadow: 0 8px 32px rgba(0,0,0,0.08);
      display: flex; flex-direction: column; gap: 18px;
    }
    .ppt-ocs-app { display: flex; flex-direction: column; gap: 4px; text-align: center; }
    .ppt-ocs-app-name { font-size: 19px; font-weight: 700; color: var(--ppt-fg-primary,#111827); margin: 0; }
    .ppt-ocs-app-desc { font-size: 13px; color: var(--ppt-fg-secondary,#6b7280); margin: 0; }
    .ppt-ocs-lead { font-size: 14px; color: var(--ppt-fg-secondary,#374151); margin: 0; text-align: center; }
    .ppt-ocs-scopes { display: flex; flex-direction: column; gap: 10px; margin: 0; padding: 0; list-style: none; }
    .ppt-ocs-scope {
      display: flex; gap: 10px; align-items: flex-start;
      border: 1px solid var(--ppt-border-default,#e5e7eb); border-radius: 9px;
      padding: 11px 13px; background: var(--ppt-bg-subtle,#f9fafb);
    }
    .ppt-ocs-scope-check { color: var(--ppt-brand-600,#2563eb); font-weight: 700; line-height: 1.3; flex-shrink: 0; }
    .ppt-ocs-scope-body { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
    .ppt-ocs-scope-name { font-family: var(--ppt-font-mono,monospace); font-size: 12px; font-weight: 600; color: var(--ppt-fg-primary,#111827); }
    .ppt-ocs-scope-desc { font-size: 12px; color: var(--ppt-fg-secondary,#6b7280); }
    .ppt-ocs-redirect { font-size: 11px; color: var(--ppt-fg-muted,#9ca3af); text-align: center; word-break: break-all; margin: 0; }
    .ppt-ocs-actions { display: flex; gap: 10px; margin-top: 4px; }
    .ppt-ocs-btn {
      flex: 1; display: inline-flex; align-items: center; justify-content: center; gap: 7px;
      padding: 10px 14px; border-radius: 8px; font-size: 14px; font-weight: 600;
      cursor: pointer; border: 1px solid transparent; transition: background 120ms ease;
    }
    .ppt-ocs-btn:disabled { opacity: 0.5; cursor: not-allowed; }
    .ppt-ocs-btn--approve { background: var(--ppt-brand-500,#3b82f6); color: #fff; border-color: var(--ppt-brand-500,#3b82f6); }
    .ppt-ocs-btn--approve:hover:not(:disabled) { background: var(--ppt-brand-600,#2563eb); }
    .ppt-ocs-btn--deny { background: transparent; color: var(--ppt-fg-secondary,#374151); border-color: var(--ppt-border-default,#e5e7eb); }
    .ppt-ocs-btn--deny:hover:not(:disabled) { background: var(--ppt-bg-hover,#f3f4f6); }
    .ppt-ocs-status { text-align: center; font-size: 14px; color: var(--ppt-fg-secondary,#6b7280); padding: 12px 0; }
    .ppt-ocs-error {
      font-size: 13px; color: var(--ppt-danger-700,#b91c1c);
      background: #fee2e2; border: 1px solid #fecaca; border-radius: 8px; padding: 11px 13px;
    }
    .ppt-ocs-spinner {
      width: 15px; height: 15px; border: 2px solid rgba(255,255,255,0.4);
      border-top-color: #fff; border-radius: 50%;
      animation: ppt-ocs-spin 0.7s linear infinite; display: inline-block;
    }
    @keyframes ppt-ocs-spin { to { transform: rotate(360deg); } }
  `;
  document.head.appendChild(el);
}

// ============================================================
// Helpers
// ============================================================

/**
 * Append query params to a URI string, preserving any the URI already carries.
 * Falls back to plain string concatenation if the URI is not absolute (the
 * backend has already validated `redirect_uri` against the client, so this is
 * defensive rather than a security boundary).
 */
function appendParams(uri: string, params: Record<string, string>): string {
  try {
    const url = new URL(uri);
    for (const [k, v] of Object.entries(params)) url.searchParams.set(k, v);
    return url.toString();
  } catch {
    const qs = new URLSearchParams(params).toString();
    return uri.includes('?') ? `${uri}&${qs}` : `${uri}?${qs}`;
  }
}

// ============================================================
// OAuthConsentPage
// ============================================================

export default function OAuthConsentPage() {
  ensureStyles();
  const { t } = useTranslation();
  const { isAuthenticated } = useAdminAuth();
  const location = useLocation();
  const [searchParams] = useSearchParams();

  // Parse the inbound authorize request from the query string. Memoised so the
  // query-key and the enabled-gate in useConsentPageData stay stable across
  // renders (a fresh object every render would thrash TanStack Query).
  const params = useMemo<AuthorizeRequestParams>(
    () => ({
      responseType: searchParams.get('response_type') ?? '',
      clientId: searchParams.get('client_id') ?? '',
      redirectUri: searchParams.get('redirect_uri') ?? '',
      scope: searchParams.get('scope') ?? undefined,
      state: searchParams.get('state') ?? undefined,
      codeChallenge: searchParams.get('code_challenge') ?? undefined,
      codeChallengeMethod: searchParams.get('code_challenge_method') ?? undefined,
    }),
    [searchParams]
  );

  const hasRequiredParams =
    params.responseType.length > 0 && params.clientId.length > 0 && params.redirectUri.length > 0;

  const consentQuery = useConsentPageData(hasRequiredParams ? params : null);
  const submitMutation = useSubmitConsent(params);

  // Must be signed in for the consent endpoints (they require a Bearer token).
  // Preserve the full authorize URL so the user lands back here post-login.
  if (!isAuthenticated) {
    return (
      <Navigate to="/login" state={{ from: `${location.pathname}${location.search}` }} replace />
    );
  }

  async function handleDecision(decision: 'approve' | 'deny') {
    if (decision === 'deny') {
      // RFC 6749 §4.1.2.1 — bounce back to the client with an error code.
      const errParams: Record<string, string> = { error: 'access_denied' };
      if (params.state) errParams.state = params.state;
      window.location.assign(appendParams(params.redirectUri, errParams));
      return;
    }
    try {
      const res = await submitMutation.mutateAsync('approve');
      const okParams: Record<string, string> = { code: res.code };
      if (res.state) okParams.state = res.state;
      window.location.assign(appendParams(params.redirectUri, okParams));
    } catch {
      // Error surfaced inline below via submitMutation.error.
    }
  }

  let body: React.ReactNode;

  if (!hasRequiredParams) {
    body = (
      <div className="ppt-ocs-error" role="alert">
        {t('admin.oauthConsent.invalidRequest')}
      </div>
    );
  } else if (consentQuery.isLoading) {
    body = <div className="ppt-ocs-status">{t('admin.oauthConsent.loading')}</div>;
  } else if (consentQuery.isError) {
    body = (
      <div className="ppt-ocs-error" role="alert">
        {t('admin.oauthConsent.loadError', {
          message:
            consentQuery.error instanceof Error
              ? consentQuery.error.message
              : String(consentQuery.error),
        })}
      </div>
    );
  } else if (consentQuery.data) {
    const data = consentQuery.data;
    const submitting = submitMutation.isPending;
    body = (
      <>
        <div className="ppt-ocs-app">
          <p className="ppt-ocs-app-name">{data.clientName}</p>
          {data.clientDescription && <p className="ppt-ocs-app-desc">{data.clientDescription}</p>}
        </div>
        <p className="ppt-ocs-lead">{t('admin.oauthConsent.lead', { app: data.clientName })}</p>
        {data.scopes.length > 0 && (
          <ul className="ppt-ocs-scopes">
            {data.scopes.map((scope) => (
              <li key={scope.name} className="ppt-ocs-scope">
                <span className="ppt-ocs-scope-check" aria-hidden="true">
                  &#x2713;
                </span>
                <span className="ppt-ocs-scope-body">
                  <span className="ppt-ocs-scope-name">{scope.name}</span>
                  <span className="ppt-ocs-scope-desc">{scope.description}</span>
                </span>
              </li>
            ))}
          </ul>
        )}
        <p className="ppt-ocs-redirect">
          {t('admin.oauthConsent.redirectNotice', { uri: data.redirectUri })}
        </p>
        {submitMutation.isError && (
          <div className="ppt-ocs-error" role="alert">
            {t('admin.oauthConsent.submitError', {
              message:
                submitMutation.error instanceof Error
                  ? submitMutation.error.message
                  : String(submitMutation.error),
            })}
          </div>
        )}
        <div className="ppt-ocs-actions">
          <button
            type="button"
            className="ppt-ocs-btn ppt-ocs-btn--deny"
            onClick={() => void handleDecision('deny')}
            disabled={submitting}
          >
            {t('admin.oauthConsent.deny')}
          </button>
          <button
            type="button"
            className="ppt-ocs-btn ppt-ocs-btn--approve"
            onClick={() => void handleDecision('approve')}
            disabled={submitting}
          >
            {submitting && <span className="ppt-ocs-spinner" aria-hidden="true" />}
            {t('admin.oauthConsent.approve')}
          </button>
        </div>
      </>
    );
  }

  return (
    <div className="ppt-ocs-wrap">
      <main className="ppt-ocs-card" aria-labelledby="ppt-ocs-title">
        <h1 id="ppt-ocs-title" className="ppt-ocs-app-name" style={{ fontSize: 16 }}>
          {t('admin.oauthConsent.title')}
        </h1>
        {body}
      </main>
    </div>
  );
}
