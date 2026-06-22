/// <reference types="vitest/globals" />
/**
 * Tests for the document-detail e-signature panel (Story 7B.3).
 *
 * The panel wires the Epic 84 `signature_requests` backend into the document
 * detail screen. These tests mock only the `@ppt/api-client` esignature hooks
 * (the network boundary) and assert the contractual behaviour:
 *
 *   1. empty / loading / error states;
 *   2. a request renders its status and each signer's status, sorted by order;
 *   3. "Send reminder" fires the reminder mutation for that request;
 *   4. reminder is disabled once every signer has responded;
 *   5. the create dialog validates that at least one signer has a valid email
 *      before allowing submit.
 */

import { fireEvent, render, screen, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { DocumentSignaturePanel } from './DocumentSignaturePanel';

// ─── network boundary: @ppt/api-client esignature hooks ─────────────────────
const useSignatureRequestsMock = vi.fn();
const createMutateAsync = vi.fn();
const remindMutate = vi.fn();
const cancelMutate = vi.fn();

vi.mock('@ppt/api-client', () => ({
  useSignatureRequests: (documentId: string) => useSignatureRequestsMock(documentId),
  useCreateSignatureRequest: () => ({
    mutateAsync: createMutateAsync,
    isPending: false,
    isError: false,
    error: null,
  }),
  useSendSignatureReminder: () => ({
    mutate: remindMutate,
    isPending: false,
    isSuccess: false,
    data: undefined,
    error: null,
  }),
  useCancelSignatureRequest: () => ({
    mutate: cancelMutate,
    isPending: false,
    error: null,
  }),
}));

// react-i18next: honour the second-arg default string and interpolate {{x}}.
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (_key: string, second?: unknown) => {
      if (typeof second === 'string') return second;
      if (second && typeof second === 'object') {
        const { defaultValue, ...vars } = second as Record<string, unknown>;
        let out = String(defaultValue ?? _key);
        for (const [k, v] of Object.entries(vars)) {
          out = out.replace(new RegExp(`{{${k}}}`, 'g'), String(v));
        }
        return out;
      }
      return _key;
    },
  }),
}));

function makeSigner(over: Record<string, unknown> = {}) {
  return {
    email: 'a@example.com',
    name: 'Alice',
    order: 0,
    status: 'pending',
    ...over,
  };
}

function makeRequest(over: Record<string, unknown> = {}) {
  return {
    id: 'req-1',
    document_id: 'doc-1',
    organization_id: 'org-1',
    status: 'pending',
    subject: 'Contract',
    signers: [makeSigner()],
    created_by: 'u-1',
    created_at: '2026-06-01T10:00:00Z',
    updated_at: '2026-06-01T10:00:00Z',
    ...over,
  };
}

beforeEach(() => {
  vi.clearAllMocks();
});

describe('DocumentSignaturePanel', () => {
  it('shows the empty state when there are no requests', () => {
    useSignatureRequestsMock.mockReturnValue({
      data: { signature_requests: [], total: 0 },
      isLoading: false,
      error: null,
      refetch: vi.fn(),
    });
    render(<DocumentSignaturePanel documentId="doc-1" />);
    expect(screen.getByText(/has not been sent for signature/i)).toBeInTheDocument();
  });

  it('shows the error state', () => {
    useSignatureRequestsMock.mockReturnValue({
      data: undefined,
      isLoading: false,
      error: new Error('boom'),
      refetch: vi.fn(),
    });
    render(<DocumentSignaturePanel documentId="doc-1" />);
    expect(screen.getByText(/failed to load signature requests/i)).toBeInTheDocument();
  });

  it('renders request status and signers sorted by order', () => {
    useSignatureRequestsMock.mockReturnValue({
      data: {
        signature_requests: [
          makeRequest({
            signers: [
              makeSigner({
                name: 'Bob',
                email: 'b@example.com',
                order: 1,
                status: 'signed',
                signed_at: '2026-06-02T09:00:00Z',
              }),
              makeSigner({ name: 'Alice', email: 'a@example.com', order: 0, status: 'sent' }),
            ],
          }),
        ],
        total: 1,
      },
      isLoading: false,
      error: null,
      refetch: vi.fn(),
    });
    render(<DocumentSignaturePanel documentId="doc-1" />);

    expect(screen.getByText('Pending signature')).toBeInTheDocument();
    const names = screen.getAllByText(/^(Alice|Bob)$/).map((n) => n.textContent);
    // Alice (order 0) renders before Bob (order 1).
    expect(names).toEqual(['Alice', 'Bob']);
    expect(screen.getByText('Signed')).toBeInTheDocument();
    expect(screen.getByText('Sent')).toBeInTheDocument();
  });

  it('fires the reminder mutation for an open request with pending signers', () => {
    useSignatureRequestsMock.mockReturnValue({
      data: { signature_requests: [makeRequest()], total: 1 },
      isLoading: false,
      error: null,
      refetch: vi.fn(),
    });
    render(<DocumentSignaturePanel documentId="doc-1" />);

    const remindBtn = screen.getByRole('button', { name: /send reminder/i });
    expect(remindBtn).toBeEnabled();
    fireEvent.click(remindBtn);
    expect(remindMutate).toHaveBeenCalledWith(
      { id: 'req-1' },
      expect.objectContaining({ onSuccess: expect.any(Function) })
    );
  });

  it('disables the reminder once every signer has responded', () => {
    useSignatureRequestsMock.mockReturnValue({
      data: {
        signature_requests: [
          makeRequest({
            status: 'in_progress',
            signers: [makeSigner({ status: 'signed' })],
          }),
        ],
        total: 1,
      },
      isLoading: false,
      error: null,
      refetch: vi.fn(),
    });
    render(<DocumentSignaturePanel documentId="doc-1" />);
    expect(screen.getByRole('button', { name: /send reminder/i })).toBeDisabled();
  });

  it('hides remind/cancel actions for a completed request', () => {
    useSignatureRequestsMock.mockReturnValue({
      data: {
        signature_requests: [
          makeRequest({ status: 'completed', signers: [makeSigner({ status: 'signed' })] }),
        ],
        total: 1,
      },
      isLoading: false,
      error: null,
      refetch: vi.fn(),
    });
    render(<DocumentSignaturePanel documentId="doc-1" />);
    expect(screen.queryByRole('button', { name: /send reminder/i })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /cancel request/i })).not.toBeInTheDocument();
  });

  it('blocks submit in the dialog until a valid signer is entered', () => {
    useSignatureRequestsMock.mockReturnValue({
      data: { signature_requests: [], total: 0 },
      isLoading: false,
      error: null,
      refetch: vi.fn(),
    });
    render(<DocumentSignaturePanel documentId="doc-1" />);

    fireEvent.click(screen.getByRole('button', { name: /send for signature/i }));
    const dialog = screen.getByRole('dialog');

    // Submit button inside the dialog footer; disabled with no valid signer.
    const submit = within(dialog).getByRole('button', { name: /send for signature/i });
    expect(submit).toBeDisabled();

    fireEvent.change(within(dialog).getByLabelText('Full name'), { target: { value: 'Carol' } });
    fireEvent.change(within(dialog).getByLabelText('email@example.com'), {
      target: { value: 'carol@example.com' },
    });
    expect(submit).toBeEnabled();
  });
});
