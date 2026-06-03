/**
 * Two-Factor Authentication Setup Page (UC-14.10, Epic 9, Stories 9.1 + 9.2).
 *
 * Wires to the backend MFA endpoints via @ppt/api-client useMfa* hooks:
 *   POST /api/v1/auth/mfa/setup              — initiate TOTP setup (returns secret + QR)
 *   POST /api/v1/auth/mfa/verify             — confirm TOTP code, enables 2FA, returns 10
 *                                              single-use recovery codes (shown ONCE)
 *   POST /api/v1/auth/mfa/disable            — disable 2FA (requires code)
 *   GET  /api/v1/auth/mfa/status             — current enabled/disabled state + remaining codes
 *   POST /api/v1/auth/mfa/backup-codes/regenerate — refresh recovery codes
 *
 * Recovery-codes UI (Story 9.2):
 *   - The 10 codes are surfaced once on enable and once on each regenerate.
 *   - A "Copy all" action copies the whole set to the clipboard.
 *   - A warning banner appears when the remaining count is low or exhausted,
 *     prompting the user to regenerate before they are locked out.
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
import { Trans, useTranslation } from 'react-i18next';
import { Navigate } from 'react-router-dom';
import { useToast } from '../../../components/Toast';
import { useAuth } from '../../../contexts/AuthContext';
import '../styles/AuthPage.css';

// Threshold below which we nudge the user to regenerate codes (0 = exhausted).
const LOW_CODES_THRESHOLD = 3;

// ─── QR code canvas renderer ─────────────────────────────────────────────────

function QrCodeCanvas({ uri }: { uri: string }) {
  const { t } = useTranslation();
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    if (canvasRef.current) {
      QRCode.toCanvas(canvasRef.current, uri, { width: 200, margin: 2 }).catch(() => {});
    }
  }, [uri]);

  return (
    <canvas
      ref={canvasRef}
      aria-label={t('auth.twoFactor.qrAriaLabel')}
      style={{ border: '1px solid #ddd', borderRadius: 4 }}
    />
  );
}

// ─── Recovery codes panel ────────────────────────────────────────────────────

/**
 * Displays a one-time list of recovery codes with copy-all and download
 * actions. Reused by the enable flow and the regenerate flow.
 */
function RecoveryCodesPanel({ codes }: { codes: string[] }) {
  const { t } = useTranslation();
  const { showToast } = useToast();
  const [copied, setCopied] = useState(false);

  const handleCopyAll = useCallback(async () => {
    const text = codes.join('\n');
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
      showToast({ type: 'success', title: t('auth.twoFactor.copiedToast') });
    } catch {
      showToast({
        type: 'error',
        title: t('auth.twoFactor.copyFailedTitle'),
        message: t('auth.twoFactor.copyFailedMessage'),
      });
    }
  }, [codes, showToast, t]);

  return (
    <div>
      <ul className="auth-recovery-codes" aria-label={t('auth.twoFactor.recoveryCodesAriaLabel')}>
        {codes.map((c) => (
          <li key={c}>{c}</li>
        ))}
      </ul>
      <div className="auth-recovery-actions">
        <button type="button" className="auth-submit" onClick={handleCopyAll}>
          {copied ? t('auth.twoFactor.copied') : t('auth.twoFactor.copyAll')}
        </button>
      </div>
    </div>
  );
}

// ─── Step state ──────────────────────────────────────────────────────────────

type Step = 'idle' | 'setup-pending' | 'show-codes' | 'disable' | 'regen-codes';

// ─── Component ───────────────────────────────────────────────────────────────

export function TwoFactorAuthPage() {
  const { t } = useTranslation();
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
  // Recovery codes are held only in memory, shown once, then cleared.
  const [recoveryCodes, setRecoveryCodes] = useState<string[]>([]);

  // ── Handlers ──

  const handleStartSetup = useCallback(async () => {
    setFormError(undefined);
    setCode('');
    try {
      await setupMutation.mutateAsync();
      setStep('setup-pending');
    } catch (err) {
      showToast({
        type: 'error',
        title: t('auth.twoFactor.setupErrorTitle'),
        message: String(err),
      });
    }
  }, [setupMutation, showToast, t]);

  const handleVerifySetup = useCallback(
    async (event: React.FormEvent<HTMLFormElement>) => {
      event.preventDefault();
      setFormError(undefined);
      if (!/^\d{6}$/.test(code.trim())) {
        setFormError(t('auth.twoFactor.enterCode'));
        return;
      }
      try {
        const result = await verifyMutation.mutateAsync({ code: code.trim() });
        setRecoveryCodes(result.recoveryCodes ?? []);
        setStep('show-codes');
        showToast({ type: 'success', title: t('auth.twoFactor.enabledToast') });
      } catch (err) {
        setFormError(String(err) || t('auth.twoFactor.codeNotAccepted'));
      }
    },
    [code, verifyMutation, showToast, t]
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
        setFormError(t('auth.twoFactor.enterCodeToDisable'));
        return;
      }
      try {
        await disableMutation.mutateAsync({ code: code.trim() });
        setStep('idle');
        showToast({ type: 'success', title: t('auth.twoFactor.disabledToast') });
      } catch (err) {
        setFormError(String(err) || t('auth.twoFactor.codeNotAccepted'));
      }
    },
    [code, disableMutation, showToast, t]
  );

  const handleStartRegen = useCallback(() => {
    setFormError(undefined);
    setCode('');
    setRecoveryCodes([]);
    setStep('regen-codes');
  }, []);

  const handleRegen = useCallback(
    async (event: React.FormEvent<HTMLFormElement>) => {
      event.preventDefault();
      setFormError(undefined);
      if (!/^\d{6}$/.test(code.trim())) {
        setFormError(t('auth.twoFactor.enterCode'));
        return;
      }
      try {
        const result = await regenMutation.mutateAsync({ code: code.trim() });
        setRecoveryCodes(result.backupCodes);
        showToast({ type: 'success', title: t('auth.twoFactor.regeneratedToast') });
      } catch (err) {
        setFormError(String(err) || t('auth.twoFactor.codeNotAccepted'));
      }
    },
    [code, regenMutation, showToast, t]
  );

  const handleCancel = useCallback(() => {
    setStep('idle');
    setFormError(undefined);
    setCode('');
    setRecoveryCodes([]);
  }, []);

  // ── Auth guard ──

  if (isLoading) return null;
  if (!isAuthenticated) {
    return <Navigate to="/login" replace />;
  }

  const mfaEnabled = statusQuery.data?.enabled ?? false;
  const backupCodesRemaining = statusQuery.data?.backupCodesRemaining ?? 0;
  const codesExhausted = mfaEnabled && backupCodesRemaining === 0;
  const codesLow =
    mfaEnabled && backupCodesRemaining > 0 && backupCodesRemaining <= LOW_CODES_THRESHOLD;
  const setupData = setupMutation.data;

  // ─── Render ────────────────────────────────────────────────────────────────

  return (
    <div className="auth-page">
      <div className="auth-container">
        <div className="auth-header">
          <h1 className="auth-title">{t('auth.twoFactor.title')}</h1>
          <p className="auth-subtitle">{t('auth.twoFactor.subtitle')}</p>
        </div>

        {/* Loading status */}
        {statusQuery.isLoading && (
          <p className="auth-help" aria-live="polite">
            {t('auth.twoFactor.loadingStatus')}
          </p>
        )}

        {/* ── Idle: show current status + actions ── */}
        {step === 'idle' && !statusQuery.isLoading && (
          <>
            {mfaEnabled ? (
              <div className="auth-success-banner" role="status">
                <Trans i18nKey="auth.twoFactor.enabledBanner" components={{ 1: <strong /> }} />{' '}
                <span>{t('auth.twoFactor.codesRemaining', { count: backupCodesRemaining })}</span>
              </div>
            ) : (
              <p className="auth-help">{t('auth.twoFactor.notEnabled')}</p>
            )}

            {/* Exhausted / low recovery-codes warning (Story 9.2) */}
            {codesExhausted && (
              <div className="auth-warning-banner" role="alert">
                <span>
                  <Trans i18nKey="auth.twoFactor.codesExhausted" components={{ 1: <strong /> }} />
                </span>
              </div>
            )}
            {codesLow && (
              <div className="auth-warning-banner" role="status">
                <span>{t('auth.twoFactor.codesLow', { count: backupCodesRemaining })}</span>
              </div>
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
                  {setupMutation.isPending
                    ? t('auth.twoFactor.startingSetup')
                    : t('auth.twoFactor.setupButton')}
                </button>
              )}

              {mfaEnabled && (
                <>
                  <button
                    type="button"
                    className="auth-submit"
                    onClick={handleStartRegen}
                    style={
                      codesExhausted ? undefined : { background: 'var(--color-secondary, #6c757d)' }
                    }
                  >
                    {t('auth.twoFactor.regenerateButton')}
                  </button>
                  <button
                    type="button"
                    className="auth-submit"
                    onClick={handleStartDisable}
                    style={{ background: 'var(--color-danger, #c0392b)' }}
                  >
                    {t('auth.twoFactor.disableButton')}
                  </button>
                </>
              )}
            </div>
          </>
        )}

        {/* ── Setup pending: show secret/QR + verify form ── */}
        {step === 'setup-pending' && setupData && (
          <>
            <p className="auth-help">{t('auth.twoFactor.setupInstructions')}</p>

            <div style={{ textAlign: 'center', margin: '1rem 0' }}>
              <QrCodeCanvas uri={setupData.qrUri} />
            </div>

            <div className="auth-field">
              <p className="auth-label">{t('auth.twoFactor.manualEntrySecret')}</p>
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

            <form
              className="auth-form"
              onSubmit={handleVerifySetup}
              noValidate
              style={{ marginTop: '1.5rem' }}
            >
              <div className="auth-field">
                <label htmlFor="setup-code" className="auth-label">
                  {t('auth.twoFactor.verificationCode')}
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
                  {verifyMutation.isPending
                    ? t('auth.twoFactor.verifying')
                    : t('auth.twoFactor.verifyAndEnable')}
                </button>
                <button type="button" className="auth-link" onClick={handleCancel}>
                  {t('auth.twoFactor.cancel')}
                </button>
              </div>
            </form>
          </>
        )}

        {/* ── Show recovery codes once after enable ── */}
        {step === 'show-codes' && (
          <>
            <div className="auth-success-banner" role="status">
              <Trans i18nKey="auth.twoFactor.enabledSuccess" components={{ 1: <strong /> }} />
            </div>
            <p className="auth-help" style={{ fontWeight: 700, marginTop: '1rem' }}>
              {t('auth.twoFactor.saveCodesNow', { count: recoveryCodes.length })}
            </p>
            <RecoveryCodesPanel codes={recoveryCodes} />
            <button
              type="button"
              className="auth-submit"
              onClick={handleCancel}
              style={{ marginTop: '1rem' }}
            >
              {t('auth.twoFactor.savedCodes')}
            </button>
          </>
        )}

        {/* ── Disable: confirm code ── */}
        {step === 'disable' && (
          <form className="auth-form" onSubmit={handleDisable} noValidate>
            <p className="auth-help">{t('auth.twoFactor.disableInstructions')}</p>
            <div className="auth-field">
              <label htmlFor="disable-code" className="auth-label">
                {t('auth.twoFactor.authenticationCode')}
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
                {disableMutation.isPending
                  ? t('auth.twoFactor.disabling')
                  : t('auth.twoFactor.disableButton')}
              </button>
              <button type="button" className="auth-link" onClick={handleCancel}>
                {t('auth.twoFactor.cancel')}
              </button>
            </div>
          </form>
        )}

        {/* ── Regenerate recovery codes ── */}
        {step === 'regen-codes' && (
          <>
            {recoveryCodes.length === 0 ? (
              <form className="auth-form" onSubmit={handleRegen} noValidate>
                <p className="auth-help">{t('auth.twoFactor.regenInstructions')}</p>
                <div className="auth-field">
                  <label htmlFor="regen-code" className="auth-label">
                    {t('auth.twoFactor.authenticatorCode')}
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
                    {regenMutation.isPending
                      ? t('auth.twoFactor.regenerating')
                      : t('auth.twoFactor.regenerateButton')}
                  </button>
                  <button type="button" className="auth-link" onClick={handleCancel}>
                    {t('auth.twoFactor.cancel')}
                  </button>
                </div>
              </form>
            ) : (
              <>
                <p className="auth-help" style={{ fontWeight: 700 }}>
                  {t('auth.twoFactor.saveNewCodes', { count: recoveryCodes.length })}
                </p>
                <RecoveryCodesPanel codes={recoveryCodes} />
                <button
                  type="button"
                  className="auth-submit"
                  onClick={handleCancel}
                  style={{ marginTop: '1rem' }}
                >
                  {t('auth.twoFactor.done')}
                </button>
              </>
            )}
          </>
        )}
      </div>
    </div>
  );
}

TwoFactorAuthPage.displayName = 'TwoFactorAuthPage';
