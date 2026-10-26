/**
 * Agent / realtor public profile page price-formatting regression test.
 *
 * Reality Portal is a six-language portal (sk/cs/de/en/pl/hu). The agent's
 * listing prices on the public profile must render with the active locale's
 * number convention, not a hardcoded `sk-SK`. Before the fix the page called
 * `listing.price.toLocaleString('sk-SK')`, so a German visitor saw Slovak
 * space-grouping (`720 000`) instead of German dot-grouping (`720.000 €`).
 *
 * This is the follow-up slice to PR #3166 (issue #3175): the same hardcoded
 * `toLocaleString('sk-SK')` price display left behind on several reality-web
 * pages. It mirrors the agency-profile regression test (PR #3166): force the
 * active locale to `de` via next-intl's `useLocale` and assert the price uses
 * German grouping. It fails against the hardcoded-`sk-SK` implementation and
 * passes once the page threads the active locale through
 * `@/lib/format.formatPrice`.
 */

import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import AgentProfilePage from './page';

// Force a non-Slovak active locale. The page reads it via `useLocale`.
vi.mock('next-intl', () => ({
  useLocale: () => 'de',
}));

// Keep the chrome components inert so the test exercises only the page's own
// rendering (no next-intl router / provider context needed).
vi.mock('@/components/ui', () => ({
  Header: () => null,
  Footer: () => null,
}));

describe('AgentProfilePage price formatting', () => {
  it('formats listing prices using the active locale (de), not hardcoded sk-SK', () => {
    render(<AgentProfilePage />);

    // de-DE groups thousands with a dot: 720.000. The old sk-SK output was
    // 720 000 (space grouping) — the space form must NOT appear.
    const price = screen.getByText(/720\.000/);
    expect(price).toBeInTheDocument();
    expect(price.textContent).not.toMatch(/720[\s  ]000/);
  });
});
