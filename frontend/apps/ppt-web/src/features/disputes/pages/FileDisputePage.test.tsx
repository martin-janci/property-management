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

import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
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

const RESPONDENTS = [
  { id: 'res-1', name: 'Jane Doe' },
  { id: 'res-2', name: 'John Smith' },
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

/** Build a File with a forced size (jsdom derives size from blob parts). */
function makeFile(name: string, type: string, sizeBytes = 1024): File {
  const file = new File(['x'], name, { type });
  Object.defineProperty(file, 'size', { value: sizeBytes });
  return file;
}

/** Drop files onto the embedded EvidenceUploader file input. */
function attachEvidence(files: File[]) {
  const input = document.querySelector('input[type="file"]') as HTMLInputElement;
  Object.defineProperty(input, 'files', { value: files, configurable: true });
  fireEvent.change(input);
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

  it('rejects a subject longer than 200 characters', async () => {
    const { onSubmit } = renderPage();

    fireEvent.click(screen.getByRole('radio', { name: /noise/i }));
    fireEvent.change(screen.getByLabelText(/^unit/i), { target: { value: 'unit-1' } });
    // maxLength on the input is a UI guard; the zod schema is the source of
    // truth, so drive the change event past the cap directly.
    fireEvent.change(screen.getByLabelText(/^subject/i), {
      target: { value: 'x'.repeat(201) },
    });
    fireEvent.change(screen.getByLabelText(/^description/i), {
      target: { value: 'A sufficiently long description that clears the thirty-char minimum.' },
    });

    fireEvent.click(screen.getByRole('button', { name: /file dispute/i }));

    await waitFor(() => {
      expect(screen.getByText(/subject must be at most 200 characters/i)).toBeInTheDocument();
    });
    expect(onSubmit).not.toHaveBeenCalled();
  });

  // ── AC-2 ↔ AC-5: evidence flows through the filing surface ──
  it('renders the optional evidence section with the uploader drop zone', () => {
    renderPage();
    expect(screen.getByRole('heading', { name: /evidence \(optional\)/i })).toBeInTheDocument();
    expect(
      screen.getByRole('button', { name: /drop evidence files here or click to browse/i })
    ).toBeInTheDocument();
  });

  it('forwards attached evidence files in the onSubmit payload', async () => {
    const { onSubmit } = renderPage();

    fillValidForm();
    attachEvidence([
      makeFile('photo.jpg', 'image/jpeg'),
      makeFile('report.pdf', 'application/pdf'),
    ]);

    fireEvent.click(screen.getByRole('button', { name: /file dispute/i }));

    await waitFor(() => {
      expect(onSubmit).toHaveBeenCalledTimes(1);
    });
    const payload = onSubmit.mock.calls[0][0] as {
      values: DisputeFormValues;
      evidence: Array<{ file: File; status: string }>;
    };
    expect(payload.evidence).toHaveLength(2);
    expect(payload.evidence.map((e) => e.file.name)).toEqual(['photo.jpg', 'report.pdf']);
    expect(payload.evidence.every((e) => e.status === 'pending')).toBe(true);
  });

  it('keeps an invalid evidence file in the payload tagged as error', async () => {
    const { onSubmit } = renderPage();

    fillValidForm();
    // Unsupported type — the uploader keeps it queued with status 'error' so the
    // route wrapper can filter it out before upload.
    attachEvidence([makeFile('virus.exe', 'application/x-msdownload')]);

    fireEvent.click(screen.getByRole('button', { name: /file dispute/i }));

    await waitFor(() => {
      expect(onSubmit).toHaveBeenCalledTimes(1);
    });
    const payload = onSubmit.mock.calls[0][0] as {
      evidence: Array<{ status: string }>;
    };
    expect(payload.evidence).toHaveLength(1);
    expect(payload.evidence[0].status).toBe('error');
  });

  it('submits with an empty evidence array when no files are attached', async () => {
    const { onSubmit } = renderPage();

    fillValidForm();
    fireEvent.click(screen.getByRole('button', { name: /file dispute/i }));

    await waitFor(() => {
      expect(onSubmit).toHaveBeenCalledTimes(1);
    });
    const payload = onSubmit.mock.calls[0][0] as { evidence: unknown[] };
    expect(payload.evidence).toEqual([]);
  });

  // ── Step 2 · parties: optional respondent selector ──
  it('hides the other-party selector when no respondents are provided', () => {
    renderPage();
    expect(screen.queryByLabelText(/other party/i)).not.toBeInTheDocument();
  });

  it('renders and forwards the optional other-party (respondent) selection', async () => {
    const { onSubmit } = renderPage({ respondents: RESPONDENTS });

    const respondent = screen.getByLabelText(/other party/i);
    expect(respondent).toBeInTheDocument();

    fillValidForm();
    fireEvent.change(respondent, { target: { value: 'res-2' } });
    fireEvent.click(screen.getByRole('button', { name: /file dispute/i }));

    await waitFor(() => {
      expect(onSubmit).toHaveBeenCalledTimes(1);
    });
    const payload = onSubmit.mock.calls[0][0] as { values: DisputeFormValues };
    expect(payload.values.respondentId).toBe('res-2');
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

// ── AC-4: draft auto-save ──
describe('FileDisputePage · draft auto-save', () => {
  const DRAFT_KEY = 'ppt-dispute-filing-draft';

  beforeEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.runOnlyPendingTimers();
    vi.useRealTimers();
  });

  it('persists typed values to localStorage after the debounce window', () => {
    renderPage();

    fireEvent.change(screen.getByLabelText(/^subject/i), {
      target: { value: 'Drafted subject line' },
    });

    // Nothing persisted until the debounce flushes.
    expect(localStorage.getItem(DRAFT_KEY)).toBeNull();
    act(() => {
      vi.advanceTimersByTime(800);
    });

    const stored = JSON.parse(localStorage.getItem(DRAFT_KEY) as string);
    expect(stored.values.subject).toBe('Drafted subject line');
  });

  it('shows the auto-save indicator once a draft has been saved', () => {
    renderPage();
    fireEvent.change(screen.getByLabelText(/^subject/i), {
      target: { value: 'Something worth saving' },
    });
    act(() => {
      vi.advanceTimersByTime(800);
    });

    expect(screen.getByText(/draft saved/i)).toBeInTheDocument();
  });

  it('restores a previously-saved draft into the form fields on mount', () => {
    localStorage.setItem(
      DRAFT_KEY,
      JSON.stringify({
        values: { type: 'noise', subject: 'Recovered subject', description: '', unitId: '' },
        savedAt: Date.now(),
      })
    );

    renderPage();

    expect(screen.getByLabelText(/^subject/i)).toHaveValue('Recovered subject');
    expect(screen.getByText(/restored your saved draft/i)).toBeInTheDocument();
  });

  it('clears the stored draft after a successful submit', async () => {
    const onSubmit = vi.fn().mockResolvedValue(undefined);
    renderPage({ onSubmit });

    fireEvent.click(screen.getByRole('radio', { name: /noise/i }));
    fireEvent.change(screen.getByLabelText(/^unit/i), { target: { value: 'unit-1' } });
    fireEvent.change(screen.getByLabelText(/^subject/i), {
      target: { value: 'Loud parties past midnight' },
    });
    fireEvent.change(screen.getByLabelText(/^description/i), {
      target: { value: 'Repeated loud music and shouting well past the 22:00 quiet hours.' },
    });
    act(() => {
      vi.advanceTimersByTime(800);
    });
    expect(localStorage.getItem(DRAFT_KEY)).not.toBeNull();

    fireEvent.click(screen.getByRole('button', { name: /file dispute/i }));

    await act(async () => {
      await vi.runOnlyPendingTimersAsync();
    });

    expect(onSubmit).toHaveBeenCalledTimes(1);
    expect(localStorage.getItem(DRAFT_KEY)).toBeNull();
  });

  it('keeps the stored draft when the submit handler rejects', async () => {
    const onSubmit = vi.fn().mockRejectedValue(new Error('filing failed'));
    renderPage({ onSubmit });

    fireEvent.click(screen.getByRole('radio', { name: /noise/i }));
    fireEvent.change(screen.getByLabelText(/^unit/i), { target: { value: 'unit-1' } });
    fireEvent.change(screen.getByLabelText(/^subject/i), {
      target: { value: 'Loud parties past midnight' },
    });
    fireEvent.change(screen.getByLabelText(/^description/i), {
      target: { value: 'Repeated loud music and shouting well past the 22:00 quiet hours.' },
    });
    act(() => {
      vi.advanceTimersByTime(800);
    });

    fireEvent.click(screen.getByRole('button', { name: /file dispute/i }));

    await act(async () => {
      await vi.runOnlyPendingTimersAsync();
    });

    expect(onSubmit).toHaveBeenCalledTimes(1);
    expect(localStorage.getItem(DRAFT_KEY)).not.toBeNull();
  });
});
