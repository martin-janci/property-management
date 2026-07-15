/**
 * DocumentSignPage — signer-facing public signing page (Epic 84.2).
 *
 * Opened from the emailed `{BASE_URL}/sign?token=<v1…>` link. There is NO auth:
 * the HMAC `token` in the query string is the only credential. This page is the
 * frontend consumer for the already-shipped backend `/api/v1/signatures/sign`
 * endpoints:
 *
 *   GET  /api/v1/signatures/sign?token=…  → render context (does not consume nonce)
 *   POST /api/v1/signatures/sign?token=…  → record the signature (single-use)
 *
 * The manager-side half (create request / list signer status / remind / cancel)
 * lives on `DocumentSignaturePanel` (document detail) and is unrelated here.
 *
 * Because this route is public we consume the `@ppt/api-client` signer hooks
 * (`useSignContext` / `useSubmitSignature`) which use an auth-less `fetch` and
 * surface a typed `SignError` carrying the backend error `code`, so we can
 * branch the UI on EXPIRED / INVALID / ALREADY_SIGNED rather than a generic
 * error toast.
 */

import { SignError, useSignContext, useSubmitSignature } from '@ppt/api-client';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useSearchParams } from 'react-router-dom';

/** Error codes the backend returns from the `/sign` endpoints. */
type SignErrorCode =
  | 'SIGNING_LINK_EXPIRED'
  | 'INVALID_SIGNING_LINK'
  | 'REQUEST_NOT_OPEN'
  | 'ALREADY_SIGNED'
  | 'LINK_ALREADY_USED';

/** Map a caught error to the i18n key for its friendly copy. */
function errorMessageKey(error: unknown): string {
  if (error instanceof SignError) {
    const code = error.code as SignErrorCode;
    switch (code) {
      case 'SIGNING_LINK_EXPIRED':
        return 'documentSign.errors.expired';
      case 'ALREADY_SIGNED':
        return 'documentSign.errors.alreadySigned';
      case 'LINK_ALREADY_USED':
        return 'documentSign.errors.linkUsed';
      case 'REQUEST_NOT_OPEN':
        return 'documentSign.errors.notOpen';
      default:
        return 'documentSign.errors.invalid';
    }
  }
  return 'documentSign.errors.generic';
}

/** A signer who has already completed should see a friendly state, not a form. */
function isTerminalSignerStatus(status: string): boolean {
  return status === 'signed' || status === 'declined';
}

/** Card shell so every state (loading / error / form / done) shares one frame. */
function SignCard({ children }: { children: React.ReactNode }) {
  return (
    <div className="max-w-xl mx-auto px-4 py-10">
      <div
        className="rounded-xl p-6 md:p-8"
        style={{
          backgroundColor: 'var(--color-surface)',
          border: '1px solid var(--color-border)',
        }}
      >
        {children}
      </div>
    </div>
  );
}

export function DocumentSignPage() {
  const { t } = useTranslation();
  const [searchParams] = useSearchParams();
  const token = searchParams.get('token') ?? undefined;

  const { data: context, isLoading, isError, error: contextError } = useSignContext(token);

  const submit = useSubmitSignature(token ?? '');

  const [typedName, setTypedName] = useState('');
  const [nameError, setNameError] = useState<string>();

  // Prefill the adopt-signature name with the signer's roster name once loaded.
  useEffect(() => {
    if (context?.signer_name) {
      setTypedName((current) => current || context.signer_name);
    }
  }, [context?.signer_name]);

  // --- No token in the URL --------------------------------------------------
  if (!token) {
    return (
      <SignCard>
        <h1 className="text-xl font-semibold mb-2">{t('documentSign.errors.invalidTitle')}</h1>
        <p style={{ color: 'var(--color-text-secondary)' }}>{t('documentSign.errors.invalid')}</p>
      </SignCard>
    );
  }

  // --- Loading render context ----------------------------------------------
  if (isLoading) {
    return (
      <SignCard>
        <p style={{ color: 'var(--color-text-secondary)' }}>{t('documentSign.loading')}</p>
      </SignCard>
    );
  }

  // --- Render-context error (expired / invalid / already-signed) -----------
  if (isError || !context) {
    // ALREADY_SIGNED comes back as an error from GET, but it is a friendly
    // "you're done" state rather than a failure.
    const alreadySigned =
      contextError instanceof SignError && contextError.code === 'ALREADY_SIGNED';
    return (
      <SignCard>
        <h1 className="text-xl font-semibold mb-2">
          {t(
            alreadySigned ? 'documentSign.alreadySignedTitle' : 'documentSign.errors.invalidTitle'
          )}
        </h1>
        <p style={{ color: 'var(--color-text-secondary)' }}>{t(errorMessageKey(contextError))}</p>
      </SignCard>
    );
  }

  // --- Signer already completed (status echoed in the context) -------------
  if (isTerminalSignerStatus(context.signer_status)) {
    return (
      <SignCard>
        <h1 className="text-xl font-semibold mb-2">{t('documentSign.alreadySignedTitle')}</h1>
        <p style={{ color: 'var(--color-text-secondary)' }}>
          {t('documentSign.errors.alreadySigned')}
        </p>
      </SignCard>
    );
  }

  // --- Successful submit confirmation --------------------------------------
  if (submit.isSuccess) {
    const completed = submit.data.status === 'completed';
    return (
      <SignCard>
        <h1 className="text-xl font-semibold mb-2">{t('documentSign.confirmationTitle')}</h1>
        <p style={{ color: 'var(--color-text-secondary)' }}>{submit.data.message}</p>
        <p className="mt-3 text-sm" style={{ color: 'var(--color-text-secondary)' }}>
          {t(completed ? 'documentSign.allSigned' : 'documentSign.othersRemain')}
        </p>
      </SignCard>
    );
  }

  const handleSubmit = (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setNameError(undefined);
    const trimmed = typedName.trim();
    if (!trimmed) {
      setNameError(t('documentSign.form.nameRequired'));
      return;
    }
    submit.mutate({ typed_name: trimmed });
  };

  const submitError = submit.isError ? submit.error : undefined;

  return (
    <SignCard>
      <h1 className="text-2xl font-bold mb-1">{t('documentSign.title')}</h1>
      <p className="mb-6" style={{ color: 'var(--color-text-secondary)' }}>
        {t('documentSign.subtitle')}
      </p>

      {/* Document + requester context */}
      <dl className="space-y-3 mb-6">
        <div>
          <dt
            className="text-xs uppercase tracking-wide"
            style={{ color: 'var(--color-text-secondary)' }}
          >
            {t('documentSign.documentLabel')}
          </dt>
          <dd className="font-medium">{context.subject || t('documentSign.untitledDocument')}</dd>
        </div>
        {context.message && (
          <div>
            <dt
              className="text-xs uppercase tracking-wide"
              style={{ color: 'var(--color-text-secondary)' }}
            >
              {t('documentSign.messageLabel')}
            </dt>
            <dd style={{ whiteSpace: 'pre-wrap' }}>{context.message}</dd>
          </div>
        )}
        <div>
          <dt
            className="text-xs uppercase tracking-wide"
            style={{ color: 'var(--color-text-secondary)' }}
          >
            {t('documentSign.signerLabel')}
          </dt>
          <dd className="font-medium">
            {context.signer_name}{' '}
            <span style={{ color: 'var(--color-text-secondary)' }}>({context.signer_email})</span>
          </dd>
        </div>
      </dl>

      {/* Adopt & sign */}
      <form onSubmit={handleSubmit} noValidate>
        <label htmlFor="typed-name" className="block text-sm font-medium mb-1">
          {t('documentSign.form.nameLabel')}
        </label>
        <input
          id="typed-name"
          type="text"
          value={typedName}
          onChange={(event) => setTypedName(event.target.value)}
          disabled={submit.isPending}
          className="w-full rounded-lg px-3 py-2 mb-1"
          style={{
            backgroundColor: 'var(--color-background)',
            border: `1px solid ${nameError ? 'var(--color-danger, #dc2626)' : 'var(--color-border)'}`,
            color: 'var(--color-text)',
          }}
          aria-invalid={!!nameError}
          aria-describedby={nameError ? 'typed-name-error' : undefined}
        />
        {nameError && (
          <p
            id="typed-name-error"
            className="text-sm mb-2"
            style={{ color: 'var(--color-danger, #dc2626)' }}
          >
            {nameError}
          </p>
        )}
        <p className="text-xs mb-4" style={{ color: 'var(--color-text-secondary)' }}>
          {t('documentSign.form.consent')}
        </p>

        {submitError && (
          <p className="text-sm mb-4" style={{ color: 'var(--color-danger, #dc2626)' }}>
            {t(errorMessageKey(submitError))}
          </p>
        )}

        <button
          type="submit"
          disabled={submit.isPending}
          className="btn-primary-token px-5 py-2.5 rounded-lg font-medium disabled:opacity-60"
        >
          {submit.isPending ? t('documentSign.form.submitting') : t('documentSign.form.submit')}
        </button>
      </form>
    </SignCard>
  );
}
