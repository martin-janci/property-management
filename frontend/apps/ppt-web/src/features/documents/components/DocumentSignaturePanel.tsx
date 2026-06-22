/**
 * DocumentSignaturePanel — e-signature surface for the document detail page
 * (Story 7B.3).
 *
 * Wires the existing (Epic 84) `signature_requests` backend into the document
 * detail screen: it lists every signature request raised for a document with
 * per-signer status, lets a manager initiate a new signing request, send
 * reminders to pending signers, and cancel an open request.
 *
 * The lease feature already proved the `@ppt/api-client` esignature hooks
 * against `/documents/{id}/signature-requests`; this panel reuses them with a
 * document-centric (free-form signer list) entry dialog rather than the
 * lease-party preset.
 */

import {
  type CreateSigner,
  type SignatureRequest,
  type SignatureRequestStatus,
  type Signer,
  type SignerStatus,
  useCancelSignatureRequest,
  useCreateSignatureRequest,
  useSendSignatureReminder,
  useSignatureRequests,
} from '@ppt/api-client';
import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

interface DocumentSignaturePanelProps {
  documentId: string;
}

/** Statuses for which an open request can still be reminded / cancelled. */
const OPEN_STATUSES: ReadonlySet<SignatureRequestStatus> = new Set(['pending', 'in_progress']);

const REQUEST_STATUS_CLASSES: Record<SignatureRequestStatus, string> = {
  pending: 'bg-amber-100 text-amber-800',
  in_progress: 'bg-amber-100 text-amber-800',
  completed: 'bg-green-100 text-green-800',
  declined: 'bg-red-100 text-red-800',
  expired: 'bg-gray-100 text-gray-600',
  cancelled: 'bg-gray-100 text-gray-600',
};

const SIGNER_STATUS_CLASSES: Record<SignerStatus, string> = {
  pending: 'bg-gray-100 text-gray-600',
  sent: 'bg-blue-100 text-blue-700',
  viewed: 'bg-indigo-100 text-indigo-700',
  signed: 'bg-green-100 text-green-800',
  declined: 'bg-red-100 text-red-800',
};

function formatDateTime(value: string | undefined): string {
  if (!value) return '';
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return '';
  return date.toLocaleString('en-US', {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

export function DocumentSignaturePanel({ documentId }: DocumentSignaturePanelProps) {
  const { t } = useTranslation();
  const { data, isLoading, error, refetch } = useSignatureRequests(documentId);
  const [showDialog, setShowDialog] = useState(false);

  const requests = data?.signature_requests ?? [];

  return (
    <div className="signature-panel">
      <div className="signature-panel__header">
        <button type="button" className="action-btn primary" onClick={() => setShowDialog(true)}>
          <svg
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            aria-hidden="true"
          >
            <path d="M12 19l7-7 3 3-7 7-3-3z" />
            <path d="M18 13l-1.5-7.5L2 2l3.5 14.5L13 18l5-5z" />
            <path d="M2 2l7.586 7.586" />
            <circle cx="11" cy="11" r="2" />
          </svg>
          {t('documents.esign.send', 'Send for signature')}
        </button>
      </div>

      {isLoading && (
        <p className="signature-empty">
          {t('documents.esign.loading', 'Loading signature requests…')}
        </p>
      )}

      {error && (
        <p className="signature-empty signature-empty--error">
          {t('documents.esign.loadError', 'Failed to load signature requests.')}
        </p>
      )}

      {!isLoading && !error && requests.length === 0 && (
        <p className="signature-empty">
          {t('documents.esign.empty', 'This document has not been sent for signature yet.')}
        </p>
      )}

      {requests.length > 0 && (
        <ul className="signature-list">
          {requests.map((req) => (
            <SignatureRequestCard
              key={req.id}
              documentId={documentId}
              request={req}
              onChanged={() => refetch()}
            />
          ))}
        </ul>
      )}

      {showDialog && (
        <CreateSignatureRequestDialog
          documentId={documentId}
          onClose={() => setShowDialog(false)}
          onCreated={() => {
            setShowDialog(false);
            refetch();
          }}
        />
      )}

      <style>{panelStyles}</style>
    </div>
  );
}

interface SignatureRequestCardProps {
  documentId: string;
  request: SignatureRequest;
  onChanged: () => void;
}

function SignatureRequestCard({ documentId, request, onChanged }: SignatureRequestCardProps) {
  const { t } = useTranslation();
  const remind = useSendSignatureReminder(documentId);
  const cancel = useCancelSignatureRequest(documentId);

  const isOpen = OPEN_STATUSES.has(request.status);
  const hasPending = request.signers.some((s) => s.status !== 'signed' && s.status !== 'declined');
  const actionError = remind.error ?? cancel.error;

  const statusLabels: Record<SignatureRequestStatus, string> = {
    pending: t('documents.esign.status.pending', 'Pending signature'),
    in_progress: t('documents.esign.status.in_progress', 'Signing in progress'),
    completed: t('documents.esign.status.completed', 'Signed'),
    declined: t('documents.esign.status.declined', 'Declined'),
    expired: t('documents.esign.status.expired', 'Expired'),
    cancelled: t('documents.esign.status.cancelled', 'Cancelled'),
  };

  const signerStatusLabels: Record<SignerStatus, string> = {
    pending: t('documents.esign.signer.pending', 'Pending'),
    sent: t('documents.esign.signer.sent', 'Sent'),
    viewed: t('documents.esign.signer.viewed', 'Viewed'),
    signed: t('documents.esign.signer.signed', 'Signed'),
    declined: t('documents.esign.signer.declined', 'Declined'),
  };

  const handleRemind = () => {
    remind.mutate({ id: request.id }, { onSuccess: onChanged });
  };

  const handleCancel = () => {
    if (
      !window.confirm(
        t(
          'documents.esign.cancelConfirm',
          'Cancel this signature request? Signers can no longer sign.'
        )
      )
    ) {
      return;
    }
    cancel.mutate({ id: request.id }, { onSuccess: onChanged });
  };

  return (
    <li className="signature-card">
      <div className="signature-card__head">
        <span className={`signature-badge ${REQUEST_STATUS_CLASSES[request.status]}`}>
          {statusLabels[request.status]}
        </span>
        {request.subject && <span className="signature-card__subject">{request.subject}</span>}
        <span className="signature-card__date">
          {t('documents.esign.requestedAt', {
            defaultValue: 'Requested {{date}}',
            date: formatDateTime(request.created_at),
          })}
        </span>
      </div>

      <ul className="signer-list">
        {request.signers
          .slice()
          .sort((a, b) => a.order - b.order)
          .map((signer) => (
            <SignerRow
              key={`${signer.email}-${signer.order}`}
              signer={signer}
              statusLabel={signerStatusLabels[signer.status]}
              signedAtLabel={
                signer.signed_at
                  ? t('documents.esign.signedAt', {
                      defaultValue: 'signed {{date}}',
                      date: formatDateTime(signer.signed_at),
                    })
                  : undefined
              }
            />
          ))}
      </ul>

      {isOpen && (
        <div className="signature-card__actions">
          <button
            type="button"
            className="action-btn"
            onClick={handleRemind}
            disabled={!hasPending || remind.isPending}
            title={
              hasPending ? undefined : t('documents.esign.allSigned', 'All signers have responded.')
            }
          >
            {remind.isPending
              ? t('documents.esign.reminding', 'Sending…')
              : t('documents.esign.remind', 'Send reminder')}
          </button>
          <button
            type="button"
            className="action-btn action-btn--danger"
            onClick={handleCancel}
            disabled={cancel.isPending}
          >
            {cancel.isPending
              ? t('documents.esign.cancelling', 'Cancelling…')
              : t('documents.esign.cancel', 'Cancel request')}
          </button>
        </div>
      )}

      {remind.isSuccess && remind.data && (
        <p className="signature-card__hint" role="status">
          {t('documents.esign.remindSent', {
            defaultValue: '{{count}} reminder(s) sent.',
            count: remind.data.reminders_sent,
          })}
        </p>
      )}

      {actionError && (
        <p className="signature-card__hint signature-card__hint--error" role="alert">
          {actionError instanceof Error
            ? actionError.message
            : t('common.unknownError', 'Something went wrong. Please try again.')}
        </p>
      )}
    </li>
  );
}

interface SignerRowProps {
  signer: Signer;
  statusLabel: string;
  signedAtLabel?: string;
}

function SignerRow({ signer, statusLabel, signedAtLabel }: SignerRowProps) {
  return (
    <li className="signer-row">
      <span className="signer-row__identity">
        <span className="signer-row__name">{signer.name}</span>
        <span className="signer-row__email">{signer.email}</span>
      </span>
      <span className="signer-row__status">
        <span className={`signature-badge ${SIGNER_STATUS_CLASSES[signer.status]}`}>
          {statusLabel}
        </span>
        {signedAtLabel && <span className="signer-row__signed-at">{signedAtLabel}</span>}
      </span>
    </li>
  );
}

interface CreateSignatureRequestDialogProps {
  documentId: string;
  onClose: () => void;
  onCreated: () => void;
}

interface SignerDraft {
  name: string;
  email: string;
}

const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

function CreateSignatureRequestDialog({
  documentId,
  onClose,
  onCreated,
}: CreateSignatureRequestDialogProps) {
  const { t } = useTranslation();
  const createRequest = useCreateSignatureRequest(documentId);
  const dialogRef = useRef<HTMLDivElement>(null);

  const [signers, setSigners] = useState<SignerDraft[]>([{ name: '', email: '' }]);
  const [subject, setSubject] = useState('');
  const [message, setMessage] = useState('');
  const [touched, setTouched] = useState(false);

  // Focus the first field on mount and trap focus / close on Escape.
  useEffect(() => {
    const focusable = dialogRef.current?.querySelectorAll<HTMLElement>(
      'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
    );
    focusable?.[0]?.focus();
  }, []);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
        return;
      }
      if (e.key === 'Tab') {
        const focusable = dialogRef.current?.querySelectorAll<HTMLElement>(
          'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
        );
        if (!focusable || focusable.length === 0) return;
        const first = focusable[0];
        const last = focusable[focusable.length - 1];
        if (e.shiftKey && document.activeElement === first) {
          e.preventDefault();
          last.focus();
        } else if (!e.shiftKey && document.activeElement === last) {
          e.preventDefault();
          first.focus();
        }
      }
    };
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [onClose]);

  const updateSigner = (index: number, patch: Partial<SignerDraft>) => {
    setSigners((prev) => prev.map((s, i) => (i === index ? { ...s, ...patch } : s)));
  };

  const addSigner = () => setSigners((prev) => [...prev, { name: '', email: '' }]);

  const removeSigner = (index: number) =>
    setSigners((prev) => (prev.length === 1 ? prev : prev.filter((_, i) => i !== index)));

  const validSigners: CreateSigner[] = signers
    .map((s) => ({ name: s.name.trim(), email: s.email.trim() }))
    .filter((s) => s.name.length > 0 && EMAIL_RE.test(s.email))
    .map((s, idx) => ({ name: s.name, email: s.email, order: idx }));

  const canSubmit = validSigners.length > 0 && !createRequest.isPending;

  const handleSubmit = async () => {
    setTouched(true);
    if (validSigners.length === 0) return;
    try {
      await createRequest.mutateAsync({
        signers: validSigners,
        subject: subject.trim() || undefined,
        message: message.trim() || undefined,
      });
      onCreated();
    } catch {
      // surfaced via createRequest.error below
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        ref={dialogRef}
        className="bg-white rounded-lg shadow-xl w-full max-w-lg mx-4 max-h-[90vh] overflow-y-auto"
        role="dialog"
        aria-modal="true"
        aria-labelledby="doc-esign-title"
      >
        <div className="flex items-center justify-between p-6 border-b">
          <h2 id="doc-esign-title" className="text-lg font-semibold text-gray-900">
            {t('documents.esign.dialog.title', 'Send document for signature')}
          </h2>
          <button
            type="button"
            onClick={onClose}
            className="text-gray-400 hover:text-gray-600"
            aria-label={t('common.close', 'Close')}
          >
            <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20" aria-hidden="true">
              <path
                fillRule="evenodd"
                d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z"
                clipRule="evenodd"
              />
            </svg>
          </button>
        </div>

        <div className="p-6 space-y-4">
          <div>
            <p className="text-sm font-medium text-gray-700 mb-2">
              {t('documents.esign.dialog.signers', 'Signers')}
            </p>
            <ul className="space-y-2">
              {signers.map((signer, index) => {
                const emailInvalid =
                  touched && signer.email.trim().length > 0 && !EMAIL_RE.test(signer.email.trim());
                return (
                  <li key={index} className="flex items-start gap-2">
                    <div className="flex-1">
                      <input
                        type="text"
                        value={signer.name}
                        onChange={(e) => updateSigner(index, { name: e.target.value })}
                        placeholder={t('documents.esign.dialog.namePlaceholder', 'Full name')}
                        aria-label={t('documents.esign.dialog.namePlaceholder', 'Full name')}
                        className="w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                      />
                    </div>
                    <div className="flex-1">
                      <input
                        type="email"
                        value={signer.email}
                        onChange={(e) => updateSigner(index, { email: e.target.value })}
                        placeholder={t(
                          'documents.esign.dialog.emailPlaceholder',
                          'email@example.com'
                        )}
                        aria-label={t(
                          'documents.esign.dialog.emailPlaceholder',
                          'email@example.com'
                        )}
                        aria-invalid={emailInvalid}
                        className={`w-full px-3 py-2 text-sm border rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 ${
                          emailInvalid ? 'border-red-400' : 'border-gray-300'
                        }`}
                      />
                    </div>
                    <button
                      type="button"
                      onClick={() => removeSigner(index)}
                      disabled={signers.length === 1}
                      aria-label={t('documents.esign.dialog.removeSigner', 'Remove signer')}
                      className="mt-1 text-gray-400 hover:text-red-600 disabled:opacity-30 disabled:cursor-not-allowed"
                    >
                      <svg
                        className="w-5 h-5"
                        fill="none"
                        stroke="currentColor"
                        strokeWidth="2"
                        viewBox="0 0 24 24"
                        aria-hidden="true"
                      >
                        <path d="M3 6h18M8 6V4a2 2 0 012-2h4a2 2 0 012 2v2m-9 0v14a2 2 0 002 2h6a2 2 0 002-2V6" />
                      </svg>
                    </button>
                  </li>
                );
              })}
            </ul>
            <button
              type="button"
              onClick={addSigner}
              className="mt-2 text-sm font-medium text-blue-600 hover:text-blue-700"
            >
              {t('documents.esign.dialog.addSigner', '+ Add signer')}
            </button>
            {touched && validSigners.length === 0 && (
              <p role="alert" className="mt-1 text-xs text-red-600">
                {t(
                  'documents.esign.dialog.signerRequired',
                  'Add at least one signer with a name and a valid email.'
                )}
              </p>
            )}
          </div>

          <div>
            <label
              htmlFor="doc-esign-subject"
              className="block text-sm font-medium text-gray-700 mb-1"
            >
              {t('documents.esign.dialog.subject', 'Subject (optional)')}
            </label>
            <input
              id="doc-esign-subject"
              type="text"
              value={subject}
              onChange={(e) => setSubject(e.target.value)}
              placeholder={t(
                'documents.esign.dialog.subjectPlaceholder',
                'e.g. Management contract'
              )}
              className="w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
          </div>

          <div>
            <label
              htmlFor="doc-esign-message"
              className="block text-sm font-medium text-gray-700 mb-1"
            >
              {t('documents.esign.dialog.message', 'Message to signers (optional)')}
            </label>
            <textarea
              id="doc-esign-message"
              value={message}
              onChange={(e) => setMessage(e.target.value)}
              rows={3}
              placeholder={t(
                'documents.esign.dialog.messagePlaceholder',
                'Please review and sign the attached document.'
              )}
              className="w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none"
            />
          </div>

          {createRequest.isError && (
            <div role="alert" className="rounded-md bg-red-50 p-3">
              <p className="text-sm text-red-700">
                {createRequest.error instanceof Error
                  ? createRequest.error.message
                  : t('common.unknownError', 'Something went wrong. Please try again.')}
              </p>
            </div>
          )}
        </div>

        <div className="flex items-center justify-end gap-3 px-6 py-4 border-t bg-gray-50 rounded-b-lg">
          <button
            type="button"
            onClick={onClose}
            disabled={createRequest.isPending}
            className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50 disabled:opacity-50"
          >
            {t('common.cancel', 'Cancel')}
          </button>
          <button
            type="button"
            onClick={handleSubmit}
            disabled={!canSubmit}
            className="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded-md disabled:opacity-50 disabled:cursor-not-allowed inline-flex items-center gap-2"
          >
            {createRequest.isPending && (
              <svg
                className="animate-spin h-4 w-4 text-white"
                fill="none"
                viewBox="0 0 24 24"
                aria-hidden="true"
              >
                <circle
                  className="opacity-25"
                  cx="12"
                  cy="12"
                  r="10"
                  stroke="currentColor"
                  strokeWidth="4"
                />
                <path
                  className="opacity-75"
                  fill="currentColor"
                  d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"
                />
              </svg>
            )}
            {createRequest.isPending
              ? t('documents.esign.dialog.sending', 'Sending…')
              : t('documents.esign.dialog.send', 'Send for signature')}
          </button>
        </div>
      </div>
    </div>
  );
}

const panelStyles = `
  .signature-panel__header {
    margin-bottom: 0.75rem;
  }

  .signature-empty {
    font-size: 0.8125rem;
    color: var(--ppt-fg-muted);
    margin: 0;
  }

  .signature-empty--error {
    color: var(--ppt-color-danger);
  }

  .signature-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .signature-card {
    padding: 0.875rem 1rem;
    border: 1px solid var(--ppt-border-default);
    border-radius: 0.5rem;
    background: var(--ppt-bg-surface);
  }

  .signature-card__head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.625rem;
  }

  .signature-card__subject {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--ppt-fg-primary);
  }

  .signature-card__date {
    font-size: 0.75rem;
    color: var(--ppt-fg-muted);
    margin-left: auto;
  }

  .signature-badge {
    display: inline-flex;
    align-items: center;
    padding: 0.0625rem 0.5rem;
    font-size: 0.6875rem;
    font-weight: 600;
    border-radius: 9999px;
  }

  .signer-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
  }

  .signer-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    font-size: 0.8125rem;
  }

  .signer-row__identity {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .signer-row__name {
    font-weight: 500;
    color: var(--ppt-fg-primary);
  }

  .signer-row__email {
    font-size: 0.75rem;
    color: var(--ppt-fg-muted);
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .signer-row__status {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    flex: 0 0 auto;
  }

  .signer-row__signed-at {
    font-size: 0.6875rem;
    color: var(--ppt-fg-muted);
  }

  .signature-card__actions {
    display: flex;
    gap: 0.5rem;
    margin-top: 0.75rem;
  }

  .action-btn--danger {
    color: var(--ppt-color-danger);
    border-color: var(--ppt-border-default);
  }

  .action-btn--danger:hover:not(:disabled) {
    background: var(--ppt-color-danger);
    border-color: var(--ppt-color-danger);
    color: var(--ppt-fg-on-accent);
  }

  .action-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .signature-card__hint {
    margin: 0.5rem 0 0;
    font-size: 0.75rem;
    color: var(--ppt-fg-muted);
  }

  .signature-card__hint--error {
    color: var(--ppt-color-danger);
  }
`;

export default DocumentSignaturePanel;
