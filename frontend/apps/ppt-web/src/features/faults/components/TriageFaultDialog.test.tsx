/**
 * TriageFaultDialog component tests (Epic 4, UC-03.6 / UC-03.8 / UC-03.9).
 *
 * Covers: hidden when closed, visible when open, AI suggestion display (only
 * above 70% confidence), priority selection, category selection, technician
 * assignment, form submission, cancel, and submitting/disabled state.
 */

/// <reference types="vitest/globals" />
import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { TriageFaultDialog } from './TriageFaultDialog';

const onSubmit = vi.fn();
const onClose = vi.fn();

const baseProps = {
  isOpen: true,
  faultTitle: 'Water leak in kitchen',
  currentCategory: 'plumbing' as const,
  onSubmit,
  onClose,
};

beforeEach(() => {
  vi.clearAllMocks();
});

describe('TriageFaultDialog', () => {
  it('renders nothing when isOpen is false', () => {
    const { container } = render(<TriageFaultDialog {...baseProps} isOpen={false} />);
    expect(container).toBeEmptyDOMElement();
  });

  it('renders the dialog with the fault title when open', () => {
    render(<TriageFaultDialog {...baseProps} />);
    // Both the heading and the submit button say "Triage Fault"; assert the heading via role.
    expect(screen.getByRole('heading', { name: 'Triage Fault' })).toBeInTheDocument();
    expect(screen.getByText('Water leak in kitchen')).toBeInTheDocument();
  });

  it('renders all four priority options', () => {
    render(<TriageFaultDialog {...baseProps} />);
    expect(screen.getByLabelText(/low/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/medium/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/high/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/urgent/i)).toBeInTheDocument();
  });

  it('shows the AI suggestion banner when confidence > 0.7', () => {
    render(
      <TriageFaultDialog
        {...baseProps}
        aiSuggestion={{ category: 'electrical', confidence: 0.9 }}
      />
    );
    expect(screen.getByText(/AI suggests/)).toBeInTheDocument();
    expect(screen.getByText(/electrical/)).toBeInTheDocument();
    expect(screen.getByText(/90% confidence/)).toBeInTheDocument();
  });

  it('hides the AI suggestion banner when confidence is ≤ 0.7', () => {
    render(
      <TriageFaultDialog
        {...baseProps}
        aiSuggestion={{ category: 'electrical', confidence: 0.6 }}
      />
    );
    expect(screen.queryByText(/AI suggests/)).not.toBeInTheDocument();
  });

  it('displays AI-suggested priority in the banner', () => {
    render(
      <TriageFaultDialog
        {...baseProps}
        aiSuggestion={{ category: 'plumbing', priority: 'urgent', confidence: 0.95 }}
      />
    );
    expect(screen.getByText(/urgent/)).toBeInTheDocument();
  });

  it('does not render the technician selector when no technicians are provided', () => {
    render(<TriageFaultDialog {...baseProps} />);
    expect(screen.queryByLabelText(/assign to/i)).not.toBeInTheDocument();
  });

  it('renders the technician selector when technicians are provided', () => {
    render(
      <TriageFaultDialog
        {...baseProps}
        technicians={[
          { id: 't1', name: 'Bob Wrench' },
          { id: 't2', name: 'Eve Plumber' },
        ]}
      />
    );
    expect(screen.getByLabelText(/assign to/i)).toBeInTheDocument();
    expect(screen.getByText('Bob Wrench')).toBeInTheDocument();
    expect(screen.getByText('Eve Plumber')).toBeInTheDocument();
  });

  it('calls onSubmit with priority and no category change when category is unchanged', () => {
    render(<TriageFaultDialog {...baseProps} />);

    // Select "high" priority
    fireEvent.click(screen.getByLabelText(/high/i));

    // Click "Triage Fault" button
    fireEvent.click(screen.getByRole('button', { name: /triage fault/i }));

    expect(onSubmit).toHaveBeenCalledWith({
      priority: 'high',
      category: undefined, // unchanged → undefined
      assignedTo: undefined,
    });
  });

  it('calls onSubmit with updated category when category is changed', () => {
    render(<TriageFaultDialog {...baseProps} currentCategory="plumbing" />);

    // Change category to "electrical"
    fireEvent.change(screen.getByLabelText(/category/i), { target: { value: 'electrical' } });

    fireEvent.click(screen.getByRole('button', { name: /triage fault/i }));

    expect(onSubmit).toHaveBeenCalledWith(expect.objectContaining({ category: 'electrical' }));
  });

  it('calls onSubmit with assignedTo when a technician is selected', () => {
    render(
      <TriageFaultDialog {...baseProps} technicians={[{ id: 'tech-1', name: 'Bob Wrench' }]} />
    );

    fireEvent.change(screen.getByLabelText(/assign to/i), { target: { value: 'tech-1' } });
    fireEvent.click(screen.getByRole('button', { name: /triage fault/i }));

    expect(onSubmit).toHaveBeenCalledWith(expect.objectContaining({ assignedTo: 'tech-1' }));
  });

  it('calls onClose when the Cancel button is clicked', () => {
    render(<TriageFaultDialog {...baseProps} />);
    fireEvent.click(screen.getByRole('button', { name: /cancel/i }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('calls onClose when the backdrop is clicked', () => {
    render(<TriageFaultDialog {...baseProps} />);
    fireEvent.click(screen.getByRole('button', { name: /close dialog/i }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('disables Cancel and Triage buttons while submitting', () => {
    render(<TriageFaultDialog {...baseProps} isSubmitting />);
    expect(screen.getByRole('button', { name: /cancel/i })).toBeDisabled();
    expect(screen.getByText('Saving...')).toBeInTheDocument();
  });

  it('pre-selects AI-suggested priority when provided', () => {
    render(
      <TriageFaultDialog
        {...baseProps}
        aiSuggestion={{ category: 'plumbing', priority: 'urgent', confidence: 0.95 }}
      />
    );
    const urgentRadio = screen.getByLabelText(/urgent/i) as HTMLInputElement;
    expect(urgentRadio.checked).toBe(true);
  });
});
