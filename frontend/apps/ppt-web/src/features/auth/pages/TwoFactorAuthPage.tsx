/**
 * Two-Factor Authentication Setup Page (UC-14.10, Epic 9, Story 9.1).
 *
 * Wires to the backend MFA endpoints via @ppt/api-client useMfa* hooks:
 *   POST /api/v1/auth/mfa/setup              — initiate TOTP setup
 *   POST /api/v1/auth/mfa/verify             — confirm TOTP code, enables 2FA
 *   POST /api/v1/auth/mfa/disable            — disable 2FA (requires code)
 *   GET  /api/v1/auth/mfa/status             — current enabled/disabled state
 *   POST /api/v1/auth/mfa/backup-codes/regenerate — refresh backup codes
 */

import {
  useMfaDisable,
  useMfaRegenerateBackupCodes,
  useMfaSetup,
  useMfaStatus,
  useMfaVerify,
} from '@ppt/api-client';
import QRCode from 'qrcode';
import type React from 'react';
import { useCallback, useEffect, useRef, useState } from 'react';
import { Navigate } from 'react-router-dom';
import { useToast } from '../../../components/Toast';
import { useAuth } from '../../../contexts/AuthContext';
import '../styles/AuthPage.css';

// ─── QR code canvas renderer ─────────────────────────────────────────────────

function QrCodeCanvas({ uri }: { uri: string }) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    if (canvasRef.current) {
      QRCode.toCanvas(canvasRef.current, uri, { width: 200, margin: 2 }).catch(() => {});
    }
  }, [uri]);

  return (
    <canvas
      ref={canvasRef}
      aria-label="Scan this QR code with your authenticator app"
      style={{ border: '1px solid #ddd', borderRadius: 4 }}
    />
  );
}

// ─── Step state ──────────────────────────────────────────────────────────────

type Step = 'idle' | 'setup-pending' | 'verify-setup' | 'disable' | 'regen-codes';

// ─── Component ───────────────────────────────────────────────────────────────

export function TwoFactorAuthPage() {
  const { isAuthenticated, isLoading } = useAuth();
  const { showToast } = useToast();

  // Server state
  const statusQuery = useMfaStatus();
  const setupMutation = useMfaSetup();
  const verifyMutation = useMfaVerify();
  const disableMutation = useMfaDisable();
  const regenMutation = useMfaRegenerateBackupCodes();

  // Local UI state
  const [step, setStep] = useState<Step>('idle');
  const [code, setCode] = useState('');
  const [formError, setFormError] = useState<string>();
  const [newBackupCodes, setNewBackupCodes] = useState<string[]>([]);

  // ── Handlers ──

  const handleStartSetup = useCallback(async () => {
    setFormError(undefined);
    setCode('');
    try {
      await setupMutation.mutateAsync();
      setStep('setup-pending');
    } catch (err) {
      showToast({ type: 'error', title: 'Could not start MFA setup', message: String(err) });
    }
  }, [setupMutation, showToast]);

  const handleVerifySetup = useCallback(
    async (event: React.FormEvent<HTMLFormElement>) => {
      event.preventDefault();
      setFormError(undefined);
      if (!/^\d{6}$/.test(code.trim())) {
        setFormError('Enter the 6-digit code from your authenticator app.');
        return;
      }
      try {
        await verifyMutation.mutateAsync({ code: code.trim() });
        setStep('idle');
        showToast({ type: 'success', title: 'Two-factor authentication enabled' });
      } catch (err) {
        setFormError(String(err) || 'The code was not accepted. Please try again.');
      }
    },
    [code, verifyMutation, showToast]
  );

  const handleStartDisable = useCallback(() => {
    setFormError(undefined);
    setCode('');
    setStep('disable');
  }, []);

  const handleDisable = useCallback(
    async (event: React.FormEvent<HTMLFormElement>) => {
      event.preventDefault();
      setFormError(undefined);
      if (!code.trim()) {
        setFormError('Enter your authenticator code or a backup code to confirm.');
        return;
      }
      try {
        await disableMutation.mutateAsync({ code: code.trim() });
        setStep('idle');
        showToast({ type: 'success', title: 'Two-factor authentication disabled' });
      } catch (err) {
        setFormError(String(err) || 'The code was not accepted. Please try again.');
      }
    },
    [code, disableMutation, showToast]
  );

  const handleStartRegen = useCallback(() => {
    setFormError(undefined);
    setCode('');
    setNewBackupCodes([]);
    setStep('regen-codes');
  }, []);

  const handleRegen = useCallback(
    async (event: React.FormEvent<HTMLFormElement>) => {
      event.preventDefault();
      setFormError(undefined);
      if (!/^\d{6}$/.test(code.trim())) {
        setFormError('Enter the 6-digit code from your authenticator app.');
        return;
      }
      try {
        const result = await regenMutation.mutateAsync({ code: code.trim() });
        setNewBackupCodes(result.backupCodes);
        showToast({ type: 'success', title: 'Backup codes regenerated — save them now!' });
      } catch (err) {
        setFormError(String(err) || 'The code was not accepted. Please try again.');
      }
    },
    [code, regenMutation, showToast]
  );

  const handleCancel = useCallback(() => {
    setStep('idle');
    setFormError(undefined);
    setCode('');
    setNewBackupCodes([]);
  }, []);

  // ── Auth guard ──

  if (isLoading) return null;
  if (!isAuthenticated) {
    return <Navigate to="/login" replace />;
  }

  const mfaEnabled = statusQuery.data?.enabled ?? false;
  const backupCodesRemaining = statusQuery.data?.backupCodesRemaining ?? 0;
  const setupData = setupMutation.data;

  // ─── Render ────────────────────────────────────────────────────────────────

  return (
    <div className="auth-page">
      <div className="auth-container">
        <div className="auth-header">
          <h1 className="auth-title">Two-factor authentication</h1>
          <p className="auth-subtitle">
            Add an extra layer of security by requiring a code from your authenticator app.
          </p>
        </div>

        {/* Loading status */}
        {statusQuery.isLoading && (
          <p className="auth-help" aria-live="polite">
            Loading MFA status…
          </p>
        )}

        {/* ── Idle: show current status + actions ── */}
        {step === 'idle' && !statusQuery.isLoading && (
          <>
            {mfaEnabled ? (
              <div className="auth-success-banner" role="status">
                Two-factor authentication is <strong>enabled</strong>.{' '}
                {backupCodesRemaining > 0 && (
                  <span>Backup codes remaining: {backupCodesRemaining}.</span>
                )}
              </div>
            ) : (
              <p className="auth-help">Two-factor authentication is not yet enabled.</p>
            )}

            <div
              className="auth-actions"
              style={{ display: 'flex', gap: '0.75rem', flexWrap: 'wrap', marginTop: '1rem' }}
            >
              {!mfaEnabled && (
                <button
                  type="button"
                  className="auth-submit"
                  onClick={handleStartSetup}
                  disabled={setupMutation.isPending}
                >
                  {setupMutation.isPending ? 'Starting setup…' : 'Set up two-factor authentication'}
                </button>
              )}

              {mfaEnabled && (
                <>
                  <button
                    type="button"
                    className="auth-submit"
                    onClick={handleStartDisable}
                    style={{ background: 'var(--color-danger, #c0392b)' }}
                  >
                    Disable two-factor authentication
                  </button>
                  <button
                    type="button"
                    className="auth-submit"
                    onClick={handleStartRegen}
                    style={{ background: 'var(--color-secondary, #6c757d)' }}
                  >
                    Regenerate backup codes
                  </button>
                </>
              )}
            </div>
          </>
        )}

        {/* ── Setup pending: show secret/QR + verify form ── */}
        {step === 'setup-pending' && setupData && (
          <>
            <p className="auth-help">
              Scan the QR code below with your authenticator app (Google Authenticator, Authy,
              etc.), or enter the secret manually.
            </p>

            <div style={{ textAlign: 'center', margin: '1rem 0' }}>
              <QrCodeCanvas uri={setupData.qrUri} />
            </div>

            <div className="auth-field">
              <p className="auth-label">Manual entry secret</p>
              <code
                style={{
                  display: 'block',
                  padding: '0.5rem',
                  background: '#f5f5f5',
                  borderRadius: 4,
                  wordBreak: 'break-all',
                  fontSize: '0.875rem',
                }}
              >
                {setupData.secret}
              </code>
            </div>

            {setupData.backupCodes.length > 0 && (
              <div className="auth-field" style={{ marginTop: '1rem' }}>
                <p className="auth-label" style={{ fontWeight: 700 }}>
                  Save these backup codes now — they will not be shown again.
                </p>
                <ul style={{ fontFamily: 'monospace', fontSize: '0.9rem', paddingLeft: '1.25rem' }}>
                  {setupData.backupCodes.map((bc) => (
                    <li key={bc}>{bc}</li>
                  ))}
                </ul>
              </div>
            )}

            <form
              className="auth-form"
              onSubmit={handleVerifySetup}
              noValidate
              style={{ marginTop: '1.5rem' }}
            >
              <div className="auth-field">
                <label htmlFor="setup-code" className="auth-label">
                  Verification code
                </label>
                <input
                  id="setup-code"
                  name="code"
                  type="text"
                  inputMode="numeric"
                  autoComplete="one-time-code"
                  pattern="[0-9]{6}"
                  maxLength={6}
                  value={code}
                  onChange={(e) => setCode(e.target.value.replace(/\D/g, ''))}
                  className={`auth-input ${formError ? 'auth-input--error' : ''}`}
                  aria-invalid={formError ? 'true' : 'false'}
                />
                {formError && <span className="auth-field-error">{formError}</span>}
              </div>
              <div style={{ display: 'flex', gap: '0.75rem' }}>
                <button type="submit" className="auth-submit" disabled={verifyMutation.isPending}>
                  {verifyMutation.isPending ? 'Verifying…' : 'Verify and enable'}
                </button>
                <button type="button" className="auth-link" onClick={handleCancel}>
                  Cancel
                </button>
              </div>
            </form>
          </>
        )}

        {/* ── Disable: confirm code ── */}
        {step === 'disable' && (
          <form className="auth-form" onSubmit={handleDisable} noValidate>
            <p className="auth-help">
              Enter your authenticator code or a backup code to confirm disabling 2FA.
            </p>
            <div className="auth-field">
              <label htmlFor="disable-code" className="auth-label">
                Authentication code
              </label>
              <input
                id="disable-code"
                name="code"
                type="text"
                inputMode="numeric"
                autoComplete="one-time-code"
                maxLength={20}
                value={code}
                onChange={(e) => setCode(e.target.value.trim())}
                className={`auth-input ${formError ? 'auth-input--error' : ''}`}
                aria-invalid={formError ? 'true' : 'false'}
              />
              {formError && <span className="auth-field-error">{formError}</span>}
            </div>
            <div style={{ display: 'flex', gap: '0.75rem' }}>
              <button
                type="submit"
                className="auth-submit"
                style={{ background: 'var(--color-danger, #c0392b)' }}
                disabled={disableMutation.isPending}
              >
                {disableMutation.isPending ? 'Disabling…' : 'Disable two-factor authentication'}
              </button>
              <button type="button" className="auth-link" onClick={handleCancel}>
                Cancel
              </button>
            </div>
          </form>
        )}

        {/* ── Regenerate backup codes ── */}
        {step === 'regen-codes' && (
          <>
            {newBackupCodes.length === 0 ? (
              <form className="auth-form" onSubmit={handleRegen} noValidate>
                <p className="auth-help">
                  Enter your authenticator code to regenerate backup codes. All existing backup
                  codes will be invalidated.
                </p>
                <div className="auth-field">
                  <label htmlFor="regen-code" className="auth-label">
                    Authenticator code
                  </label>
                  <input
                    id="regen-code"
                    name="code"
                    type="text"
                    inputMode="numeric"
                    autoComplete="one-time-code"
                    pattern="[0-9]{6}"
                    maxLength={6}
                    value={code}
                    onChange={(e) => setCode(e.target.value.replace(/\D/g, ''))}
                    className={`auth-input ${formError ? 'auth-input--error' : ''}`}
                    aria-invalid={formError ? 'true' : 'false'}
                  />
                  {formError && <span className="auth-field-error">{formError}</span>}
                </div>
                <div style={{ display: 'flex', gap: '0.75rem' }}>
                  <button type="submit" className="auth-submit" disabled={regenMutation.isPending}>
                    {regenMutation.isPending ? 'Regenerating…' : 'Regenerate backup codes'}
                  </button>
                  <button type="button" className="auth-link" onClick={handleCancel}>
                    Cancel
                  </button>
                </div>
              </form>
            ) : (
              <div>
                <p className="auth-help" style={{ fontWeight: 700 }}>
                  Save these new backup codes — they will not be shown again.
                </p>
                <ul style={{ fontFamily: 'monospace', fontSize: '0.9rem', paddingLeft: '1.25rem' }}>
                  {newBackupCodes.map((bc) => (
                    <li key={bc}>{bc}</li>
                  ))}
                </ul>
                <button
                  type="button"
                  className="auth-submit"
                  onClick={handleCancel}
                  style={{ marginTop: '1rem' }}
                >
                  Done
                </button>
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}

TwoFactorAuthPage.displayName = 'TwoFactorAuthPage';
