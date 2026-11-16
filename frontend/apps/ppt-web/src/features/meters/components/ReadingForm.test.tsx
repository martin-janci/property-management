/**
 * ReadingForm component tests.
 *
 * Regression: a blank or non-numeric reading value must NOT be silently
 * coerced to 0 and submitted. An empty/invalid entry is a validation error;
 * a genuine typed `0` is a distinct, valid value.
 */

/// <reference types="vitest/globals" />
import { fireEvent, render, screen } from '@testing-library/react';
import type { Meter } from '../types';
import { ReadingForm } from './ReadingForm';

// Mock useTranslation — return the key so assertions are locale-independent.
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

// Mock the OCR hook — this test exercises the manual-entry path only.
vi.mock('../hooks', () => ({
  useOcrMeterReading: () => ({
    process: vi.fn(),
    correctAndSubmit: vi.fn(),
    isProcessing: false,
    processingError: null,
    reset: vi.fn(),
  }),
}));

function makeMeter(overrides: Partial<Meter> = {}): Meter {
  return {
    id: 'meter-1',
    organizationId: 'org-1',
    buildingId: 'building-1',
    meterType: 'electricity',
    serialNumber: 'SN-123',
    unit: 'kWh',
    isActive: true,
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:00:00Z',
    ...overrides,
  };
}

const VALUE_INPUT = 'spinbutton';
const SUBMIT = 'meters.form.submitReading';
const REQUIRED_ERROR = 'meters.form.errors.valueRequired';

describe('ReadingForm value validation', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('does not submit a blank value silently coerced to 0', () => {
    const onSubmit = vi.fn();
    render(<ReadingForm meter={makeMeter()} onSubmit={onSubmit} onCancel={vi.fn()} />);

    // The field starts empty (no last reading) and is left untouched.
    expect(screen.getByRole(VALUE_INPUT)).toHaveValue(null);

    fireEvent.click(screen.getByRole('button', { name: SUBMIT }));

    expect(onSubmit).not.toHaveBeenCalled();
    expect(screen.getByText(REQUIRED_ERROR)).toBeInTheDocument();
  });

  it('shows a required error when a value is entered then cleared', () => {
    const onSubmit = vi.fn();
    render(<ReadingForm meter={makeMeter()} onSubmit={onSubmit} onCancel={vi.fn()} />);

    const input = screen.getByRole(VALUE_INPUT);
    fireEvent.change(input, { target: { value: '42' } });
    fireEvent.change(input, { target: { value: '' } });

    fireEvent.click(screen.getByRole('button', { name: SUBMIT }));

    expect(onSubmit).not.toHaveBeenCalled();
    expect(screen.getByText(REQUIRED_ERROR)).toBeInTheDocument();
  });

  it('submits a genuine 0 as a valid reading (distinct from empty)', () => {
    const onSubmit = vi.fn();
    render(<ReadingForm meter={makeMeter()} onSubmit={onSubmit} onCancel={vi.fn()} />);

    fireEvent.change(screen.getByRole(VALUE_INPUT), { target: { value: '0' } });
    fireEvent.click(screen.getByRole('button', { name: SUBMIT }));

    expect(onSubmit).toHaveBeenCalledTimes(1);
    expect(onSubmit).toHaveBeenCalledWith(expect.objectContaining({ value: 0 }));
  });

  it('submits a valid numeric reading', () => {
    const onSubmit = vi.fn();
    render(<ReadingForm meter={makeMeter()} onSubmit={onSubmit} onCancel={vi.fn()} />);

    fireEvent.change(screen.getByRole(VALUE_INPUT), { target: { value: '150.5' } });
    fireEvent.click(screen.getByRole('button', { name: SUBMIT }));

    expect(onSubmit).toHaveBeenCalledTimes(1);
    expect(onSubmit).toHaveBeenCalledWith(expect.objectContaining({ value: 150.5 }));
  });
});
