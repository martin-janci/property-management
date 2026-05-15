/**
 * Sticky banner shown whenever the current session is impersonating a tenant
 * user. Leak #21 calls out impersonation as one of the things that must be
 * impossible to *hide*: the banner sits at the top of the viewport, has high
 * contrast styling, and includes a one-click "End impersonation" affordance.
 *
 * The banner is purely presentational. The token state lives in whatever auth
 * provider the host app uses; we accept the resolved fields as props.
 */

import type React from 'react';

export interface ImpersonationBannerProps {
  /** True when an impersonation token is present in the session. */
  active: boolean;
  /** Display name / email of the user being impersonated. */
  targetUserLabel?: string;
  /** ISO timestamp at which the token expires. Renders a countdown if present. */
  expiresAt?: string;
  /** Click handler for the "End impersonation" button. */
  onEnd?: () => void;
}

export const ImpersonationBanner: React.FC<ImpersonationBannerProps> = ({
  active,
  targetUserLabel,
  expiresAt,
  onEnd,
}) => {
  if (!active) return null;
  return (
    <div
      role="alert"
      aria-live="assertive"
      className="ppt-admin-impersonation-banner"
      data-testid="impersonation-banner"
      style={bannerStyle}
    >
      <span>
        <strong>IMPERSONATING</strong>
        {targetUserLabel ? <> as <code>{targetUserLabel}</code></> : null}
        {expiresAt ? <> · expires {new Date(expiresAt).toLocaleTimeString()}</> : null}
      </span>
      {onEnd ? (
        <button
          type="button"
          onClick={onEnd}
          className="ppt-admin-impersonation-banner__end"
          style={endButtonStyle}
        >
          End impersonation
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
