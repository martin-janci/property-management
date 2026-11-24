/// <reference types="vitest/globals" />
/**
 * Manager review of resident person-month submissions (UC-10).
 */
import type { PersonMonthSubmissionWithUnit } from '@ppt/api-client';
import { fireEvent, render, screen } from '@testing-library/react';
import { PendingSubmissionsPanel } from './PendingSubmissionsPanel';

const submission: PersonMonthSubmissionWithUnit = {
  id: 's1',
  unit_id: 'u1',
  unit_designation: '4B',
  year: 2026,
  month: 3,
  count: 3,
  current_count: 2,
  note: 'Newborn',
  status: 'pending',
  submitted_by: 'user-1',
  submitter_email: 'resident@example.com',
  submitted_at: '2026-03-20T10:00:00Z',
};

describe('PendingSubmissionsPanel', () => {
  it('renders nothing without submissions', () => {
    const { container } = render(
      <PendingSubmissionsPanel submissions={[]} onApprove={vi.fn()} onReject={vi.fn()} />
    );
    expect(container.firstChild).toBeNull();
  });

  it('shows the proposal and calls approve / reject with its id', () => {
    const onApprove = vi.fn();
    const onReject = vi.fn();
    render(
      <PendingSubmissionsPanel
        submissions={[submission]}
        onApprove={onApprove}
        onReject={onReject}
      />
    );
    expect(screen.getByText('4B · 03/2026')).toBeTruthy();
    expect(screen.getByText('resident@example.com')).toBeTruthy();
    expect(screen.getByText(/Newborn/)).toBeTruthy();

    const buttons = screen.getAllByRole('button');
    fireEvent.click(buttons[0]);
    fireEvent.click(buttons[1]);
    expect(onApprove).toHaveBeenCalledWith('s1');
    expect(onReject).toHaveBeenCalledWith('s1');
  });
});
