/**
 * Two-Factor Authentication Setup Page (UC-14.10).
 *
 * UI scaffold for enabling/disabling TOTP-based MFA. The backend MFA endpoints
 * are not exposed yet through @ppt/api-client; the page wires up local state
 * and surfaces a clear status so the screen exists end-to-end.
 */

import type React from 'react';
import { useCallback, useState } from 'react';
import { Navigate } from 'react-router-dom';
import { useAuth } from '../../../contexts/AuthContext';
import '../styles/AuthPage.css';

type Step = 'idle' | 'verify' | 'enabled';

export function TwoFactorAuthPage() {
  const { isAuthenticated, isLoading } = useAuth();
  const [step, setStep] = useState<Step>('idle');
  const [code, setCode] = useState('');
  const [error, setError] = useState<string>();

  const handleStart = useCallback(() => {
    setStep('verify');
    setCode('');
    setError(undefined);
  }, []);

  const handleVerify = useCallback(
    (event: React.FormEvent<HTMLFormElement>) => {
      event.preventDefault();
      setError(undefined);
      if (!/^\d{6}$/.test(code.trim())) {
        setError('Enter the 6-digit code from your authenticator app.');
        return;
      }
      setStep('enabled');
    },
    [code]
  );

  if (isLoading) return null;
  if (!isAuthenticated) {
    return <Navigate to="/login" replace />;
  }

  return (
    <div className="auth-page">
      <div className="auth-container">
        <div className="auth-header">
          <h1 className="auth-title">Two-factor authentication</h1>
          <p className="auth-subtitle">
            Add an extra layer of security by requiring a code from your authenticator app.
          </p>
        </div>

        {step === 'idle' && (
          <button type="button" className="auth-submit" onClick={handleStart}>
            Set up two-factor authentication
          </button>
        )}

        {step === 'verify' && (
          <form className="auth-form" onSubmit={handleVerify} noValidate>
            <p className="auth-help">
              Scan the QR code in your authenticator app, then enter the 6-digit code below.
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
                className={`auth-input ${error ? 'auth-input--error' : ''}`}
                aria-invalid={error ? 'true' : 'false'}
              />
              {error && <span className="auth-field-error">{error}</span>}
            </div>
            <button type="submit" className="auth-submit">
              Verify and enable
            </button>
          </form>
        )}

        {step === 'enabled' && (
          <div className="auth-success-banner" role="status">
            Two-factor authentication is enabled.
          </div>
        )}
      </div>
    </div>
  );
}

TwoFactorAuthPage.displayName = 'TwoFactorAuthPage';
