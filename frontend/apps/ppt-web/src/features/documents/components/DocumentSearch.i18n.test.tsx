/// <reference types="vitest/globals" />
/**
 * DocumentSearch i18n + info-leak regression tests.
 *
 * The search error used to render the raw API/network error text behind a
 * hardcoded English prefix ("Error searching documents: <raw message>").
 * That both broke i18n (untranslated in non-English locales) and leaked
 * backend/network internals into the UI. The error now renders a generic,
 * translated message and never interpolates `error.message`. These tests
 * lock in both behaviours.
 */

import { render, screen } from '@testing-library/react';
import { DocumentSearch } from './DocumentSearch';

const useDocumentSearchMock = vi.fn();

vi.mock('@ppt/api-client', () => ({
  DOCUMENT_CATEGORIES: [],
  useDocumentSearch: (...args: unknown[]) => useDocumentSearchMock(...args),
}));

describe('DocumentSearch error rendering', () => {
  const RAW_ERROR = 'Request failed with status code 500 at https://internal-api.ppt/documents';

  beforeEach(() => {
    useDocumentSearchMock.mockReset();
  });

  it('renders a generic translated error and never leaks the raw error text', () => {
    useDocumentSearchMock.mockReturnValue({
      data: undefined,
      isLoading: false,
      error: new Error(RAW_ERROR),
    });

    render(<DocumentSearch organizationId="org-1" />);

    // Generic translated copy resolves from the English bundle.
    expect(
      screen.getByText('Something went wrong while searching documents. Please try again.')
    ).toBeInTheDocument();

    // The raw API/network message must never reach the DOM.
    expect(screen.queryByText(RAW_ERROR)).not.toBeInTheDocument();
    expect(screen.queryByText(/internal-api/)).not.toBeInTheDocument();
    expect(screen.queryByText(/500/)).not.toBeInTheDocument();

    // The bare i18n key must never leak into the UI either.
    expect(screen.queryByText('documents.search.error')).not.toBeInTheDocument();
  });

  it('shows no error banner when the search succeeds', () => {
    useDocumentSearchMock.mockReturnValue({
      data: { results: [], total: 0, took_ms: 3 },
      isLoading: false,
      error: null,
    });

    render(<DocumentSearch organizationId="org-1" />);

    expect(
      screen.queryByText('Something went wrong while searching documents. Please try again.')
    ).not.toBeInTheDocument();
  });
});
