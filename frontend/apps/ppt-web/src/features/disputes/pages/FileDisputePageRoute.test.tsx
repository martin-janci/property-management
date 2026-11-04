/// <reference types="vitest/globals" />
/**
 * FileDisputePageRoute orchestration tests (#1095).
 *
 * The route wrapper loads the caller's units, maps the form values onto the
 * backend `POST /api/v1/disputes` body (category / title / unit + building)
 * and then toasts + navigates.
 *
 * Covered:
 *  (a) success          → create body mapped, success toast, navigate(detail)
 *  (b) lease type       → category `lease_terms`; respondent → respondentIds
 *  (c) no units          → no unit/building sent; inactive units are hidden
 *  (d) create failure   → error toast (Error.message), no navigate
 *  (e) non-Error throw  → error toast falls back to auth.unexpectedError, no navigate
 *  (f) units loading    → spinner, form not rendered yet
 */
import { render, screen, waitFor } from '@testing-library/react';
import { useState } from 'react';
import type { FileDisputeSubmitPayload } from './FileDisputePage';
import { FileDisputePageRoute } from './FileDisputePageRoute';

// ── api-client: create-dispute hook + the caller's units ──
const mockMutateAsync = vi.fn();
const mockUseMyUnits = vi.fn();
vi.mock('@ppt/api-client', () => ({
  useCreateDispute: () => ({ mutateAsync: mockMutateAsync, isPending: false }),
  useMyUnits: () => mockUseMyUnits(),
}));

vi.mock('../../../components/Spinner', () => ({
  Spinner: () => <div>loading units</div>,
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

// ── i18n: echo key (string fallbacks arrive as the 2nd arg and are ignored) ──
vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

/**
 * Stub FileDisputePage with a button that fires onSubmit with the payload
 * provided per-test via `currentPayload`. Lets us drive the wrapper's
 * handleSubmit deterministically without exercising the real form.
 */
let currentPayload: FileDisputeSubmitPayload;
let renderedUnits: Array<{ id: string; label: string }> = [];
vi.mock('./FileDisputePage', () => ({
  FileDisputePage: ({
    onSubmit,
    units,
  }: {
    onSubmit: (p: FileDisputeSubmitPayload) => void | Promise<void>;
    units: Array<{ id: string; label: string }>;
  }) => {
    const [pending, setPending] = useState(false);
    renderedUnits = units;
    return (
      <button
        type="button"
        disabled={pending}
        onClick={async () => {
          setPending(true);
          // The real FileDisputePage catches a rejected onSubmit (the wrapper
          // re-throws on failure so the page can keep its auto-saved draft);
          // mirror that here so a failed-filing test doesn't leak an unhandled
          // rejection.
          try {
            await onSubmit(currentPayload);
          } catch {
            // swallow — the wrapper already surfaced the error toast
          }
          setPending(false);
        }}
      >
        submit
      </button>
    );
  },
}));

const UNIT = {
  association: { isActive: true },
  unit: { id: 'unit-1', buildingId: 'bld-1', designation: 'A-12' },
  building: { name: 'Sunny House', street: 'Main 1' },
};
const INACTIVE_UNIT = {
  association: { isActive: false },
  unit: { id: 'unit-old', buildingId: 'bld-9', designation: 'Z-1' },
  building: { name: null, street: 'Old 9' },
};

function values(
  overrides: Partial<FileDisputeSubmitPayload['values']> = {}
): FileDisputeSubmitPayload {
  return {
    values: {
      type: 'noise',
      subject: 'Loud music at night',
      description: 'Neighbour plays loud music every night after 23:00 for weeks.',
      unitId: 'unit-1',
      respondentId: '',
      ...overrides,
    },
  };
}

async function submit() {
  render(<FileDisputePageRoute />);
  screen.getByRole('button', { name: 'submit' }).click();
}

beforeEach(() => {
  vi.clearAllMocks();
  renderedUnits = [];
  mockUseMyUnits.mockReturnValue({ data: [UNIT, INACTIVE_UNIT], isLoading: false });
  mockMutateAsync.mockResolvedValue({ id: 'dsp-new' });
});

describe('FileDisputePageRoute', () => {
  it('(a) maps the form onto the create body, toasts success and opens the detail', async () => {
    currentPayload = values();
    await submit();
    await waitFor(() => expect(mockNavigate).toHaveBeenCalledWith('/disputes/dsp-new'));
    expect(mockMutateAsync).toHaveBeenCalledWith({
      category: 'noise',
      title: 'Loud music at night',
      description: 'Neighbour plays loud music every night after 23:00 for weeks.',
      unitId: 'unit-1',
      buildingId: 'bld-1',
      respondentIds: [],
    });
    expect(mockShowToast).toHaveBeenCalledWith(
      expect.objectContaining({ type: 'success', title: 'disputes.filedSuccessfully' })
    );
  });

  it('(b) maps the lease type to lease_terms and forwards a respondent', async () => {
    currentPayload = values({ type: 'lease', respondentId: 'user-2' });
    await submit();
    await waitFor(() => expect(mockMutateAsync).toHaveBeenCalled());
    expect(mockMutateAsync.mock.calls[0][0]).toMatchObject({
      category: 'lease_terms',
      respondentIds: ['user-2'],
    });
  });

  it('(c) offers only active units and files without a unit when none is chosen', async () => {
    mockUseMyUnits.mockReturnValue({ data: [], isLoading: false });
    currentPayload = values({ unitId: '' });
    await submit();
    await waitFor(() => expect(mockMutateAsync).toHaveBeenCalled());
    expect(mockMutateAsync.mock.calls[0][0]).toMatchObject({
      unitId: undefined,
      buildingId: undefined,
    });
    expect(renderedUnits).toEqual([]);
  });

  it('(c2) labels active units with building and designation', () => {
    render(<FileDisputePageRoute />);
    expect(renderedUnits).toEqual([
      { id: 'unit-1', buildingId: 'bld-1', label: 'Sunny House – A-12' },
    ]);
  });

  it('(d) create-dispute failure → error toast and no navigate', async () => {
    mockMutateAsync.mockRejectedValue(new Error('boom'));
    currentPayload = values();
    await submit();
    await waitFor(() =>
      expect(mockShowToast).toHaveBeenCalledWith(
        expect.objectContaining({ type: 'error', title: 'disputes.failedToFile', message: 'boom' })
      )
    );
    expect(mockNavigate).not.toHaveBeenCalled();
  });

  it('(e) non-Error rejection → error toast falls back to auth.unexpectedError, no navigate', async () => {
    mockMutateAsync.mockRejectedValue('nope');
    currentPayload = values();
    await submit();
    await waitFor(() =>
      expect(mockShowToast).toHaveBeenCalledWith(
        expect.objectContaining({ type: 'error', message: 'auth.unexpectedError' })
      )
    );
    expect(mockNavigate).not.toHaveBeenCalled();
  });

  it('(f) shows a spinner until the units have loaded', () => {
    mockUseMyUnits.mockReturnValue({ data: undefined, isLoading: true });
    render(<FileDisputePageRoute />);
    expect(screen.getByText('loading units')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'submit' })).not.toBeInTheDocument();
  });
});
