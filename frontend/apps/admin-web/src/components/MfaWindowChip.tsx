/**
 * MfaWindowChip — presentational header chip showing MFA verification window state.
 *
 * Three states:
 *   - unverified: gray pill, "MFA · not verified"
 *   - verified (>2min):  green pill with pulsing dot, "MFA verified · MM:SS"
 *   - expiring (≤2min): amber pill with pulsing dot, "MFA expires · M:SS"
 *
 * Context: MfaWindowProvider reads useAdminAuth().token and decodes the JWT for
 * the MFA-verified claim (`mfa_verified_at` + `mfa_window_seconds`).
 *
 * TODO(backend): api-server must include `mfa_verified_at` and `mfa_window_seconds`
 * in the access-token claims for this chip to show verified/expiring states.
 * Until that claim is present the provider stubs { state: 'unverified', msRemaining: 0 }.
 */

import {
  createContext,
  type ReactNode,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';

import { useAdminAuth } from '../auth/AdminAuthContext';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type MfaWindowState = 'unverified' | 'verified' | 'expiring';

export interface MfaWindowChipProps {
  state: MfaWindowState;
  msRemaining?: number;
  onClick?: () => void;
}

interface MfaWindowContextValue {
  state: MfaWindowState;
  msRemaining: number;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Decode a JWT payload without verifying the signature.
 * Only used client-side for display purposes — never for auth decisions.
 */
function decodeJwtPayload(token: string): Record<string, unknown> | null {
  try {
    const parts = token.split('.');
    if (parts.length !== 3) return null;
    const padded = parts[1].replace(/-/g, '+').replace(/_/g, '/');
    const json = atob(padded.padEnd(padded.length + ((4 - (padded.length % 4)) % 4), '='));
    return JSON.parse(json) as Record<string, unknown>;
  } catch {
    return null;
  }
}

const TWO_MINUTES_MS = 2 * 60 * 1000;

function deriveWindowState(token: string | null): MfaWindowContextValue {
  if (!token) return { state: 'unverified', msRemaining: 0 };

  const payload = decodeJwtPayload(token);
  if (!payload) return { state: 'unverified', msRemaining: 0 };

  // TODO(backend): depend on `mfa_verified_at` (unix seconds) and
  // `mfa_window_seconds` claims emitted by api-server's JWT minting.
  const mfaVerifiedAt = payload.mfa_verified_at;
  const mfaWindowSec = payload.mfa_window_seconds;

  if (typeof mfaVerifiedAt !== 'number' || typeof mfaWindowSec !== 'number') {
    return { state: 'unverified', msRemaining: 0 };
  }

  const expiresAt = (mfaVerifiedAt + mfaWindowSec) * 1000;
  const msRemaining = Math.max(0, expiresAt - Date.now());

  if (msRemaining === 0) return { state: 'unverified', msRemaining: 0 };
  if (msRemaining <= TWO_MINUTES_MS) return { state: 'expiring', msRemaining };
  return { state: 'verified', msRemaining };
}

// ---------------------------------------------------------------------------
// Context + Provider
// ---------------------------------------------------------------------------

const MfaWindowContext = createContext<MfaWindowContextValue | null>(null);

export function MfaWindowProvider({ children }: { children: ReactNode }) {
  const { token } = useAdminAuth();
  const [value, setValue] = useState<MfaWindowContextValue>(() => deriveWindowState(token));
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const tick = useCallback(() => {
    setValue(deriveWindowState(token));
  }, [token]);

  // Recompute every second so the countdown stays live.
  useEffect(() => {
    setValue(deriveWindowState(token));
    if (timerRef.current) clearInterval(timerRef.current);
    timerRef.current = setInterval(tick, 1000);
    return () => {
      if (timerRef.current) clearInterval(timerRef.current);
    };
  }, [token, tick]);

  return <MfaWindowContext.Provider value={value}>{children}</MfaWindowContext.Provider>;
}

MfaWindowProvider.displayName = 'MfaWindowProvider';

/** Returns current MFA window state derived from the active token. */
export function useMfaWindow(): MfaWindowContextValue {
  const ctx = useContext(MfaWindowContext);
  if (!ctx) throw new Error('useMfaWindow must be used inside <MfaWindowProvider>');
  return ctx;
}

// ---------------------------------------------------------------------------
// Format helpers
// ---------------------------------------------------------------------------

/** Format ms → "MM:SS" */
function fmtMmSs(ms: number): string {
  const totalSec = Math.max(0, Math.ceil(ms / 1000));
  const m = Math.floor(totalSec / 60);
  const s = totalSec % 60;
  return `${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
}

/** Format ms → "M:SS" (no leading zero on minutes) */
function fmtMSs(ms: number): string {
  const totalSec = Math.max(0, Math.ceil(ms / 1000));
  const m = Math.floor(totalSec / 60);
  const s = totalSec % 60;
  return `${m}:${String(s).padStart(2, '0')}`;
}

// ---------------------------------------------------------------------------
// Styles (injected once as a <style> tag)
// ---------------------------------------------------------------------------

const STYLE_ID = 'ppt-mfa-window-chip-styles';

function ensureStyles() {
  if (typeof document === 'undefined') return;
  if (document.getElementById(STYLE_ID)) return;
  const el = document.createElement('style');
  el.id = STYLE_ID;
  el.textContent = `
    @keyframes ppt-mfa-pulse {
      0%, 100% { opacity: 1; }
      50%       { opacity: 0.4; }
    }
    .ppt-mfa-chip {
      display: inline-flex;
      align-items: center;
      gap: 6px;
      padding: 4px 10px;
      border-radius: 999px;
      font-size: 12px;
      font-weight: 500;
      font-variant-numeric: tabular-nums;
      cursor: default;
      border: none;
      background: none;
      user-select: none;
      transition: filter 120ms ease;
      line-height: 1.4;
    }
    .ppt-mfa-chip--clickable {
      cursor: pointer;
    }
    .ppt-mfa-chip--clickable:hover {
      filter: brightness(0.96);
    }
    .ppt-mfa-chip--clickable:focus-visible {
      outline: 2px solid var(--ppt-brand-500, #3b82f6);
      outline-offset: 2px;
    }
    .ppt-mfa-chip--unverified {
      background: var(--ppt-neutral-100, #f3f4f6);
      color: var(--ppt-neutral-700, #374151);
    }
    .ppt-mfa-chip--verified {
      background: var(--ppt-success-100, #d1fae5);
      color: var(--ppt-success-800, #065f46);
    }
    .ppt-mfa-chip--expiring {
      background: var(--ppt-warning-100, #fef3c7);
      color: var(--ppt-warning-800, #92400e);
    }
    .ppt-mfa-chip__dot {
      width: 6px;
      height: 6px;
      border-radius: 50%;
      flex-shrink: 0;
      animation: ppt-mfa-pulse 1.5s ease-in-out infinite;
    }
    .ppt-mfa-chip--verified .ppt-mfa-chip__dot {
      background: var(--ppt-success-500, #10b981);
    }
    .ppt-mfa-chip--expiring .ppt-mfa-chip__dot {
      background: var(--ppt-warning-500, #f59e0b);
    }
  `;
  document.head.appendChild(el);
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function MfaWindowChip({ state, msRemaining = 0, onClick }: MfaWindowChipProps) {
  ensureStyles();

  const isClickable = Boolean(onClick);

  const label = useMemo<string>(() => {
    switch (state) {
      case 'unverified':
        return 'MFA · not verified';
      case 'verified':
        return `MFA verified · ${fmtMmSs(msRemaining)}`;
      case 'expiring':
        return `MFA expires · ${fmtMSs(msRemaining)}`;
    }
  }, [state, msRemaining]);

  const ariaLabel = useMemo<string>(() => {
    switch (state) {
      case 'unverified':
        return 'MFA not verified. Click to verify.';
      case 'verified':
        return `MFA verified. Window expires in ${fmtMmSs(msRemaining)}.`;
      case 'expiring':
        return `MFA window expiring soon. ${fmtMSs(msRemaining)} remaining.`;
    }
  }, [state, msRemaining]);

  const className = [
    'ppt-mfa-chip',
    `ppt-mfa-chip--${state}`,
    isClickable ? 'ppt-mfa-chip--clickable' : '',
  ]
    .filter(Boolean)
    .join(' ');

  const showDot = state === 'verified' || state === 'expiring';

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (!onClick) return;
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        onClick();
      }
    },
    [onClick]
  );

  return (
    <button
      type="button"
      className={className}
      aria-label={ariaLabel}
      onClick={onClick}
      onKeyDown={handleKeyDown}
      tabIndex={isClickable ? 0 : -1}
    >
      {showDot && <span className="ppt-mfa-chip__dot" aria-hidden="true" />}
      <span>{label}</span>
    </button>
  );
}

MfaWindowChip.displayName = 'MfaWindowChip';

// ---------------------------------------------------------------------------
// Re-export a connected version that reads from context (convenience)
// ---------------------------------------------------------------------------

export function ConnectedMfaWindowChip({ onClick }: { onClick?: () => void }) {
  const { state, msRemaining } = useMfaWindow();
  return <MfaWindowChip state={state} msRemaining={msRemaining} onClick={onClick} />;
}

ConnectedMfaWindowChip.displayName = 'ConnectedMfaWindowChip';
