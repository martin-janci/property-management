/**
 * FaultTimeline component tests.
 *
 * Covers: empty state, loading state, entry rendering (action labels, internal
 * flag, status/priority change display, notes, attachment and rating entries).
 */

/// <reference types="vitest/globals" />
import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { FaultTimeline, type TimelineEntry } from './FaultTimeline';

function entry(overrides: Partial<TimelineEntry> = {}): TimelineEntry {
  return {
    id: 'e1',
    faultId: 'f1',
    userId: 'u1',
    action: 'created',
    isInternal: false,
    createdAt: '2026-06-01T10:00:00Z',
    userName: 'Alice Manager',
    userEmail: 'alice@example.com',
    ...overrides,
  };
}

describe('FaultTimeline', () => {
  it('shows an empty state message when there are no entries', () => {
    render(<FaultTimeline entries={[]} />);
    expect(screen.getByText('No timeline entries.')).toBeInTheDocument();
  });

  it('shows a loading spinner (not the empty state) while loading', () => {
    render(<FaultTimeline entries={[]} isLoading />);
    // Spinner is a div; no "No timeline entries." text.
    expect(screen.queryByText('No timeline entries.')).not.toBeInTheDocument();
  });

  it('renders the Timeline heading and entry author name', () => {
    render(<FaultTimeline entries={[entry()]} />);
    expect(screen.getByText('Timeline')).toBeInTheDocument();
    expect(screen.getByText('Alice Manager')).toBeInTheDocument();
  });

  it('renders the correct action label for "created"', () => {
    render(<FaultTimeline entries={[entry({ action: 'created' })]} />);
    expect(screen.getByText('Reported fault')).toBeInTheDocument();
  });

  it('renders the correct action label for "triaged"', () => {
    render(<FaultTimeline entries={[entry({ action: 'triaged' })]} />);
    expect(screen.getByText('Triaged fault')).toBeInTheDocument();
  });

  it('renders the correct action label for "resolved"', () => {
    render(<FaultTimeline entries={[entry({ action: 'resolved' })]} />);
    expect(screen.getByText('Resolved fault')).toBeInTheDocument();
  });

  it('renders the correct action label for "comment"', () => {
    render(<FaultTimeline entries={[entry({ action: 'comment' })]} />);
    expect(screen.getByText('Added comment')).toBeInTheDocument();
  });

  it('shows the "Internal" badge only on internal entries', () => {
    render(
      <FaultTimeline
        entries={[entry({ id: 'e1', isInternal: true }), entry({ id: 'e2', isInternal: false })]}
      />
    );
    const badges = screen.getAllByText('Internal');
    expect(badges).toHaveLength(1);
  });

  it('renders old→new values for status_changed entries', () => {
    render(
      <FaultTimeline
        entries={[entry({ action: 'status_changed', oldValue: 'new', newValue: 'triaged' })]}
      />
    );
    expect(screen.getByText('new')).toBeInTheDocument();
    expect(screen.getByText('triaged')).toBeInTheDocument();
    // Arrow separator between old and new values
    expect(screen.getByText('→')).toBeInTheDocument();
  });

  it('renders the note text when present', () => {
    render(<FaultTimeline entries={[entry({ note: 'Checked the pipe and fixed it.' })]} />);
    expect(screen.getByText('Checked the pipe and fixed it.')).toBeInTheDocument();
  });

  it('renders attachment filename for attachment_added entries', () => {
    render(
      <FaultTimeline
        entries={[entry({ action: 'attachment_added', newValue: 'photo_of_leak.jpg' })]}
      />
    );
    expect(screen.getByText(/photo_of_leak\.jpg/)).toBeInTheDocument();
  });

  it('renders one star per rating point for "rated" entries', () => {
    render(<FaultTimeline entries={[entry({ action: 'rated', newValue: '4' })]} />);
    // The component renders '⭐'.repeat(4) = 4 stars
    expect(screen.getByText('⭐⭐⭐⭐')).toBeInTheDocument();
  });

  it('renders multiple entries in the order provided', () => {
    render(
      <FaultTimeline
        entries={[
          entry({ id: 'e1', userName: 'First User', action: 'created' }),
          entry({ id: 'e2', userName: 'Second User', action: 'triaged' }),
        ]}
      />
    );
    const allEntries = screen.getAllByText(/User/);
    expect(allEntries[0]).toHaveTextContent('First User');
    expect(allEntries[1]).toHaveTextContent('Second User');
  });
});
