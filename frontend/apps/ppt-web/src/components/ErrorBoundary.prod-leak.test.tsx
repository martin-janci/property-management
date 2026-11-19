/**
 * Regression coverage: the global ErrorBoundary must NOT leak raw error text
 * to end users in a production build.
 *
 * Before the fix, the fallback UI rendered `<code>{error.message}</code>`
 * unconditionally, so any uncaught render error surfaced its raw message —
 * which can carry internal server/DB/stack detail and ships English
 * regardless of locale — directly to the user in production.
 *
 * The fix gates that detail block behind `import.meta.env.DEV` (mirroring
 * `devDetail` in `lib/describeError`), so production shows only the generic,
 * localized copy while development still shows the raw detail for debugging.
 */
/// <reference types="vitest/globals" />

import { render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ErrorBoundary } from './ErrorBoundary';

// A sentinel that stands in for internal detail a real error might carry.
const RAW_SECRET = 'internal db constraint ppt_users_pkey violated at pool#3';
const GENERIC_MESSAGE = 'We apologize for the inconvenience. An unexpected error has occurred.';

/** A child that always throws during render with a detail-bearing message. */
function Boom(): never {
  throw new Error(RAW_SECRET);
}

// React logs caught render errors to console.error; silence it so the
// expected-failure renders don't spam the reporter.
beforeEach(() => {
  vi.spyOn(console, 'error').mockImplementation(() => {});
});
afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllEnvs();
});

describe('ErrorBoundary raw-message leak', () => {
  it('hides the raw error message in a production build', () => {
    vi.stubEnv('DEV', false);

    render(
      <ErrorBoundary>
        <Boom />
      </ErrorBoundary>
    );

    // The generic, localized copy is still shown…
    expect(screen.getByRole('alert')).toBeInTheDocument();
    expect(screen.getByText(GENERIC_MESSAGE)).toBeInTheDocument();
    // …but the raw internal detail is NOT rendered to the user.
    expect(screen.queryByText(RAW_SECRET)).not.toBeInTheDocument();
  });

  it('shows the raw error message in a development build for debugging', () => {
    vi.stubEnv('DEV', true);

    render(
      <ErrorBoundary>
        <Boom />
      </ErrorBoundary>
    );

    expect(screen.getByText(GENERIC_MESSAGE)).toBeInTheDocument();
    expect(screen.getByText(RAW_SECRET)).toBeInTheDocument();
  });
});
