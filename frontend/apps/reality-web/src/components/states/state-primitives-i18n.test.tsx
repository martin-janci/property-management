/**
 * Regression guard: the shared state primitives must source their default
 * user-facing copy from the next-intl `states.*` catalog, not from hardcoded
 * English literals.
 *
 * The next-intl mock in `src/test/setup.tsx` echoes the translation key, so
 * asserting on the key (e.g. `errorTitle`) proves the string is wired through
 * `useTranslations('states')` rather than falling back to a hardcoded default.
 * Before the i18n fix these assertions failed because the components rendered
 * literal English ("Something went wrong", "Loading…", …).
 */

import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { ErrorState } from './ErrorState';
import { LoadingSkeleton } from './LoadingSkeleton';

describe('shared state primitives are i18n-wired', () => {
  it('ErrorState default copy comes from the states.* catalog', () => {
    render(<ErrorState onRetry={() => {}} />);
    expect(screen.getByText('errorTitle')).toBeInTheDocument();
    expect(screen.getByText('errorDescription')).toBeInTheDocument();
    expect(screen.getByText('retry')).toBeInTheDocument();
  });

  it('ErrorState still honours explicit prop overrides', () => {
    render(<ErrorState title="Boom" description="Details" retryLabel="Again" onRetry={() => {}} />);
    expect(screen.getByText('Boom')).toBeInTheDocument();
    expect(screen.getByText('Details')).toBeInTheDocument();
    expect(screen.getByText('Again')).toBeInTheDocument();
  });

  it('LoadingSkeleton default accessible label comes from the states.* catalog', () => {
    render(<LoadingSkeleton />);
    expect(screen.getByLabelText('loading')).toBeInTheDocument();
  });

  it('LoadingSkeleton honours an explicit label', () => {
    render(<LoadingSkeleton label="Fetching results" />);
    expect(screen.getByLabelText('Fetching results')).toBeInTheDocument();
  });
});
