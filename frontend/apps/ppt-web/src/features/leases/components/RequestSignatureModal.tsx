/**
 * RequestSignatureModal — select signers and send an e-signature request.
 * Epic 84, Story 84.2.
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { CreateSigner, SignatureRequestStatus } from '@ppt/api-client';
import { useCreateSignatureRequest } from '@ppt/api-client';

interface Party {
  name: string;
  email: string;
  role: string;
}

interface RequestSignatureModalProps {
  /** The document ID to attach the signature request to */
  documentId: string;
  /** Pre-populated parties involved (e.g. tenant + landlord) */
  parties: Party[];
  onSuccess: (newStatus: SignatureRequestStatus) => void;
  onClose: () => void;
}

export function RequestSignatureModal({
  documentId,
  parties,
  onSuccess,
  onClose,
}: RequestSignatureModalProps) {
  const { t } = useTranslation();
  const createRequest = useCreateSignatureRequest(documentId);

  const [selectedEmails, setSelectedEmails] = useState<Set<string>>(
    new Set(parties.map((p) => p.email))
  );
  const [subject, setSubject] = useState('');
  const [message, setMessage] = useState('');

  const toggleEmail = (email: string) => {
    setSelectedEmails((prev) => {
      const next = new Set(prev);
      if (next.has(email)) {
        next.delete(email);
      } else {
        next.add(email);
      }
      return next;
    });
  };

  const handleSend = async () => {
    const signers: CreateSigner[] = parties
      .filter((p) => selectedEmails.has(p.email))
      .map((p, idx) => ({
        email: p.email,
        name: p.name,
        order: idx,
      }));

    if (signers.length === 0) {
      return;
    }

    try {
      await createRequest.mutateAsync({
        signers,
        subject: subject.trim() || undefined,
        message: message.trim() || undefined,
      });
      onSuccess('pending');
    } catch {
      // error displayed via createRequest.error
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
      role="dialog"
      aria-modal="true"
      aria-labelledby="esign-modal-title"
    >
      <div className="bg-white rounded-lg shadow-xl w-full max-w-md mx-4">
        {/* Header */}
        <div className="flex items-center justify-between p-6 border-b">
          <h2 id="esign-modal-title" className="text-lg font-semibold text-gray-900">
            {t('leases.esign.modal.title', 'Request Signature')}
          </h2>
          <button
            type="button"
            onClick={onClose}
            className="text-gray-400 hover:text-gray-600"
            aria-label={t('common.close', 'Close')}
          >
            <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
              <path
                fillRule="evenodd"
                d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z"
                clipRule="evenodd"
              />
            </svg>
          </button>
        </div>

        {/* Body */}
        <div className="p-6 space-y-4">
          {/* Signer selection */}
          <div>
            <p className="text-sm font-medium text-gray-700 mb-2">
              {t('leases.esign.modal.selectSigners', 'Select signers')}
            </p>
            <ul className="space-y-2">
              {parties.map((party) => (
                <li key={party.email}>
                  <label className="flex items-center gap-3 cursor-pointer">
                    <input
                      type="checkbox"
                      checked={selectedEmails.has(party.email)}
                      onChange={() => toggleEmail(party.email)}
                      className="h-4 w-4 rounded border-gray-300 text-blue-600 focus:ring-blue-500"
                    />
                    <span className="flex-1">
                      <span className="text-sm font-medium text-gray-900">{party.name}</span>
                      <span className="ml-2 text-xs text-gray-500">({party.role})</span>
                      <br />
                      <span className="text-xs text-gray-500">{party.email}</span>
                    </span>
                  </label>
                </li>
              ))}
            </ul>
          </div>

          {/* Subject */}
          <div>
            <label htmlFor="esign-subject" className="block text-sm font-medium text-gray-700 mb-1">
              {t('leases.esign.modal.subject', 'Subject (optional)')}
            </label>
            <input
              id="esign-subject"
              type="text"
              value={subject}
              onChange={(e) => setSubject(e.target.value)}
              placeholder={t('leases.esign.modal.subjectPlaceholder', 'e.g. Lease Agreement Signing')}
              className="w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
          </div>

          {/* Message */}
          <div>
            <label htmlFor="esign-message" className="block text-sm font-medium text-gray-700 mb-1">
              {t('leases.esign.modal.message', 'Message to signers (optional)')}
            </label>
            <textarea
              id="esign-message"
              value={message}
              onChange={(e) => setMessage(e.target.value)}
              rows={3}
              placeholder={t(
                'leases.esign.modal.messagePlaceholder',
                'Please review and sign the attached lease agreement.'
              )}
              className="w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none"
            />
          </div>

          {/* Error */}
          {createRequest.isError && (
            <div className="rounded-md bg-red-50 p-3">
              <p className="text-sm text-red-700">
                {createRequest.error instanceof Error
                  ? createRequest.error.message
                  : t('common.unknownError', 'An error occurred. Please try again.')}
              </p>
            </div>
          )}
        </div>

        {/* Footer */}
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
            onClick={handleSend}
            disabled={createRequest.isPending || selectedEmails.size === 0}
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
              ? t('leases.esign.modal.sending', 'Sending…')
              : t('leases.esign.modal.send', 'Send for Signature')}
          </button>
        </div>
      </div>
    </div>
  );
}
