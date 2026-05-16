/**
 * MfaEnrollmentScreen (Phase 6 B6).
 *
 * Three-step enrollment flow for platform principals:
 *   1. Start — shows the QR code + manual secret; user scans with authenticator.
 *   2. Verify — user enters the first 6-digit TOTP code to confirm enrollment.
 *   3. Codes  — displays the 10 one-time recovery codes; user clicks "I've saved
 *               these" to dismiss.
 *
 * Design contract (mirrors MfaChallengeModal):
 *   * i18n-agnostic: host app passes a `labels` prop with pre-translated strings;
 *     all fields fall back to English defaults so callers without i18n still work.
 *   * Dependency-free w.r.t. fetch: the host injects `onStart` / `onVerify`
 *     callbacks, keeping this package free of any API-client coupling.
 *   * Inline styles only (no CSS imports).
 */

import {
  type FormEvent,
  type ReactElement,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';

// ── Label types ───────────────────────────────────────────────────────────────

export interface MfaEnrollmentLabels {
  /** Step 1 heading */
  stepScanTitle?: string;
  /** Step 1 QR-code instruction */
  stepScanDescription?: string;
  /** Step 1 "Manual entry" label above the secret */
  manualEntryLabel?: string;
  /** Step 1 "Next" / continue button */
  next?: string;

  /** Step 2 heading */
  stepVerifyTitle?: string;
  /** Step 2 instruction */
  stepVerifyDescription?: string;
  /** Step 2 code input label */
  codeLabel?: string;
  /** Step 2 submit button */
  verify?: string;
  /** Step 2 submit button while in-flight */
  verifying?: string;
  /** Step 2 invalid-format error */
  invalidFormat?: string;
  /** Step 2 wrong-code error */
  invalidCode?: string;
  /** Generic error fallback */
  verificationFailed?: string;

  /** Step 3 heading */
  stepCodesTitle?: string;
  /** Step 3 instruction — tell the user to save the codes */
  stepCodesDescription?: string;
  /** Step 3 confirm button */
  savedCodes?: string;
}

const defaultLabels: Required<MfaEnrollmentLabels> = {
  stepScanTitle: 'Set up two-factor authentication',
  stepScanDescription:
    'Scan the QR code below with your authenticator app (Google Authenticator, Authy, etc.).',
  manualEntryLabel: 'Or enter this code manually:',
  next: 'Next',

  stepVerifyTitle: 'Verify your authenticator',
  stepVerifyDescription: 'Enter the 6-digit code from your authenticator app to confirm setup.',
  codeLabel: 'Verification code',
  verify: 'Verify',
  verifying: 'Verifying…',
  invalidFormat: 'Enter the 6-digit code from your authenticator app.',
  invalidCode: 'That code did not match. Try again.',
  verificationFailed: 'Verification failed.',

  stepCodesTitle: 'Save your recovery codes',
  stepCodesDescription:
    'These 10 codes can each be used once if you lose access to your authenticator. Store them somewhere safe — they will not be shown again.',
  savedCodes: "I've saved these codes",
};

// ── Prop types ────────────────────────────────────────────────────────────────

export interface MfaEnrollmentScreenProps {
  /**
   * Async callback: calls `POST /admin/mfa/enroll/start`.
   * Returns `{ uri: string; secret: string }` on success, throws on failure.
   */
  onStart: () => Promise<{ uri: string; secret: string }>;

  /**
   * Async callback: calls `POST /admin/mfa/enroll/verify`.
   * Returns `{ recovery_codes: string[] }` on success; returns false or throws
   * when the code is rejected.
   */
  onVerify: (code: string) => Promise<{ recovery_codes: string[] } | false>;

  /**
   * Called after the user clicks "I've saved these codes", signalling that
   * enrollment is complete and the parent can route to the admin console.
   */
  onComplete: () => void;

  /** Optional pre-translated label overrides. */
  labels?: MfaEnrollmentLabels;
}

// ── Component ─────────────────────────────────────────────────────────────────

type Step = 'start' | 'loading' | 'scan' | 'verify' | 'codes';

export function MfaEnrollmentScreen({
  onStart,
  onVerify,
  onComplete,
  labels,
}: MfaEnrollmentScreenProps): ReactElement {
  const l = useMemo<Required<MfaEnrollmentLabels>>(
    () => ({ ...defaultLabels, ...(labels ?? {}) }),
    [labels]
  );

  const [step, setStep] = useState<Step>('start');
  const [enrollData, setEnrollData] = useState<{ uri: string; secret: string } | null>(null);
  const [recoveryCodes, setRecoveryCodes] = useState<string[]>([]);
  const [code, setCode] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const codeRef = useRef<HTMLInputElement | null>(null);

  // Kick off enrollment on mount.
  useEffect(() => {
    let cancelled = false;
    setStep('loading');
    onStart()
      .then((data) => {
        if (!cancelled) {
          setEnrollData(data);
          setStep('scan');
        }
      })
      .catch((e) => {
        if (!cancelled) {
          setError(e instanceof Error ? e.message : 'Failed to start enrollment.');
          setStep('scan'); // show error in scan step area
        }
      });
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Auto-focus the code input when we enter the verify step.
  useEffect(() => {
    if (step === 'verify') {
      const id = window.setTimeout(() => codeRef.current?.focus(), 0);
      return () => window.clearTimeout(id);
    }
    return undefined;
  }, [step]);

  const handleVerify = useCallback(
    async (event: FormEvent<HTMLFormElement>) => {
      event.preventDefault();
      if (!/^\d{6}$/.test(code.trim())) {
        setError(l.invalidFormat);
        return;
      }
      setBusy(true);
      setError(null);
      try {
        const result = await onVerify(code.trim());
        if (result === false) {
          setError(l.invalidCode);
        } else {
          setRecoveryCodes(result.recovery_codes);
          setStep('codes');
        }
      } catch (e) {
        setError(e instanceof Error ? e.message : l.verificationFailed);
      } finally {
        setBusy(false);
      }
    },
    [code, onVerify, l]
  );

  // ── Render ─────────────────────────────────────────────────────────────────

  return (
    <div style={containerStyle}>
      <div style={cardStyle}>
        {step === 'loading' && <p style={mutedText}>Setting up enrollment…</p>}

        {(step === 'scan' || step === 'start') && (
          <>
            <h2 style={headingStyle}>{l.stepScanTitle}</h2>
            <p style={mutedText}>{l.stepScanDescription}</p>
            {error && (
              <p role="alert" style={errorText}>
                {error}
              </p>
            )}
            {enrollData && (
              <>
                {/* QR code via Google Charts — no external runtime dependency */}
                <div style={{ textAlign: 'center', margin: '1rem 0' }}>
                  <img
                    src={`https://chart.googleapis.com/chart?chs=200x200&chld=M|0&cht=qr&chl=${encodeURIComponent(enrollData.uri)}`}
                    alt="TOTP QR code"
                    width={200}
                    height={200}
                    style={{ border: '1px solid #eee', borderRadius: 4 }}
                  />
                </div>
                <p style={{ fontSize: '0.8rem', color: '#777', marginBottom: '0.25rem' }}>
                  {l.manualEntryLabel}
                </p>
                <code style={secretStyle}>{enrollData.secret}</code>
                <div style={{ marginTop: '1.25rem', display: 'flex', justifyContent: 'flex-end' }}>
                  <button
                    type="button"
                    onClick={() => {
                      setError(null);
                      setCode('');
                      setStep('verify');
                    }}
                    style={primaryBtnStyle}
                  >
                    {l.next}
                  </button>
                </div>
              </>
            )}
          </>
        )}

        {step === 'verify' && (
          <>
            <h2 style={headingStyle}>{l.stepVerifyTitle}</h2>
            <p style={mutedText}>{l.stepVerifyDescription}</p>
            <form onSubmit={handleVerify} noValidate>
              <label htmlFor="ppt-mfa-enroll-code" style={labelStyle}>
                {l.codeLabel}
              </label>
              <input
                id="ppt-mfa-enroll-code"
                ref={codeRef}
                type="text"
                inputMode="numeric"
                autoComplete="one-time-code"
                pattern="[0-9]{6}"
                maxLength={6}
                value={code}
                onChange={(e) => setCode(e.target.value.replace(/\D/g, ''))}
                disabled={busy}
                aria-invalid={error ? 'true' : 'false'}
                style={codeInputStyle}
              />
              {error && (
                <p role="alert" style={errorText}>
                  {error}
                </p>
              )}
              <div style={actionsStyle}>
                <button
                  type="button"
                  onClick={() => setStep('scan')}
                  disabled={busy}
                  style={secondaryBtnStyle}
                >
                  Back
                </button>
                <button type="submit" disabled={busy} style={primaryBtnStyle}>
                  {busy ? l.verifying : l.verify}
                </button>
              </div>
            </form>
          </>
        )}

        {step === 'codes' && (
          <>
            <h2 style={headingStyle}>{l.stepCodesTitle}</h2>
            <p style={mutedText}>{l.stepCodesDescription}</p>
            <div style={codesGridStyle}>
              {recoveryCodes.map((c, i) => (
                // biome-ignore lint/suspicious/noArrayIndexKey: index is stable for a fixed list
                <code key={i} style={codeChipStyle}>
                  {c}
                </code>
              ))}
            </div>
            <div style={{ marginTop: '1.25rem', display: 'flex', justifyContent: 'flex-end' }}>
              <button type="button" onClick={onComplete} style={primaryBtnStyle}>
                {l.savedCodes}
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}

MfaEnrollmentScreen.displayName = 'MfaEnrollmentScreen';

// ── Styles ────────────────────────────────────────────────────────────────────

const containerStyle: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  minHeight: '100vh',
  background: '#f4f6f8',
  padding: '1rem',
};

const cardStyle: React.CSSProperties = {
  background: '#fff',
  borderRadius: '10px',
  padding: '2rem',
  width: 'min(480px, 96vw)',
  boxShadow: '0 4px 16px rgba(0,0,0,0.12)',
  fontFamily: 'system-ui, -apple-system, Segoe UI, Roboto, sans-serif',
};

const headingStyle: React.CSSProperties = {
  margin: '0 0 0.5rem',
  fontSize: '1.25rem',
  fontWeight: 600,
};

const mutedText: React.CSSProperties = {
  color: '#555',
  fontSize: '0.9rem',
  margin: '0 0 0.75rem',
};

const errorText: React.CSSProperties = {
  color: '#b00020',
  fontSize: '0.85rem',
  marginTop: '0.5rem',
};

const secretStyle: React.CSSProperties = {
  display: 'block',
  background: '#f5f5f5',
  padding: '0.5rem 0.75rem',
  borderRadius: '6px',
  fontSize: '0.95rem',
  letterSpacing: '0.1em',
  wordBreak: 'break-all',
};

const labelStyle: React.CSSProperties = {
  display: 'block',
  fontSize: '0.85rem',
  marginBottom: '0.25rem',
  color: '#333',
};

const codeInputStyle: React.CSSProperties = {
  width: '100%',
  padding: '0.5rem 0.75rem',
  fontSize: '1.5rem',
  letterSpacing: '0.35em',
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
  padding: '0.5rem 1.25rem',
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

const codesGridStyle: React.CSSProperties = {
  display: 'grid',
  gridTemplateColumns: '1fr 1fr',
  gap: '0.5rem',
  margin: '0.75rem 0',
};

const codeChipStyle: React.CSSProperties = {
  display: 'block',
  background: '#f0f4ff',
  border: '1px solid #c8d6f5',
  borderRadius: '5px',
  padding: '0.4rem 0.6rem',
  fontSize: '0.9rem',
  letterSpacing: '0.08em',
  fontFamily: 'monospace',
  textAlign: 'center',
};
