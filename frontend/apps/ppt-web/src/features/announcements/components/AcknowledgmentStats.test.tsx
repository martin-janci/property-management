/**
 * AcknowledgmentStats tests (gap-6-2 — Story 6.2 web viewing & acknowledgment).
 *
 * Verifies: read/acknowledged percentage math, pending-count pluralization,
 * and the divide-by-zero guard when no users are targeted.
 */
/// <reference types="vitest/globals" />

import type { AcknowledgmentStats as AcknowledgmentStatsType } from '@ppt/api-client';
import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { AcknowledgmentStats } from './AcknowledgmentStats';

function stats(overrides: Partial<AcknowledgmentStatsType> = {}): AcknowledgmentStatsType {
  return {
    announcementId: 'ann-1',
    totalTargeted: 10,
    readCount: 6,
    acknowledgedCount: 2,
    pendingCount: 4,
    ...overrides,
  };
}

describe('AcknowledgmentStats', () => {
  it('renders read and acknowledged counts against the targeted total', () => {
    render(<AcknowledgmentStats stats={stats()} />);

    expect(screen.getByText('6')).toBeInTheDocument();
    expect(screen.getByText('2')).toBeInTheDocument();
    // Both columns reference the same /10 denominator.
    expect(screen.getAllByText('/ 10')).toHaveLength(2);
  });

  it('computes whole-number percentages', () => {
    render(<AcknowledgmentStats stats={stats({ readCount: 6, acknowledgedCount: 2 })} />);

    expect(screen.getByText('Read (60%)')).toBeInTheDocument();
    expect(screen.getByText('Acknowledged (20%)')).toBeInTheDocument();
  });

  it('guards against divide-by-zero when no users are targeted', () => {
    render(
      <AcknowledgmentStats
        stats={stats({ totalTargeted: 0, readCount: 0, acknowledgedCount: 0, pendingCount: 0 })}
      />
    );

    expect(screen.getByText('Read (0%)')).toBeInTheDocument();
    expect(screen.getByText('Acknowledged (0%)')).toBeInTheDocument();
  });

  it('shows the pending banner with singular wording for one pending user', () => {
    render(<AcknowledgmentStats stats={stats({ pendingCount: 1 })} />);

    expect(screen.getByText('1')).toBeInTheDocument();
    expect(screen.getByText(/user haven't read yet/)).toBeInTheDocument();
  });

  it('shows the pending banner with plural wording for multiple pending users', () => {
    render(<AcknowledgmentStats stats={stats({ pendingCount: 3 })} />);

    expect(screen.getByText(/users haven't read yet/)).toBeInTheDocument();
  });

  it('hides the pending banner when nobody is pending', () => {
    render(<AcknowledgmentStats stats={stats({ pendingCount: 0 })} />);

    expect(screen.queryByText(/haven't read yet/)).not.toBeInTheDocument();
  });

  it('forwards a custom className to the root element', () => {
    const { container } = render(<AcknowledgmentStats stats={stats()} className="custom-x" />);

    expect(container.firstChild).toHaveClass('custom-x');
  });
});
