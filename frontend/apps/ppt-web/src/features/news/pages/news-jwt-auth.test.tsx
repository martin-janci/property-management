/**
 * News pages — JWT auth regression (#2982, follow-up to #2979/#2978).
 *
 * Before the fix, every `features/news/pages/*` API call was a raw
 * `fetch('/api/v1/news/…')` with only `Content-Type: application/json` and NO
 * Authorization header. Those pages sit behind <ProtectedRoute>, so in
 * production every read and mutation went out unauthenticated and 401'd — the
 * same failure mode as #2978, at a larger surface (delete/publish/archive/pin,
 * create, edit, view, reactions, comments).
 *
 * The pages now route through the shared axios client (`getApiClient()`), whose
 * request interceptor stamps `Authorization: Bearer <token>`. These tests seed
 * a token provider on the configured client and capture the outbound request
 * with a recording adapter (the #2979 pattern): they assert the request carries
 * the bearer token, proving it went through the interceptor rather than a raw
 * fetch. They FAIL on the pre-fix raw-fetch version (header undefined) and pass
 * once the pages use getApiClient().
 */
/// <reference types="vitest/globals" />
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import type { AxiosAdapter, AxiosResponse } from 'axios';
import { AxiosHeaders } from 'axios';
import type { ReactNode } from 'react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { OrganizationContext } from '../../../hooks';
import { configureApiClient, resetApiClient } from '../../../lib/api';
import { CreateArticlePage } from './CreateArticlePage';
import { NewsListPage } from './NewsListPage';

const ACCESS_TOKEN = 'news-access-token-123';
const ORG_ID = 'org-1';

// Translations: return the key (or a trivial value) so the pages render.
vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

interface RecordedRequest {
  auth: string | undefined;
  url?: string;
  method?: string;
  params: unknown;
  data: unknown;
}

/** Adapter that records each request and returns a canned response. */
function recordingAdapter(responseData: unknown, status = 200) {
  const seen: RecordedRequest[] = [];
  const adapter: AxiosAdapter = (config) => {
    const headers = AxiosHeaders.from(config.headers);
    seen.push({
      auth: headers.get('Authorization') as string | undefined,
      url: config.url,
      method: config.method,
      params: config.params,
      data: config.data,
    });
    const response: AxiosResponse = {
      data: responseData,
      status,
      statusText: status >= 400 ? 'Error' : 'OK',
      headers: {},
      config,
    };
    if (status >= 400) {
      return Promise.reject(
        Object.assign(new Error(`Request failed with status code ${status}`), {
          isAxiosError: true,
          response,
          config,
        })
      );
    }
    return Promise.resolve(response);
  };
  return { adapter, requests: () => seen };
}

function renderWithProviders(ui: ReactNode) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: 0 }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <OrganizationContext.Provider value={{ organizationId: ORG_ID }}>
        <MemoryRouter>{ui}</MemoryRouter>
      </OrganizationContext.Provider>
    </QueryClientProvider>
  );
}

describe('news pages — route through the JWT-injecting axios client (#2982)', () => {
  beforeEach(() => {
    resetApiClient();
  });

  afterEach(() => {
    resetApiClient();
    vi.restoreAllMocks();
  });

  it('NewsListPage: the article-list GET carries the Authorization header', async () => {
    const instance = configureApiClient({ getToken: () => ACCESS_TOKEN });
    const { adapter, requests } = recordingAdapter([]);
    instance.defaults.adapter = adapter;

    renderWithProviders(<NewsListPage />);

    await waitFor(() => expect(requests().length).toBeGreaterThan(0));

    const listReq = requests().find((r) => r.url === '/news');
    expect(listReq).toBeDefined();
    // Core regression assertion: the request went through the interceptor and
    // carried the bearer token (pre-fix raw fetch sent no Authorization header).
    expect(listReq?.auth).toBe(`Bearer ${ACCESS_TOKEN}`);
    expect(listReq?.method).toBe('get');
    expect(listReq?.params).toMatchObject({ organization_id: ORG_ID });
  });

  it('CreateArticlePage: the create mutation POST carries the Authorization header', async () => {
    const instance = configureApiClient({ getToken: () => ACCESS_TOKEN });
    const { adapter, requests } = recordingAdapter({ id: 'article-1' });
    instance.defaults.adapter = adapter;

    renderWithProviders(<CreateArticlePage />);

    fireEvent.change(screen.getByLabelText(/news\.title/i), {
      target: { value: 'Hello world' },
    });
    fireEvent.change(screen.getByLabelText(/news\.content/i), {
      target: { value: 'Body content' },
    });
    fireEvent.click(screen.getByText('common.save'));

    await waitFor(() => expect(requests().length).toBeGreaterThan(0));

    const createReq = requests().find((r) => r.url === '/news' && r.method === 'post');
    expect(createReq).toBeDefined();
    expect(createReq?.auth).toBe(`Bearer ${ACCESS_TOKEN}`);
    expect(createReq?.params).toMatchObject({ organization_id: ORG_ID });
  });
});
