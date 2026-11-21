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
 *   4. added evidence links ride along in the POST body as `attachments`
 *      (#3017) — the control is now functional (URL links, no file upload),
 *      and an invalid link is rejected inline rather than sent.
 *
 * On `dev` test (1) fails at the `expect(post).toBeDefined()` assertion
 * because no fetch ever happens — this is the IG3 regression guard. The
 * attachment tests are the #3017 regression guard: on `dev` the control is
 * disabled and collects nothing, so the "attachments in body" assertion fails.
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

// Override the global next-intl mock (src/test/setup.tsx) to add `t.rich`,
// which the GDPR consent label uses — the setup mock's `t` has no `.rich`.
// Both `t(key)` and `t.rich(key, …)` echo the key verbatim, so the assertions
// below pin message KEYS (`successTitle`, `errorRateLimit`, …) rather than any
// one locale's copy.
vi.mock('next-intl', () => {
  const t = Object.assign((key: string) => key, { rich: (key: string) => key });
  return {
    useTranslations: () => t,
    useLocale: () => 'en',
    useMessages: () => ({}),
    NextIntlClientProvider: ({ children }: { children: React.ReactNode }) => children,
  };
});

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

// The page now has several buttons (Add-link, per-row Remove, Submit), so a
// bare getByRole('button') is ambiguous. The submit button's label is the
// `submit`/`submitting` message key (echoed verbatim by the mock), so scope by
// name. The Add-link button is `attachmentsAdd`; Remove is `attachmentsRemove`.
const submitBtn = () => screen.getByRole('button', { name: /submit/i });

// Select first problem card, type a description, accept GDPR.
function fillForm(container: HTMLElement, description = 'This looks like a scam') {
  fireEvent.click(screen.getAllByRole('radio')[0]);
  const textarea = container.querySelector('textarea');
  if (!textarea) throw new Error('description textarea not found');
  fireEvent.change(textarea, { target: { value: description } });
  fireEvent.click(screen.getByRole('checkbox'));
}

// Type a link into the attachment URL field and click Add.
function addAttachment(container: HTMLElement, url: string) {
  const input = container.querySelector('#attachment-url') as HTMLInputElement | null;
  if (!input) throw new Error('attachment url input not found');
  fireEvent.change(input, { target: { value: url } });
  fireEvent.click(screen.getByRole('button', { name: 'attachmentsAdd' }));
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
    fireEvent.click(submitBtn());

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
    expect(await screen.findByText('successTitle')).toBeInTheDocument();
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
    fireEvent.click(submitBtn());

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
    fireEvent.click(submitBtn());

    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('errorRateLimit');
    expect(screen.queryByText('successTitle')).not.toBeInTheDocument();
  });

  // #3017: the report form accepts evidence as pre-hosted http(s) links (the
  // public portal has no upload endpoint, but POST /api/v1/reports takes an
  // `attachments: string[]` of URLs). Added links must ride along in the POST
  // body. On `dev` the control is a disabled "coming soon" placeholder with no
  // URL input, so this test fails there — the #3017 regression guard.
  it('includes added evidence links in the submit body as attachments', async () => {
    const fetchMock = vi.fn((_url: string, _init?: RequestInit) =>
      Promise.resolve({
        ok: true,
        status: 201,
        json: async () => ({ report: { id: 'r-a', listing_id: LISTING_ID, status: 'received' } }),
      })
    );
    vi.stubGlobal('fetch', fetchMock);

    const { container } = renderPage();
    addAttachment(container, 'https://example.com/evidence-1.png');
    addAttachment(container, 'https://example.com/evidence-2.pdf');
    fillForm(container);
    fireEvent.click(submitBtn());

    await waitFor(() => {
      const post = fetchMock.mock.calls.find(([, init]) => init?.method === 'POST');
      expect(post).toBeDefined();
      const body = JSON.parse(String(post?.[1]?.body));
      expect(body.attachments).toEqual([
        'https://example.com/evidence-1.png',
        'https://example.com/evidence-2.pdf',
      ]);
    });
  });

  it('rejects an invalid attachment link inline and never sends it', async () => {
    const fetchMock = vi.fn((_url: string, _init?: RequestInit) =>
      Promise.resolve({
        ok: true,
        status: 201,
        json: async () => ({ report: { id: 'r-b', listing_id: LISTING_ID, status: 'received' } }),
      })
    );
    vi.stubGlobal('fetch', fetchMock);

    const { container } = renderPage();
    // A javascript: scheme is the stored-XSS vector the server also rejects.
    addAttachment(container, 'javascript:alert(1)');
    expect(await screen.findByText('attachmentsInvalidUrl')).toBeInTheDocument();

    fillForm(container);
    fireEvent.click(submitBtn());

    await waitFor(() => {
      const post = fetchMock.mock.calls.find(([, init]) => init?.method === 'POST');
      expect(post).toBeDefined();
      const body = JSON.parse(String(post?.[1]?.body));
      // The rejected link was never added, so the body carries no attachments.
      expect(body).not.toHaveProperty('attachments');
    });
  });

  it('omits the attachments key when no link is added', async () => {
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
    fireEvent.click(submitBtn());

    await waitFor(() => {
      const post = fetchMock.mock.calls.find(([, init]) => init?.method === 'POST');
      expect(post).toBeDefined();
      const body = JSON.parse(String(post?.[1]?.body));
      // No evidence links added → no `attachments` key (optional on the wire).
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
    fireEvent.click(submitBtn());

    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('errorNoListing');
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it('maps 404 to the not-found message and other failures to the generic one', async () => {
    for (const [status, re] of [
      [404, 'errorNotFound'],
      [500, 'errorGeneric'],
    ] as const) {
      const fetchMock = vi.fn((_url: string, _init?: RequestInit) =>
        Promise.resolve({ ok: false, status, json: async () => ({}) })
      );
      vi.stubGlobal('fetch', fetchMock);
      const { container, unmount } = renderPage();
      fillForm(container);
      fireEvent.click(submitBtn());
      expect(await screen.findByRole('alert')).toHaveTextContent(re);
      expect(screen.queryByText('successTitle')).not.toBeInTheDocument();
      unmount();
    }
  });

  it('resolves the listing id from a pasted URL and preserves it as [ref: …]', async () => {
    setSearchParams(new URLSearchParams());
    const fetchMock = vi.fn((_url: string, _init?: RequestInit) =>
      Promise.resolve({ ok: true, status: 201, json: async () => ({ report: { id: 'r-3' } }) })
    );
    vi.stubGlobal('fetch', fetchMock);

    const { container } = renderPage();
    const url = `https://rlt.sk/listings/${LISTING_ID}`;
    fireEvent.change(container.querySelector('input[type="text"]') as HTMLInputElement, {
      target: { value: url },
    });
    fillForm(container);
    fireEvent.click(submitBtn());

    await waitFor(() => {
      const post = fetchMock.mock.calls.find(([, init]) => init?.method === 'POST');
      expect(post).toBeDefined();
      const body = JSON.parse(String(post?.[1]?.body));
      expect(body.listing_id).toBe(LISTING_ID);
      expect(body.description).toBe(`This looks like a scam\n\n[ref: ${url}]`);
    });
  });

  it('does not append [ref: …] when the reference is the bare UUID', async () => {
    const _OTHER = 'aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee';
    const fetchMock = vi.fn((_url: string, _init?: RequestInit) =>
      Promise.resolve({ ok: true, status: 201, json: async () => ({ report: { id: 'r-4' } }) })
    );
    vi.stubGlobal('fetch', fetchMock);

    const { container } = renderPage();
    fireEvent.change(container.querySelector('input[type="text"]') as HTMLInputElement, {
      target: { value: LISTING_ID },
    });
    fillForm(container);
    fireEvent.click(submitBtn());
    await waitFor(() => expect(fetchMock).toHaveBeenCalled());
    const body = JSON.parse(String(fetchMock.mock.calls[0][1]?.body));
    expect(body.description).toBe('This looks like a scam');
  });

  it('prefers ?listing over a different UUID in the free-text reference', async () => {
    const OTHER = 'aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee';
    setSearchParams(new URLSearchParams({ listing: OTHER }));
    const fetchMock = vi.fn((_url: string, _init?: RequestInit) =>
      Promise.resolve({ ok: true, status: 201, json: async () => ({ report: { id: 'r-6' } }) })
    );
    vi.stubGlobal('fetch', fetchMock);
    const { container } = renderPage();
    fireEvent.change(container.querySelector('input[type="text"]') as HTMLInputElement, {
      target: { value: LISTING_ID },
    });
    fillForm(container);
    fireEvent.click(submitBtn());
    await waitFor(() => expect(fetchMock).toHaveBeenCalled());
    const body = JSON.parse(String(fetchMock.mock.calls[0][1]?.body));
    expect(body.listing_id).toBe(OTHER);
  });

  it('does not POST while the form is incomplete', () => {
    const fetchMock = vi.fn();
    vi.stubGlobal('fetch', fetchMock);
    const { container } = renderPage();
    // Radio + description but GDPR not accepted.
    fireEvent.click(screen.getAllByRole('radio')[0]);
    fireEvent.change(container.querySelector('textarea') as HTMLTextAreaElement, {
      target: { value: 'x' },
    });
    const form = container.querySelector('form') as HTMLFormElement;
    fireEvent.submit(form);
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it('ignores a second submit event while the first POST is in flight', async () => {
    let resolve: (v: unknown) => void = () => {};
    const fetchMock = vi.fn(
      (_url: string, _init?: RequestInit) =>
        new Promise((r) => {
          resolve = r;
        })
    );
    vi.stubGlobal('fetch', fetchMock);

    const { container } = renderPage();
    fillForm(container);
    const form = container.querySelector('form') as HTMLFormElement;
    fireEvent.submit(form);
    await waitFor(() => expect(submitBtn()).toHaveTextContent('submitting'));
    fireEvent.submit(form);
    expect(fetchMock).toHaveBeenCalledTimes(1);

    resolve({ ok: true, status: 201, json: async () => ({ report: { id: 'r-5' } }) });
    expect(await screen.findByText('successTitle')).toBeInTheDocument();
  });
});
