/**
 * Journal index — placeholder-content gating.
 *
 * The journal has no CMS / reality-server feed yet (screen-map
 * docs/screens/reality/journal.md → endpoints: []). The MOCK_ARTICLES that
 * back the page are design-review scaffolding with fabricated bylines and
 * dates; shipping them on a deployed build presents fake editorial content
 * as if it were real. The page now gates them to local development (same
 * convention as the DevPanel / MSW mocks) and shows an honest "coming soon"
 * state otherwise.
 *
 * These tests run with the default Vitest NODE_ENV ('test'), i.e. the
 * NON-development path, and assert the fabricated records never reach the
 * DOM while the honest empty-state copy does. They fail against the old
 * implementation, which rendered MOCK_ARTICLES unconditionally.
 *
 * next-intl's useTranslations is stubbed to the identity function and
 * useLocale to 'en' by src/test/setup.tsx, so chrome keys render as their own
 * literal and the empty state shows the English copy from `_empty.ts`.
 */

import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { journalEmptyCopy } from './_empty';
import { MOCK_ARTICLES } from './_mock';
import JournalPage from './page';

// Header/Footer pull in the full portal chrome; stub them so this test stays
// focused on the journal body.
vi.mock('@/components/ui', () => ({
  Header: () => <header data-testid="header" />,
  Footer: () => <footer data-testid="footer" />,
}));

describe('JournalPage — placeholder gating (non-development build)', () => {
  it('renders the honest coming-soon empty state instead of fabricated articles', () => {
    render(<JournalPage />);

    // Locale is mocked to 'en', so the empty state shows the English copy.
    const copy = journalEmptyCopy('en');
    expect(screen.getByText(copy.title)).toBeInTheDocument();
    expect(screen.getByText(copy.body)).toBeInTheDocument();
  });

  it('never prints a fabricated byline or article title on a deployed build', () => {
    render(<JournalPage />);

    // None of the invented author names or titles from the placeholder set
    // may appear when the dev-only mock is gated off.
    for (const author of new Set(MOCK_ARTICLES.map((a) => a.author))) {
      expect(screen.queryByText(author)).not.toBeInTheDocument();
    }
    expect(screen.queryByText(/Trh s nehnuteľnosťami v roku 2025/)).not.toBeInTheDocument();
  });

  it('keeps the newsletter signup available even with no articles', () => {
    render(<JournalPage />);
    // newsletterSubmit is the identity-mocked key on the subscribe button.
    expect(screen.getByText('newsletterSubmit')).toBeInTheDocument();
  });
});
