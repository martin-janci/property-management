/**
 * Two-Factor Authentication Setup Page (UC-14.10, Story 9.1).
 *
 * Wires TOTP-based MFA to /api/v1/auth/mfa/* via useMfa* hooks
 * from @ppt/api-client. Supports: status display, setup (QR + verify),
 * and disable flows.
 */

import type React from 'react';
import { useCallback, useState } from 'react';
import { Navigate } from 'react-router-dom';
import {
  useMfaDisable,
  useMfaSetup,
  useMfaStatus,
  useMfaVerify,
  type MfaSetupResponse,
} from '@ppt/api-client';
import { useAuth } from '../../../contexts/AuthContext';
import '../styles/AuthPage.css';

type View = 'idle' | 'setup-qr' | 'setup-verify' | 'disable';

export function TwoFactorAuthPage() {
  const { isAuthenticated, isLoading: authLoading } = useAuth();

  const { data: mfaStatus, isLoading: statusLoading } = useMfaStatus();
  const setupMutation = useMfaSetup();
  const verifyMutation = useMfaVerify();
  const disableMutation = useMfaDisable();

  const [view, setView] = useState<View>('idle');
  const [setupData, setSetupData] = useState<MfaSetupResponse | null>(null);
  const [code, setCode] = useState('');
  const [localError, setLocalError] = useState<string | undefined>();

  const handleStartSetup = useCallback(async () => {
    setLocalError(undefined);
    setCode('');
    try {
      const data = await setupMutation.mutateAsync();
      setSetupData(data);
      setView('setup-qr');
    } catch (err) {
      setLocalError(err instanceof Error ? err.message : 'Failed to initiate MFA setup.');
    }
  }, [setupMutation]);

  const handleProceedToVerify = useCallback(() => {
    setCode('');
    setLocalError(undefined);
    setView('setup-verify');
  }, []);

  const handleVerify = useCallback(
    async (event: React.FormEvent<HTMLFormElement>) => {
      event.preventDefault();
      setLocalError(undefined);
      if (!/^\d{6}$/.test(code.trim())) {
        setLocalError('Enter the 6-digit code from your authenticator app.');
        return;
      }
      try {
        await verifyMutation.mutateAsync({ code: code.trim() });
        setView('idle');
        setSetupData(null);
        setCode('');
      } catch (err) {
        setLocalError(err instanceof Error ? err.message : 'Verification failed. Try again.');
      }
    },
    [code, verifyMutation]
  );

  const handleStartDisable = useCallback(() => {
    setCode('');
    setLocalError(undefined);
    setView('disable');
  }, []);

  const handleDisable = useCallback(
    async (event: React.FormEvent<HTMLFormElement>) => {
      event.preventDefault();
      setLocalError(undefined);
      if (!code.trim()) {
        setLocalError('Enter your authenticator code or a backup code.');
        return;
      }
      try {
        await disableMutation.mutateAsync({ code: code.trim() });
        setView('idle');
        setCode('');
      } catch (err) {
        setLocalError(err instanceof Error ? err.message : 'Failed to disable MFA. Try again.');
      }
    },
    [code, disableMutation]
  );

  const handleCancel = useCallback(() => {
    setView('idle');
    setCode('');
    setSetupData(null);
    setLocalError(undefined);
  }, []);

  if (authLoading) return null;
  if (!isAuthenticated) return <Navigate to="/login" replace />;

  const isBusy =
    setupMutation.isPending || verifyMutation.isPending || disableMutation.isPending;

  return (
    <div className="auth-page">
      <div className="auth-container">
        <div className="auth-header">
          <h1 className="auth-title">Two-factor authentication</h1>
          <p className="auth-subtitle">
            Add an extra layer of security by requiring a code from your authenticator app.
          </p>
        </div>

        {view === 'idle' && (
          <>
            {statusLoading && (
              <p className="auth-help" aria-live="polite">
                Checking MFA status…
              </p>
            )}

            {!statusLoading && mfaStatus?.enabled && (
              <div className="auth-success-banner" role="status">
                Two-factor authentication is <strong>enabled</strong>.{' '}
                {mfaStatus.backupCodesRemaining > 0
                  ? `${mfaStatus.backupCodesRemaining} backup codes remaining.`
                  : 'No backup codes remaining — consider regenerating.'}
              </div>
            )}

            {localError && (
              <span className="auth-field-error" role="alert">
                {localError}
              </span>
            )}

            {!statusLoading && mfaStatus?.enabled ? (
              <button
                type="button"
                className="auth-submit auth-submit--danger"
                onClick={handleStartDisable}
                disabled={isBusy}
              >
                Disable two-factor authentication
              </button>
            ) : (
              <button
                type="button"
                className="auth-submit"
                onClick={handleStartSetup}
                disabled={isBusy || statusLoading}
              >
                {setupMutation.isPending ? 'Setting up…' : 'Set up two-factor authentication'}
              </button>
            )}
          </>
        )}

        {view === 'setup-qr' && setupData && (
          <div>
            <p className="auth-help">
              Scan the QR code below with your authenticator app, then click{' '}
              <strong>Continue</strong>.
            </p>
            <div className="auth-qr-wrapper" aria-label="QR code for authenticator app">
              <img
                src={`https://api.qrserver.com/v1/create-qr-code/?data=${encodeURIComponent(setupData.qrUri)}&size=200x200`}
                alt="Scan this QR code with your authenticator app"
                width={200}
                height={200}
              />
            </div>
            <p className="auth-help">
              Can&apos;t scan? Manual entry key:{' '}
              <span className="auth-mono">
                <strong>{setupData.secret}</strong>
              </span>
            </p>
            <details className="auth-backup-codes">
              <summary>Show one-time backup codes (save these now)</summary>
              <ul className="auth-backup-codes__list" aria-label="Backup codes">
                {setupData.backupCodes.map((bc) => (
                  <li key={bc} className="auth-mono">
                    {bc}
                  </li>
                ))}
              </ul>
              <p className="auth-help auth-help--warning">
                These codes are shown only once. Store them securely.
              </p>
            </details>
            <div className="auth-actions">
              <button type="button" className="auth-submit" onClick={handleProceedToVerify}>
                Continue
              </button>
              <button type="button" className="auth-link" onClick={handleCancel}>
                Cancel
              </button>
            </div>
          </div>
        )}

        {view === 'setup-verify' && (
          <form className="auth-form" onSubmit={handleVerify} noValidate>
            <p className="auth-help">
              Enter the 6-digit code currently shown in your authenticator app.
            </p>
            <div className="auth-field">
              <label htmlFor="code" className="auth-label">
                Verification code
              </label>
              <input
                id="code"
                name="code"
                type="text"
                inputMode="numeric"
                autoComplete="one-time-code"
                pattern="[0-9]{6}"
                maxLength={6}
                value={code}
                onChange={(e) => setCode(e.target.value.replace(/\D/g, ''))}
                className={`auth-input ${localError ? 'auth-input--error' : ''}`}
                aria-invalid={localError ? 'true' : 'false'}
                aria-describedby={localError ? 'code-error' : undefined}
                disabled={isBusy}
              />
              {localError && (
                <span id="code-error" className="auth-field-error" role="alert">
                  {localError}
                </span>
              )}
            </div>
            <div className="auth-actions">
              <button type="submit" className="auth-submit" disabled={isBusy}>
                {verifyMutation.isPending ? 'Verifying…' : 'Verify and enable'}
              </button>
              <button
                type="button"
                className="auth-link"
                onClick={handleCancel}
                disabled={isBusy}
              >
                Cancel
              </button>
            </div>
          </form>
        )}

        {view === 'disable' && (
          <form className="auth-form" onSubmit={handleDisable} noValidate>
            <p className="auth-help">
              Enter your current authenticator code (or a backup code) to confirm disabling
              two-factor authentication.
            </p>
            <div className="auth-field">
              <label htmlFor="disable-code" className="auth-label">
                Authenticator code
              </label>
              <input
                id="disable-code"
                name="disable-code"
                type="text"
                inputMode="numeric"
                autoComplete="one-time-code"
                value={code}
                onChange={(e) => setCode(e.target.value.trim())}
                className={`auth-input ${localError ? 'auth-input--error' : ''}`}
                aria-invalid={localError ? 'true' : 'false'}
                aria-describedby={localError ? 'disable-code-error' : undefined}
                disabled={isBusy}
              />
              {localError && (
                <span id="disable-code-error" className="auth-field-error" role="alert">
                  {localError}
                </span>
              )}
            </div>
            <div className="auth-actions">
              <button
                type="submit"
                className="auth-submit auth-submit--danger"
                disabled={isBusy}
              >
                {disableMutation.isPending ? 'Disabling…' : 'Disable two-factor authentication'}
              </button>
              <button
                type="button"
                className="auth-link"
                onClick={handleCancel}
                disabled={isBusy}
              >
                Cancel
              </button>
            </div>
          </form>
        )}
      </div>
    </div>
  );
}

TwoFactorAuthPage.displayName = 'TwoFactorAuthPage';
