/// <reference types="vitest/globals" />
/**
 * UnitForm tests (Story 3.2 — Unit management UI).
 *
 * Covers the create/edit unit form acceptance criteria:
 *  - AC: required designation gates submit
 *  - AC: ownership-share range validation (0–100)
 *  - AC: a valid submit forwards the typed values (decimals as strings)
 *  - edit mode pre-populates the fields from `initial`
 */

import { fireEvent, render, screen } from '@testing-library/react';
import { UnitForm } from './UnitForm';

function renderForm(props: Partial<React.ComponentProps<typeof UnitForm>> = {}) {
  const onSubmit = props.onSubmit ?? vi.fn();
  const onClose = props.onClose ?? vi.fn();
  render(<UnitForm isOpen mode="create" onSubmit={onSubmit} onClose={onClose} {...props} />);
  return { onSubmit, onClose };
}

describe('UnitForm', () => {
  it('keeps submit disabled until a designation is entered', () => {
    renderForm();
    const submit = screen.getByRole('button', { name: 'Add unit' });
    expect(submit).toBeDisabled();

    fireEvent.change(screen.getByLabelText(/designation/i), { target: { value: '3B' } });
    expect(submit).not.toBeDisabled();
  });

  it('rejects an ownership share outside 0–100', () => {
    renderForm();
    fireEvent.change(screen.getByLabelText(/designation/i), { target: { value: '3B' } });
    fireEvent.change(screen.getByLabelText(/share/i), { target: { value: '150' } });

    expect(screen.getByText(/between 0 and 100/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Add unit' })).toBeDisabled();
  });

  it('forwards typed values on a valid submit', () => {
    const { onSubmit } = renderForm();
    fireEvent.change(screen.getByLabelText(/designation/i), { target: { value: '101' } });
    fireEvent.change(screen.getByLabelText(/floor/i), { target: { value: '2' } });
    fireEvent.change(screen.getByLabelText(/type/i), { target: { value: 'commercial' } });
    fireEvent.change(screen.getByLabelText(/size/i), { target: { value: '72.5' } });
    fireEvent.change(screen.getByLabelText(/share/i), { target: { value: '50' } });

    fireEvent.click(screen.getByRole('button', { name: 'Add unit' }));

    expect(onSubmit).toHaveBeenCalledWith(
      expect.objectContaining({
        designation: '101',
        floor: 2,
        unit_type: 'commercial',
        size_sqm: '72.5',
        ownership_share: '50',
        occupancy_status: 'unknown',
      })
    );
  });

  it('pre-populates fields in edit mode', () => {
    renderForm({
      mode: 'edit',
      initial: {
        designation: '12A',
        floor: -1,
        unit_type: 'storage',
        ownership_share: '33.33',
        occupancy_status: 'rented',
      },
    });

    expect(screen.getByLabelText(/designation/i)).toHaveValue('12A');
    expect(screen.getByLabelText(/floor/i)).toHaveValue(-1);
    expect(screen.getByLabelText(/share/i)).toHaveValue(33.33);
    expect(screen.getByRole('button', { name: 'Save changes' })).toBeInTheDocument();
  });
});
