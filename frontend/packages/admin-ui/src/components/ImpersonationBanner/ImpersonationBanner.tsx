/**
 * Sticky banner shown whenever the current session is impersonating a tenant
 * user. Leak #21 calls out impersonation as one of the things that must be
 * impossible to *hide*: the banner sits at the top of the viewport, has high
 * contrast styling, and includes a one-click "End impersonation" affordance.
 *
 * The banner is purely presentational. The token state lives in whatever auth
 * provider the host app uses; we accept the resolved fields as props.
 *
 * i18n (D3): mirrors `<MfaChallengeModal>` — the package has no i18n runtime.
 * The host passes a `labels` prop with already-translated strings; any field
 * left out falls back to the English default so callers without i18n still
 * get a working banner. Templates use the `{name}` / `{time}` placeholders.
 */

import type React from 'react';

export interface ImpersonationBannerLabels {
  /** Static "IMPERSONATING" prefix shown at the leading edge. */
  label?: string;
  /**
   * Template appended after the label when `targetUserLabel` is provided.
   * Receives the user label via the `{name}` placeholder.
   */
  asUser?: string;
  /**
   * Template appended after the user label when `expiresAt` is provided.
   * Receives the formatted local time via the `{time}` placeholder.
   */
  expires?: string;
  /** Text shown on the "End impersonation" button. */
  stop?: string;
}

const defaultLabels: Required<ImpersonationBannerLabels> = {
  label: 'IMPERSONATING',
  asUser: 'as {name}',
  expires: 'expires {time}',
  stop: 'End impersonation',
};

export interface ImpersonationBannerProps {
  /** True when an impersonation token is present in the session. */
  active: boolean;
  /** Display name / email of the user being impersonated. */
  targetUserLabel?: string;
  /** ISO timestamp at which the token expires. Renders a countdown if present. */
  expiresAt?: string;
  /** Click handler for the "End impersonation" button. */
  onEnd?: () => void;
  /**
   * Optional translated labels. The host app should pass these from its own
   * i18n layer (ppt-web uses react-i18next; see `admin.impersonation.*` keys).
   * Any field left out falls back to the English default.
   */
  labels?: ImpersonationBannerLabels;
}

export const ImpersonationBanner: React.FC<ImpersonationBannerProps> = ({
  active,
  targetUserLabel,
  expiresAt,
  onEnd,
  labels,
}) => {
  if (!active) return null;
  const l: Required<ImpersonationBannerLabels> = { ...defaultLabels, ...(labels ?? {}) };
  const asUserText = targetUserLabel
    ? l.asUser.replace('{name}', targetUserLabel)
    : null;
  const expiresText = expiresAt
    ? l.expires.replace('{time}', new Date(expiresAt).toLocaleTimeString())
    : null;
  return (
    <div
      role="alert"
      aria-live="assertive"
      className="ppt-admin-impersonation-banner"
      data-testid="impersonation-banner"
      style={bannerStyle}
    >
      <span>
        <strong>{l.label}</strong>
        {asUserText ? <> {asUserText}</> : null}
        {expiresText ? <> · {expiresText}</> : null}
      </span>
      {onEnd ? (
        <button
          type="button"
          onClick={onEnd}
          className="ppt-admin-impersonation-banner__end"
          style={endButtonStyle}
        >
          {l.stop}
        </button>
      ) : null}
    </div>
  );
};

const bannerStyle: React.CSSProperties = {
  position: 'sticky',
  top: 0,
  zIndex: 9999,
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'space-between',
  gap: '1rem',
  padding: '0.5rem 1rem',
  background: '#b91c1c',
  color: '#ffffff',
  fontWeight: 600,
};

const endButtonStyle: React.CSSProperties = {
  background: '#ffffff',
  color: '#b91c1c',
  border: 'none',
  padding: '0.25rem 0.75rem',
  borderRadius: 4,
  cursor: 'pointer',
  fontWeight: 700,
};
