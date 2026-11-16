/// <reference types="vitest/globals" />
/**
 * Regression test for the e-signature request modal (Epic 84, Story 84.2).
 *
 * The modal must NOT surface the raw thrown `Error.message` (server English or
 * internal DB/transport text) when a signature request fails — sibling
 * components route failures through the shared `describeError` helper so the
 * rendered copy is localized catalogue text. This test pins that behaviour:
 * the raw server message never reaches the DOM; the localized catalogue copy
 * does.
 */

import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { RequestSignatureModal } from './RequestSignatureModal';

const RAW_SERVER_LEAK = 'duplicate key value violates unique constraint "sig_pkey"';

// ─── network boundary: @ppt/api-client esignature hook ──────────────────────
// Force the modal into its error state with a raw, un-localized Error message.
vi.mock('@ppt/api-client', () => ({
  useCreateSignatureRequest: () => ({
    mutateAsync: vi.fn(),
    isPending: false,
    isError: true,
    error: new Error(RAW_SERVER_LEAK),
  }),
}));

// react-i18next: return the key (or the second-arg default string) so we can
// assert which translation key the modal resolved its copy from.
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, second?: unknown) => (typeof second === 'string' ? second : key),
  }),
}));

const parties = [{ name: 'Alice Tenant', email: 'alice@example.com', role: 'tenant' }];

describe('RequestSignatureModal error handling', () => {
  it('shows localized catalogue copy, not the raw server error message', () => {
    render(
      <RequestSignatureModal
        documentId="doc-1"
        parties={parties}
        onSuccess={vi.fn()}
        onClose={vi.fn()}
      />
    );

    const alert = screen.getByRole('alert');

    // The raw server/DB text must never be rendered to the user.
    expect(alert.textContent).not.toContain(RAW_SERVER_LEAK);
    expect(screen.queryByText(RAW_SERVER_LEAK)).toBeNull();

    // The displayed copy is resolved from the localized error catalogue.
    expect(alert.textContent).toContain('errors.catalogue');
  });
});
