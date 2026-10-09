/**
 * Report-a-listing page — abuse-report submit wiring (#code-review
 * reality-web /report page submits locally only).
 *
 * Before the fix, `handleSubmit` just flipped `setSubmitted(true)` — the
 * report was NEVER POSTed to reality-server, so every abuse report was
 * silently dropped. These tests render the REAL page and drive the form end
 * to end, pinning:
 *   1. a valid submit POSTs `/api/v1/reports` with the snake_case wire body
 *      (`listing_id` / `problem_type` / `description`) the server expects,
 *      and only then shows the success screen;
 *   2. a rate-limit (429) surfaces an error alert instead of a fake success;
 *   3. a missing listing reference blocks the POST and surfaces an error;
 *   4. the attachment control collects nothing and the submit body is
 *      attachment-free (#3017) — the page must not imply evidence is sent.
 *
 * On `dev` test (1) fails at the `expect(post).toBeDefined()` assertion
 * because no fetch ever happens — this is the IG3 regression guard. Test (4)
 * is the #3017 regression guard: on `dev` the file input still exists so the
 * `toBeNull()` assertion fails.
 *
 * Only the Header/Footer chrome and next/navigation are mocked; the submit
 * path and `useSubmitReport` are the real thing.
 */
/// <reference types="vitest/globals" />

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const LISTING_ID = '11111111-2222-3333-4444-555555555555';

// Mutable search-params holder — `vi.hoisted` so the hoisted `vi.mock` factory
// can read it. Individual tests swap the value via `setSearchParams`.
const nav = vi.hoisted(() => ({ params: new URLSearchParams() }));
function setSearchParams(params: URLSearchParams) {
  nav.params = params;
}

vi.mock('next/navigation', () => ({
  useSearchParams: () => nav.params,
  useRouter: () => ({ push: vi.fn(), replace: vi.fn(), refresh: vi.fn() }),
  usePathname: () => '/en/report',
  useParams: () => ({ locale: 'en' }),
}));

vi.mock('@/components/ui', () => ({
  Header: () => <header />,
  Footer: () => <footer />,
}));

import ReportPage from './page';

function renderPage() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <ReportPage />
    </QueryClientProvider>
  );
}

// Select first problem card, type a description, accept GDPR.
function fillForm(container: HTMLElement, description = 'This looks like a scam') {
  fireEvent.click(screen.getAllByRole('radio')[0]);
  const textarea = container.querySelector('textarea');
  if (!textarea) throw new Error('description textarea not found');
  fireEvent.change(textarea, { target: { value: description } });
  fireEvent.click(screen.getByRole('checkbox'));
}

describe('ReportPage — abuse-report submit wiring', () => {
  beforeEach(() => {
    // Default: a deep-link `?listing=<uuid>` is present.
    setSearchParams(new URLSearchParams({ listing: LISTING_ID }));
  });
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('POSTs /api/v1/reports with the snake_case body and then shows success', async () => {
    const fetchMock = vi.fn((_url: string, _init?: RequestInit) =>
      Promise.resolve({
        ok: true,
        status: 201,
        json: async () => ({
          report: { id: 'r-1', listing_id: LISTING_ID, status: 'received' },
        }),
      })
    );
    vi.stubGlobal('fetch', fetchMock);

    const { container } = renderPage();
    fillForm(container);
    fireEvent.click(screen.getByRole('button'));

    await waitFor(() => {
      const post = fetchMock.mock.calls.find(([, init]) => init?.method === 'POST');
      expect(post).toBeDefined();
      expect(String(post?.[0])).toMatch(/\/api\/v1\/reports$/);
      const body = JSON.parse(String(post?.[1]?.body));
      expect(body).toEqual({
        listing_id: LISTING_ID,
        problem_type: 'fraudulent_listing',
        description: 'This looks like a scam',
      });
    });

    // Success screen replaces the form once the POST resolves.
    expect(await screen.findByText('Nahlásenie bolo odoslané')).toBeInTheDocument();
  });

  // #3042: the server returns `201 { report: { id } }` but the success card
  // used to ignore it, so the "ID R-…" acknowledgement was aspirational and
  // the reporter had no reference to quote in a follow-up. The success card
  // must now read `report.id` back and render it. On `dev` (before the fix)
  // the id is never surfaced, so the `findByTestId` below times out — this is
  // the IG3 regression guard.
  it('surfaces the created report id on the success card', async () => {
    const REPORT_ID = 'c0ffee00-dead-beef-cafe-0123456789ab';
    const fetchMock = vi.fn((_url: string, _init?: RequestInit) =>
      Promise.resolve({
        ok: true,
        status: 201,
        json: async () => ({
          report: { id: REPORT_ID, listing_id: LISTING_ID, status: 'received' },
        }),
      })
    );
    vi.stubGlobal('fetch', fetchMock);

    const { container } = renderPage();
    fillForm(container);
    fireEvent.click(screen.getByRole('button'));

    const ref = await screen.findByTestId('report-reference-id');
    expect(ref).toHaveTextContent(REPORT_ID);
  });

  it('surfaces an error alert on a 429 rate-limit instead of a fake success', async () => {
    const fetchMock = vi.fn((_url: string, _init?: RequestInit) =>
      Promise.resolve({ ok: false, status: 429, json: async () => ({}) })
    );
    vi.stubGlobal('fetch', fetchMock);

    const { container } = renderPage();
    fillForm(container);
    fireEvent.click(screen.getByRole('button'));

    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent(/Priveľa nahlásení/);
    expect(screen.queryByText('Nahlásenie bolo odoslané')).not.toBeInTheDocument();
  });

  // #3017: the attachment control used to collect File objects that
  // handleSubmit never transmitted, so a reporter saw a success screen while
  // the evidence was silently dropped. Until an upload endpoint exists the
  // control is disabled with a "coming soon" hint and carries no file input,
  // so the page must not imply evidence can be attached and sent.
  it('does not collect attachments (control disabled, no file input, coming-soon hint)', () => {
    const { container } = renderPage();
    // No file input: nothing to collect, nothing to silently drop.
    expect(container.querySelector('input[type="file"]')).toBeNull();
    // A disabled "coming soon" placeholder stands in for the old control.
    const placeholder = screen.getByText('Pridávanie príloh bude čoskoro dostupné');
    expect(placeholder).toBeInTheDocument();
    expect(placeholder).toHaveAttribute('aria-disabled', 'true');
  });

  it('keeps the submit body attachment-free even after the form is filled', async () => {
    const fetchMock = vi.fn((_url: string, _init?: RequestInit) =>
      Promise.resolve({
        ok: true,
        status: 201,
        json: async () => ({ report: { id: 'r-2', listing_id: LISTING_ID, status: 'received' } }),
      })
    );
    vi.stubGlobal('fetch', fetchMock);

    const { container } = renderPage();
    fillForm(container);
    fireEvent.click(screen.getByRole('button'));

    await waitFor(() => {
      const post = fetchMock.mock.calls.find(([, init]) => init?.method === 'POST');
      expect(post).toBeDefined();
      const body = JSON.parse(String(post?.[1]?.body));
      // The POST body carries no `attachments` key — the page does not pretend
      // to submit evidence it cannot upload yet.
      expect(body).not.toHaveProperty('attachments');
    });
  });

  it('blocks the POST and shows an error when no listing reference is present', async () => {
    setSearchParams(new URLSearchParams());
    const fetchMock = vi.fn((_url: string, _init?: RequestInit) =>
      Promise.resolve({ ok: true, status: 201, json: async () => ({ report: {} }) })
    );
    vi.stubGlobal('fetch', fetchMock);

    const { container } = renderPage();
    fillForm(container);
    fireEvent.click(screen.getByRole('button'));

    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent(/Nevedeli sme určiť/);
    expect(fetchMock).not.toHaveBeenCalled();
  });
});
