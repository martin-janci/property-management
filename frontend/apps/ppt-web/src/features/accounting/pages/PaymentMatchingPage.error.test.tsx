/// <reference types="vitest/globals" />
/**
 * PaymentMatchingPage query-failure surfacing.
 *
 * The three reads on this screen (`fetchStatements`, `fetchStatementLines`,
 * `fetchLineMatches`) originally discarded `isError`. When the match-review
 * read rejected, the right-hand "Match review" panel rendered NOTHING — no
 * error, no empty state, no spinner — so a failed payment match looked like a
 * no-op (tier1d / code-review-ppt-web-ui-payment-match-silent-fail). The
 * mutations on the same page already surface failures via `isError` +
 * `role="alert"`; these tests assert the queries now do the same.
 *
 * Only the page's own data layer + i18n are mocked; the real page renders so
 * the select-statement → select-line → matches-query → onError → alert path is
 * exercised end-to-end.
 */
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router-dom';
import { PaymentMatchingPage } from './PaymentMatchingPage';

// ── i18n: resolve keys to their bare key so assertions are locale-stable ──
vi.mock('react-i18next', async (importOriginal) => ({
  ...(await importOriginal<typeof import('react-i18next')>()),
  useTranslation: () => ({
    t: (key: string, opts?: { defaultValue?: string }) => opts?.defaultValue ?? key,
    i18n: { language: 'en' },
  }),
}));

// ── data layer: control each read independently ──
const mockFetchStatements = vi.fn();
const mockFetchStatementLines = vi.fn();
const mockFetchLineMatches = vi.fn();

vi.mock('../api/paymentMatching', () => ({
  fetchStatements: (...args: unknown[]) => mockFetchStatements(...args),
  fetchStatementLines: (...args: unknown[]) => mockFetchStatementLines(...args),
  fetchLineMatches: (...args: unknown[]) => mockFetchLineMatches(...args),
  confirmMatch: vi.fn().mockResolvedValue(undefined),
  rejectMatch: vi.fn().mockResolvedValue(undefined),
  uploadStatement: vi.fn().mockResolvedValue(undefined),
}));

const STATEMENT = {
  id: 'stmt-1',
  source_filename: 'july-statement.csv',
  imported_at: '2026-07-01T00:00:00Z',
  account_iban: 'SK1111',
};

const LINE = {
  id: 'line-1',
  amount: '250.00',
  currency: 'EUR',
  booking_date: '2026-07-01T00:00:00Z',
  match_state: 'suggested',
  variable_symbol: '12345',
  counterparty_iban: 'SK9999',
};

function renderPage() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={['/accounting/payment-matching']}>
        <PaymentMatchingPage />
      </MemoryRouter>
    </QueryClientProvider>
  );
}

describe('PaymentMatchingPage query-failure surfacing', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('surfaces an error alert when the match-review read rejects (silent-fail regression)', async () => {
    mockFetchStatements.mockResolvedValue([STATEMENT]);
    mockFetchStatementLines.mockResolvedValue([LINE]);
    mockFetchLineMatches.mockRejectedValue(new Error('match lookup boom'));

    renderPage();

    // Select the statement → lines load.
    await userEvent.click(
      await screen.findByRole('button', { name: /july-statement\.csv/ }, { timeout: 5000 })
    );
    // Select the transaction line → the match-review read fires and rejects.
    await userEvent.click(await screen.findByRole('button', { name: /250\.00/ }));

    await waitFor(() => {
      const alerts = screen.getAllByRole('alert');
      expect(
        alerts.some((el) => el.textContent?.includes('accounting.paymentMatching.loadFailed'))
      ).toBe(true);
    });
  });

  it('surfaces an error alert when the statements read rejects', async () => {
    mockFetchStatements.mockRejectedValue(new Error('statements boom'));

    renderPage();

    await waitFor(() => {
      const alerts = screen.getAllByRole('alert');
      expect(
        alerts.some((el) => el.textContent?.includes('accounting.paymentMatching.loadFailed'))
      ).toBe(true);
    });
  });
});
