/**
 * MFA challenge modal (N9).
 *
 * Renders a TOTP code input + Verify button. Used by the app shell's
 * `<MfaChallengeProvider>` to satisfy the api-client's `mfa_required`
 * interceptor: when an admin action returns 401 mfa_required, this modal
 * opens, the user enters a 6-digit code, we POST it to the existing
 * `/api/v1/auth/mfa/verify` endpoint, and on 200 the original action is
 * retried.
 *
 * The component is dependency-free w.r.t. fetch (the verify call is
 * passed in as a prop). That keeps `@ppt/admin-ui` free of any
 * `@ppt/api-client` coupling and makes the modal trivially testable
 * with a stubbed `onVerify`.
 *
 * i18n (D3): the package has no i18n runtime. Instead the host app
 * passes a `labels` prop with already-translated strings. All fields
 * are optional and fall back to English defaults so callers without
 * i18n still get a working modal.
 */

import { useCallback, useEffect, useMemo, useRef, useState, type FormEvent } from 'react';

export interface MfaChallengeLabels {
  /** Modal title. */
  title?: string;
  /** Description shown when no `actionLabel` is provided. */
  description?: string;
  /**
   * Description template used when `actionLabel` is provided.
   * Receives the action label via the `{action}` placeholder.
   */
  descriptionForAction?: string;
  /** Visible label for the verification code input. */
  codeLabel?: string;
  /** Submit button text. */
  verify?: string;
  /** Submit button text while a verification is in flight. */
  verifying?: string;
  /** Cancel button text. */
  cancel?: string;
  /** Error shown when the entered string is not a 6-digit code. */
  invalidFormat?: string;
  /** Error shown when the verifier returns false. */
  invalidCode?: string;
  /** Generic error shown when the verifier throws without a message. */
  verificationFailed?: string;
}

const defaultLabels: Required<MfaChallengeLabels> = {
  title: 'Confirm with two-factor code',
  description: 'This action requires a fresh authentication code.',
  descriptionForAction: '{action} requires a fresh authentication code.',
  codeLabel: 'Verification code',
  verify: 'Verify',
  verifying: 'Verifying…',
  cancel: 'Cancel',
  invalidFormat: 'Enter the 6-digit code from your authenticator app.',
  invalidCode: 'That code did not work. Try again.',
  verificationFailed: 'Verification failed.',
};

export interface MfaChallengeModalProps {
  /** Controls visibility. Driven by the provider on mfa_required. */
  open: boolean;
  /**
   * Async verifier — receives the 6-digit code, returns true on success.
   * The provider injects a closure that POSTs to /api/v1/auth/mfa/verify.
   */
  onVerify: (code: string) => Promise<boolean>;
  /** Called after a successful verification. Provider resolves the gate. */
  onSuccess: () => void;
  /** Called on Cancel / Escape. Provider rejects the gate. */
  onCancel: () => void;
  /** Optional friendly label (e.g. "Suspend agency") for context. */
  actionLabel?: string;
  /**
   * Optional translated labels. The host app should pass these from its
   * own i18n layer (ppt-web uses react-i18next; see `admin.mfa.*` keys).
   * Any field left out falls back to the English default.
   */
  labels?: MfaChallengeLabels;
}

export function MfaChallengeModal({
  open,
  onVerify,
  onSuccess,
  onCancel,
  actionLabel,
  labels,
}: MfaChallengeModalProps): JSX.Element | null {
  const [code, setCode] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const inputRef = useRef<HTMLInputElement | null>(null);

  // Merge once per render — the spread is cheap and the result is used
  // in three places below.
  const l = useMemo<Required<MfaChallengeLabels>>(
    () => ({ ...defaultLabels, ...(labels ?? {}) }),
    [labels],
  );

  // Reset every time we open — leaving stale codes around is a footgun
  // (user re-opens the modal a minute later and the old code is invalid).
  useEffect(() => {
    if (open) {
      setCode('');
      setError(null);
      setBusy(false);
      // Defer focus so the element exists when we grab it.
      const id = window.setTimeout(() => inputRef.current?.focus(), 0);
      return () => window.clearTimeout(id);
    }
    return undefined;
  }, [open]);

  // Escape closes the modal — matches the platform-wide modal contract.
  useEffect(() => {
    if (!open) return undefined;
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && !busy) {
        onCancel();
      }
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [open, busy, onCancel]);

  const handleSubmit = useCallback(
    async (event: FormEvent<HTMLFormElement>) => {
      event.preventDefault();
      if (!/^\d{6}$/.test(code.trim())) {
        setError(l.invalidFormat);
        return;
      }
      setBusy(true);
      setError(null);
      try {
        const ok = await onVerify(code.trim());
        if (ok) {
          onSuccess();
        } else {
          setError(l.invalidCode);
        }
      } catch (e) {
        setError(e instanceof Error ? e.message : l.verificationFailed);
      } finally {
        setBusy(false);
      }
    },
    [code, onVerify, onSuccess, l],
  );

  if (!open) return null;

  // Description: substitute `{action}` placeholder if the host provided
  // a translated template; otherwise fall back to the no-action variant.
  const description = actionLabel
    ? l.descriptionForAction.replace('{action}', actionLabel)
    : l.description;

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="ppt-mfa-modal-title"
      style={overlayStyle}
    >
      <div style={dialogStyle}>
        <h2 id="ppt-mfa-modal-title" style={{ margin: '0 0 0.5rem', fontSize: '1.125rem' }}>
          {l.title}
        </h2>
        <p style={{ margin: '0 0 1rem', color: '#555', fontSize: '0.9rem' }}>{description}</p>
        <form onSubmit={handleSubmit} noValidate>
          <label
            htmlFor="ppt-mfa-modal-code"
            style={{ display: 'block', fontSize: '0.85rem', marginBottom: '0.25rem' }}
          >
            {l.codeLabel}
          </label>
          <input
            id="ppt-mfa-modal-code"
            ref={inputRef}
            type="text"
            inputMode="numeric"
            autoComplete="one-time-code"
            pattern="[0-9]{6}"
            maxLength={6}
            value={code}
            onChange={(e) => setCode(e.target.value.replace(/\D/g, ''))}
            disabled={busy}
            aria-invalid={error ? 'true' : 'false'}
            style={inputStyle}
          />
          {error && (
            <p role="alert" style={{ color: '#b00020', fontSize: '0.85rem', marginTop: '0.5rem' }}>
              {error}
            </p>
          )}
          <div style={actionsStyle}>
            <button
              type="button"
              onClick={onCancel}
              disabled={busy}
              style={secondaryBtnStyle}
            >
              {l.cancel}
            </button>
            <button type="submit" disabled={busy} style={primaryBtnStyle}>
              {busy ? l.verifying : l.verify}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

MfaChallengeModal.displayName = 'MfaChallengeModal';

// Inline styles only — `@ppt/admin-ui` is consumed by both ppt-web (Vite)
// and (future) other apps; pulling in a CSS file would force every host
// to wire up the loader.
const overlayStyle: React.CSSProperties = {
  position: 'fixed',
  inset: 0,
  background: 'rgba(0,0,0,0.45)',
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  zIndex: 9999,
};

const dialogStyle: React.CSSProperties = {
  background: '#fff',
  borderRadius: '8px',
  padding: '1.5rem',
  width: 'min(420px, 92vw)',
  boxShadow: '0 12px 32px rgba(0,0,0,0.25)',
  fontFamily: 'system-ui, -apple-system, Segoe UI, Roboto, sans-serif',
};

const inputStyle: React.CSSProperties = {
  width: '100%',
  padding: '0.5rem 0.75rem',
  fontSize: '1.25rem',
  letterSpacing: '0.25em',
  textAlign: 'center',
  border: '1px solid #ccc',
  borderRadius: '6px',
  boxSizing: 'border-box',
};

const actionsStyle: React.CSSProperties = {
  display: 'flex',
  justifyContent: 'flex-end',
  gap: '0.5rem',
  marginTop: '1rem',
};

const primaryBtnStyle: React.CSSProperties = {
  padding: '0.5rem 1rem',
  background: '#0050b3',
  color: '#fff',
  border: 'none',
  borderRadius: '6px',
  cursor: 'pointer',
  fontSize: '0.95rem',
};

const secondaryBtnStyle: React.CSSProperties = {
  padding: '0.5rem 1rem',
  background: '#fff',
  color: '#333',
  border: '1px solid #ccc',
  borderRadius: '6px',
  cursor: 'pointer',
  fontSize: '0.95rem',
};
