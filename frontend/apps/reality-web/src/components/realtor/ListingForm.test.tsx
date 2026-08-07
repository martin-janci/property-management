/**
 * ListingForm validation tests.
 *
 * Regression for the bug where numeric fields (area / rooms) were submitted
 * via `area ? Number(area) : undefined`, which silently forwarded `NaN` for
 * non-numeric input and let negative values pass through. The form must now
 * reject non-numeric / negative area & rooms with an inline error and never
 * call onSubmit with a `NaN` or negative measure.
 */

import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ListingForm } from './ListingForm';

function fillRequiredFields() {
  fireEvent.change(screen.getByLabelText(/Title/i), { target: { value: 'Nice flat' } });
  fireEvent.change(screen.getByLabelText(/Description/i), {
    target: { value: 'A lovely place to live' },
  });
  fireEvent.change(screen.getByLabelText(/City/i), { target: { value: 'Bratislava' } });
  fireEvent.change(screen.getByLabelText(/Price/i), { target: { value: '120000' } });
}

function submit() {
  fireEvent.click(screen.getByRole('button', { name: /save/i }));
}

describe('ListingForm — numeric field validation', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('rejects a negative area and does not submit', () => {
    const onSubmit = vi.fn();
    render(<ListingForm submitLabel="Save" onSubmit={onSubmit} />);

    fillRequiredFields();
    fireEvent.change(screen.getByLabelText(/Area/i), { target: { value: '-5' } });
    submit();

    expect(screen.getByText(/Area must not be negative/i)).toBeInTheDocument();
    expect(onSubmit).not.toHaveBeenCalled();
  });

  it('rejects a negative room count and does not submit', () => {
    const onSubmit = vi.fn();
    render(<ListingForm submitLabel="Save" onSubmit={onSubmit} />);

    fillRequiredFields();
    fireEvent.change(screen.getByLabelText(/Rooms/i), { target: { value: '-2' } });
    submit();

    expect(screen.getByText(/Rooms must not be negative/i)).toBeInTheDocument();
    expect(onSubmit).not.toHaveBeenCalled();
  });

  it('never forwards NaN for a non-numeric area value', () => {
    const onSubmit = vi.fn();
    render(<ListingForm submitLabel="Save" onSubmit={onSubmit} />);

    fillRequiredFields();
    // Simulate a value `Number()` would coerce to NaN reaching the field.
    fireEvent.change(screen.getByLabelText(/Area/i), { target: { value: 'not-a-number' } });
    submit();

    // Either it is rejected (error shown, no submit) or the field was
    // sanitized to empty — in both cases NaN must never reach onSubmit.
    if (onSubmit.mock.calls.length > 0) {
      const draft = onSubmit.mock.calls[0][0];
      expect(draft.area === undefined || Number.isFinite(draft.area)).toBe(true);
    } else {
      expect(onSubmit).not.toHaveBeenCalled();
    }
  });

  it('submits valid numeric area & rooms as finite numbers (no NaN)', () => {
    const onSubmit = vi.fn();
    render(<ListingForm submitLabel="Save" onSubmit={onSubmit} />);

    fillRequiredFields();
    fireEvent.change(screen.getByLabelText(/Area/i), { target: { value: '85' } });
    fireEvent.change(screen.getByLabelText(/Rooms/i), { target: { value: '3' } });
    submit();

    expect(onSubmit).toHaveBeenCalledTimes(1);
    const draft = onSubmit.mock.calls[0][0];
    expect(draft.area).toBe(85);
    expect(draft.rooms).toBe(3);
    expect(Number.isNaN(draft.area)).toBe(false);
    expect(Number.isNaN(draft.rooms)).toBe(false);
  });

  it('omits empty optional numeric fields (undefined, not NaN)', () => {
    const onSubmit = vi.fn();
    render(<ListingForm submitLabel="Save" onSubmit={onSubmit} />);

    fillRequiredFields();
    submit();

    expect(onSubmit).toHaveBeenCalledTimes(1);
    const draft = onSubmit.mock.calls[0][0];
    expect(draft.area).toBeUndefined();
    expect(draft.rooms).toBeUndefined();
  });
});
