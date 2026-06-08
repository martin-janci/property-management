/// <reference types="vitest/globals" />
/**
 * FileDisputePageRoute orchestration tests (#1095).
 *
 * The route wrapper threads three steps together — create dispute, sequentially
 * upload non-errored evidence, then toast + navigate — and previously had zero
 * coverage. These tests pin the success / partial-fail / filtered / create-fail
 * branches so the behaviour can't silently regress.
 *
 * Covered:
 *  (a) all-success     → success toast + navigate(detail, undefined)
 *  (b) partial-fail    → warning toast (correct count) + navigate(detail, {state:{failedEvidence}})
 *  (c) errored entries → filtered out before upload (status === 'error')
 *  (d) create failure  → error toast (Error.message), no navigate
 *  (e) non-Error throw  → error toast falls back to auth.unexpectedError, no navigate
 */
import { render, screen, waitFor } from '@testing-library/react';
import { useState } from 'react';
import type { FileDisputeSubmitPayload } from './FileDisputePage';
import { FileDisputePageRoute } from './FileDisputePageRoute';

// ── api-client: create-dispute hook + raw evidence upload ──
const mockMutateAsync = vi.fn();
const mockUploadEvidence = vi.fn();
vi.mock('@ppt/api-client', () => ({
  useCreateDispute: () => ({ mutateAsync: mockMutateAsync, isPending: false }),
  uploadEvidence: (...args: unknown[]) => mockUploadEvidence(...args),
}));

// ── auth context: provide an org so the gate passes ──
vi.mock('../../../contexts', () => ({
  useAuth: () => ({ user: { organizationId: 'org-1' } }),
}));

// ── toast + auth gate ──
const mockShowToast = vi.fn();
vi.mock('../../../components', () => ({
  useToast: () => ({ showToast: mockShowToast }),
  AuthRequiredGate: () => <div>auth required</div>,
}));

// ── router navigate ──
const mockNavigate = vi.fn();
vi.mock('react-router-dom', () => ({
  useNavigate: () => mockNavigate,
}));

// ── i18n: echo key, but interpolate {count} so the warning message is assertable ──
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, opts?: unknown) => {
      if (key === 'disputes.evidenceUploadErrorsMsg' && opts && typeof opts === 'object') {
        const count = (opts as { count?: number }).count;
        return `${count} file(s) failed`;
      }
      // String fallbacks come through as the 2nd arg; ignore them, just echo key.
      return key;
    },
  }),
}));

/**
 * Stub FileDisputePage with a button that fires onSubmit with the payload
 * provided per-test via `currentPayload`. Lets us drive the wrapper's
 * handleSubmit deterministically without exercising the real form.
 */
let currentPayload: FileDisputeSubmitPayload;
vi.mock('./FileDisputePage', () => ({
  FileDisputePage: ({
    onSubmit,
  }: {
    onSubmit: (p: FileDisputeSubmitPayload) => void | Promise<void>;
  }) => {
    const [pending, setPending] = useState(false);
    return (
      <button
        type="button"
        disabled={pending}
        onClick={async () => {
          setPending(true);
          await onSubmit(currentPayload);
          setPending(false);
        }}
      >
        submit
      </button>
    );
  },
}));

function makeEvidence(
  id: string,
  status: 'pending' | 'error' = 'pending',
  description = ''
): FileDisputeSubmitPayload['evidence'][number] {
  return {
    id,
    file: new File(['x'], `${id}.jpg`, { type: 'image/jpeg' }),
    description,
    status,
  };
}

function makePayload(evidence: FileDisputeSubmitPayload['evidence']): FileDisputeSubmitPayload {
  return {
    values: {
      type: 'noise',
      subject: 'Loud parties past midnight',
      description: 'Repeated loud music well past the 22:00 quiet hours threshold.',
      unitId: 'unit-1',
      respondentId: '',
    },
    evidence,
  };
}

async function submit() {
  screen.getByRole('button', { name: /submit/i }).click();
}

beforeEach(() => {
  vi.clearAllMocks();
  mockMutateAsync.mockResolvedValue({ id: 'dsp-99' });
  mockUploadEvidence.mockResolvedValue(undefined);
});

describe('FileDisputePageRoute', () => {
  it('(a) all-success → success toast + navigate to detail with no router state', async () => {
    currentPayload = makePayload([makeEvidence('e1'), makeEvidence('e2')]);
    render(<FileDisputePageRoute />);

    await submit();

    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith('/disputes/dsp-99', { state: undefined });
    });
    expect(mockMutateAsync).toHaveBeenCalledTimes(1);
    expect(mockUploadEvidence).toHaveBeenCalledTimes(2);
    expect(mockShowToast).toHaveBeenCalledWith(
      expect.objectContaining({ type: 'success', title: 'disputes.filedSuccessfully' })
    );
  });

  it('(b) partial-fail → warning toast with count + navigate with failedEvidence state', async () => {
    const ok = makeEvidence('ok');
    const bad = makeEvidence('bad');
    currentPayload = makePayload([ok, bad]);
    // Second upload throws.
    mockUploadEvidence.mockResolvedValueOnce(undefined).mockRejectedValueOnce(new Error('boom'));
    render(<FileDisputePageRoute />);

    await submit();

    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith('/disputes/dsp-99', {
        state: { failedEvidence: [bad] },
      });
    });
    expect(mockShowToast).toHaveBeenCalledWith(
      expect.objectContaining({
        type: 'warning',
        title: 'disputes.filedWithEvidenceErrors',
        message: '1 file(s) failed',
      })
    );
  });

  it("(c) evidence with status 'error' is filtered out before upload", async () => {
    const good = makeEvidence('good', 'pending');
    const errored = makeEvidence('errored', 'error');
    currentPayload = makePayload([good, errored]);
    render(<FileDisputePageRoute />);

    await submit();

    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith('/disputes/dsp-99', { state: undefined });
    });
    // Only the non-errored file is uploaded.
    expect(mockUploadEvidence).toHaveBeenCalledTimes(1);
    expect(mockUploadEvidence).toHaveBeenCalledWith('dsp-99', good.file, 'good.jpg');
    expect(mockShowToast).toHaveBeenCalledWith(expect.objectContaining({ type: 'success' }));
  });

  it('(d) create-dispute failure → error toast and no navigate / upload', async () => {
    currentPayload = makePayload([makeEvidence('e1')]);
    mockMutateAsync.mockRejectedValueOnce(new Error('create failed'));
    render(<FileDisputePageRoute />);

    await submit();

    await waitFor(() => {
      expect(mockShowToast).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'error',
          title: 'disputes.failedToFile',
          message: 'create failed',
        })
      );
    });
    expect(mockUploadEvidence).not.toHaveBeenCalled();
    expect(mockNavigate).not.toHaveBeenCalled();
  });

  it('(e) non-Error rejection → error toast falls back to auth.unexpectedError, no navigate', async () => {
    currentPayload = makePayload([makeEvidence('e1')]);
    // A non-Error throw (e.g. a string or rejected non-Error value) must not
    // crash the handler — the catch falls back to the localized
    // `auth.unexpectedError` message instead of `error.message`.
    mockMutateAsync.mockRejectedValueOnce('network down');
    render(<FileDisputePageRoute />);

    await submit();

    await waitFor(() => {
      expect(mockShowToast).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'error',
          title: 'disputes.failedToFile',
          message: 'auth.unexpectedError',
        })
      );
    });
    expect(mockUploadEvidence).not.toHaveBeenCalled();
    expect(mockNavigate).not.toHaveBeenCalled();
  });
});
