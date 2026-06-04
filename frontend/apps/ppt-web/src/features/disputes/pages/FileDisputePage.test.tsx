/// <reference types="vitest/globals" />
/**
 * FileDisputePage form tests (Epic 80, Story 80.2).
 *
 * Regression coverage for the dispute-filing surface acceptance criteria:
 *  - AC-1: dispute-type radio-card grid renders all 6 categories + is required
 *  - AC-3: subject (min 5) + description (min 30) validation, char counter
 *  - AC-1/AC-3: required unit selector
 *  - AC-5: valid submit forwards { values, evidence } to onSubmit; the submit
 *          button is disabled (shows "Filing…") while isSubmitting is true
 *  - navigation: Cancel / Back route to /disputes
 */

import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { type DisputeFormValues, FileDisputePage } from './FileDisputePage';

// jsdom lacks object-URL support used by the embedded EvidenceUploader.
URL.createObjectURL = vi.fn(() => 'blob:mock-url');
URL.revokeObjectURL = vi.fn();

const mockNavigate = vi.fn();
vi.mock('react-router-dom', async (importOriginal) => {
  const actual = await importOriginal<typeof import('react-router-dom')>();
  return { ...actual, useNavigate: () => mockNavigate };
});

const UNITS = [
  { id: 'unit-1', label: 'Unit 1A' },
  { id: 'unit-2', label: 'Unit 2B' },
];

function renderPage(props: Partial<React.ComponentProps<typeof FileDisputePage>> = {}) {
  const onSubmit = props.onSubmit ?? vi.fn();
  render(
    <MemoryRouter>
      <FileDisputePage units={UNITS} onSubmit={onSubmit} {...props} />
    </MemoryRouter>
  );
  return { onSubmit };
}

/** Fill the three required fields with valid values. */
function fillValidForm() {
  fireEvent.click(screen.getByRole('radio', { name: /noise/i }));
  fireEvent.change(screen.getByLabelText(/^unit/i), { target: { value: 'unit-1' } });
  fireEvent.change(screen.getByLabelText(/^subject/i), {
    target: { value: 'Loud parties past midnight' },
  });
  fireEvent.change(screen.getByLabelText(/^description/i), {
    target: { value: 'Repeated loud music and shouting well past the 22:00 quiet hours.' },
  });
}

describe('FileDisputePage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ── AC-1: dispute type selector ──
  it('renders all six dispute-type radio cards', () => {
    renderPage();
    for (const label of [
      /noise/i,
      /property damage/i,
      /payment \/ fees/i,
      /lease terms/i,
      /maintenance/i,
      /^other$/i,
    ]) {
      expect(screen.getByRole('radio', { name: label })).toBeInTheDocument();
    }
  });

  it('marks the type radiogroup as required', () => {
    renderPage();
    expect(screen.getByRole('radiogroup', { name: /dispute type/i })).toHaveAttribute(
      'aria-required',
      'true'
    );
  });

  it('blocks submit and shows the type error when no type is chosen', async () => {
    const { onSubmit } = renderPage();

    fireEvent.click(screen.getByRole('button', { name: /file dispute/i }));

    await waitFor(() => {
      expect(screen.getByText(/please select a dispute type/i)).toBeInTheDocument();
    });
    expect(onSubmit).not.toHaveBeenCalled();
  });

  // ── AC-3: subject + description validation ──
  it('rejects a too-short subject and description', async () => {
    const { onSubmit } = renderPage();

    fireEvent.click(screen.getByRole('radio', { name: /noise/i }));
    fireEvent.change(screen.getByLabelText(/^unit/i), { target: { value: 'unit-1' } });
    fireEvent.change(screen.getByLabelText(/^subject/i), { target: { value: 'hi' } });
    fireEvent.change(screen.getByLabelText(/^description/i), { target: { value: 'too short' } });

    fireEvent.click(screen.getByRole('button', { name: /file dispute/i }));

    await waitFor(() => {
      expect(screen.getByText(/subject must be at least 5 characters/i)).toBeInTheDocument();
    });
    expect(screen.getByText(/description must be at least 30 characters/i)).toBeInTheDocument();
    expect(onSubmit).not.toHaveBeenCalled();
  });

  it('requires a unit to be selected', async () => {
    const { onSubmit } = renderPage();

    fireEvent.click(screen.getByRole('radio', { name: /noise/i }));
    fireEvent.change(screen.getByLabelText(/^subject/i), {
      target: { value: 'A valid subject line' },
    });
    fireEvent.change(screen.getByLabelText(/^description/i), {
      target: { value: 'A sufficiently long description that clears the thirty-char minimum.' },
    });

    fireEvent.click(screen.getByRole('button', { name: /file dispute/i }));

    await waitFor(() => {
      expect(screen.getByText(/please select a unit/i)).toBeInTheDocument();
    });
    expect(onSubmit).not.toHaveBeenCalled();
  });

  it('updates the description character counter as the user types', () => {
    renderPage();
    fireEvent.change(screen.getByLabelText(/^description/i), {
      target: { value: 'hello world' },
    });
    expect(screen.getByText('11 / 5000')).toBeInTheDocument();
  });

  // ── AC-5: submit ──
  it('forwards the validated values and evidence array to onSubmit', async () => {
    const { onSubmit } = renderPage();

    fillValidForm();
    fireEvent.click(screen.getByRole('button', { name: /file dispute/i }));

    await waitFor(() => {
      expect(onSubmit).toHaveBeenCalledTimes(1);
    });
    const payload = onSubmit.mock.calls[0][0] as {
      values: DisputeFormValues;
      evidence: unknown[];
    };
    expect(payload.values).toMatchObject({
      type: 'noise',
      unitId: 'unit-1',
      subject: 'Loud parties past midnight',
    });
    expect(payload.values.description.length).toBeGreaterThanOrEqual(30);
    expect(Array.isArray(payload.evidence)).toBe(true);
  });

  it('disables the submit button and shows a filing spinner while submitting', () => {
    renderPage({ isSubmitting: true });
    const button = screen.getByRole('button', { name: /filing/i });
    expect(button).toBeDisabled();
  });

  // ── Navigation ──
  it('navigates back to the disputes list from Cancel', () => {
    renderPage();
    fireEvent.click(screen.getByRole('button', { name: /^cancel$/i }));
    expect(mockNavigate).toHaveBeenCalledWith('/disputes');
  });

  it('navigates back to the disputes list from the Back link', () => {
    renderPage();
    fireEvent.click(screen.getByRole('button', { name: /back to disputes/i }));
    expect(mockNavigate).toHaveBeenCalledWith('/disputes');
  });
});
