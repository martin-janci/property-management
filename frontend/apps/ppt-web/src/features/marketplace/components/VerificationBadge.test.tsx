/**
 * VerificationBadge expiry-logic regression tests.
 *
 * Bug: `isExpiring` was `expiresAt - now < 30 days`, which is ALSO true when the
 * difference is negative (already expired). An already-expired trust signal was
 * therefore rendered as "(Expiring soon)" — as if still valid.
 *
 * Fix: an already-expired badge renders as "(Expired)"; only a not-yet-expired
 * badge inside the 30-day window renders as "(Expiring soon)".
 */
/// <reference types="vitest/globals" />

import { render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { type Badge, VerificationBadge, VerificationStatusBadge } from './VerificationBadge';

const DAY = 24 * 60 * 60 * 1000;
const NOW = new Date('2026-08-21T12:00:00.000Z');

function iso(offsetMs: number): string {
  return new Date(NOW.getTime() + offsetMs).toISOString();
}

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(NOW);
});

afterEach(() => {
  vi.useRealTimers();
});

// The badge label appears twice in the DOM (visible text + the SVG <title>),
// so we read the tooltip from the outer <span>'s `title` attribute directly.
function badgeTitle(container: HTMLElement): string | null {
  return container.querySelector('span[title]')?.getAttribute('title') ?? null;
}

describe('VerificationBadge expiry state', () => {
  it('marks an already-expired badge as (Expired), not (Expiring soon)', () => {
    const badge: Badge = {
      type: 'verified_business',
      awardedAt: iso(-400 * DAY),
      expiresAt: iso(-5 * DAY),
    };
    const { container } = render(<VerificationBadge badge={badge} />);
    expect(badgeTitle(container)).toBe('Verified Business (Expired)');
    expect(badgeTitle(container)).not.toContain('Expiring soon');
  });

  it('marks a not-yet-expired badge inside 30 days as (Expiring soon)', () => {
    const badge: Badge = {
      type: 'verified_business',
      awardedAt: iso(-300 * DAY),
      expiresAt: iso(10 * DAY),
    };
    const { container } = render(<VerificationBadge badge={badge} />);
    expect(badgeTitle(container)).toBe('Verified Business (Expiring soon)');
  });

  it('leaves a far-future badge with no expiry suffix', () => {
    const badge: Badge = {
      type: 'verified_business',
      awardedAt: iso(-1 * DAY),
      expiresAt: iso(200 * DAY),
    };
    const { container } = render(<VerificationBadge badge={badge} />);
    expect(badgeTitle(container)).toBe('Verified Business');
  });

  it('leaves a badge with no expiry date unmarked', () => {
    const badge: Badge = { type: 'insured', awardedAt: iso(-1 * DAY) };
    const { container } = render(<VerificationBadge badge={badge} />);
    expect(badgeTitle(container)).toBe('Insured');
  });
});

describe('VerificationStatusBadge expiry hint', () => {
  it('does not show "(Soon)" for a verification whose expiry date already passed', () => {
    render(
      <VerificationStatusBadge
        verification={{
          id: '1',
          type: 'insurance',
          documentName: 'policy.pdf',
          status: 'verified',
          expiryDate: iso(-3 * DAY),
        }}
      />
    );
    expect(screen.queryByText(/\(Soon\)/)).toBeNull();
  });

  it('shows "(Soon)" for a still-valid verification inside the 30-day window', () => {
    render(
      <VerificationStatusBadge
        verification={{
          id: '2',
          type: 'insurance',
          documentName: 'policy.pdf',
          status: 'verified',
          expiryDate: iso(7 * DAY),
        }}
      />
    );
    expect(screen.getByText(/\(Soon\)/)).toBeInTheDocument();
  });
});
