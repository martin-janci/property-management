/**
 * ChatInterface — delete-session failure path (code-review finding:
 * ppt-web-ui-ai-chat-delete-swallows-err).
 *
 * Regression: `handleDeleteSession` awaited `deleteSession.mutateAsync` inside a
 * try/catch whose `catch` block discarded the error with a comment ("Error
 * handling is managed by mutation") — but nothing in the UI rendered the
 * `deleteSession` error state, so a failed delete produced no feedback at all.
 * The user confirmed the deletion, the request failed, and the conversation
 * silently stayed in the list.
 *
 * The fix renders an error banner bound to `deleteSession.isError`, mirroring
 * the existing `sendMessage.isError` banner. This test drives the real hooks
 * through a recording axios adapter: it confirms the delete, fails the DELETE
 * with a non-retryable 403, and asserts the banner surfaces.
 */
/// <reference types="vitest/globals" />
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type { AxiosAdapter, AxiosResponse } from 'axios';
import { AxiosHeaders } from 'axios';
import type { ReactNode } from 'react';
import { configureApiClient, resetApiClient } from '../../../lib/api';
import { ChatInterface } from './ChatInterface';

const SESSION_ID = 's-1';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, opts?: Record<string, unknown>) => {
      const translations: Record<string, string> = {
        'aiChat.deleteConversation': 'Delete conversation',
        'aiChat.confirmDelete': 'Are you sure you want to delete this conversation?',
        'aiChat.deleteError': 'Failed to delete conversation',
        'aiChat.sendError': 'Failed to send message',
        'common.delete': 'Delete',
        'common.cancel': 'Cancel',
        'common.confirmTitle': 'Are you sure?',
      };
      if (key === 'aiChat.messageCount') return `${opts?.count ?? 0} messages`;
      return translations[key] || key;
    },
  }),
}));

interface RecordedRequest {
  url?: string;
  method?: string;
}

/**
 * Routing adapter: lists one session, fails the DELETE with `deleteStatus`, and
 * records every outbound request.
 */
function routingAdapter(deleteStatus = 403): {
  adapter: AxiosAdapter;
  requests: () => RecordedRequest[];
} {
  const seen: RecordedRequest[] = [];
  const reply = (config: Parameters<AxiosAdapter>[0], data: unknown, status: number) => {
    const response: AxiosResponse = {
      data,
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

  const adapter: AxiosAdapter = (config) => {
    const method = (config.method || 'get').toLowerCase();
    const url = config.url || '';
    AxiosHeaders.from(config.headers); // touch headers like the real client does
    seen.push({ url, method });

    if (method === 'get' && url === '/ai/chat/sessions') {
      return reply(
        config,
        {
          sessions: [
            {
              id: SESSION_ID,
              title: 'Open faults',
              messageCount: 2,
              createdAt: '2026-01-01T00:00:00Z',
              lastMessageAt: '2026-01-01T00:00:00Z',
            },
          ],
        },
        200
      );
    }
    if (method === 'delete' && /\/ai\/chat\/sessions\/.+$/.test(url)) {
      return reply(config, {}, deleteStatus);
    }
    return reply(config, {}, 200);
  };
  return { adapter, requests: () => seen };
}

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: 0 },
      mutations: { retry: false },
    },
  });
  return ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
}

function deleteRequests(requests: RecordedRequest[]): RecordedRequest[] {
  return requests.filter((r) => r.method === 'delete');
}

describe('ChatInterface — failed session delete', () => {
  beforeAll(() => {
    Element.prototype.scrollIntoView = vi.fn();
  });

  beforeEach(() => {
    resetApiClient();
  });

  afterEach(() => {
    resetApiClient();
  });

  it('surfaces a delete-session failure in an error banner', async () => {
    const user = userEvent.setup();
    const instance = configureApiClient({ getToken: () => 'tok' });
    const { adapter, requests } = routingAdapter(403);
    instance.defaults.adapter = adapter;

    render(<ChatInterface />, { wrapper: createWrapper() });

    // The session renders in the sidebar once the list query resolves.
    const deleteButton = await screen.findByLabelText('Delete conversation');
    await user.click(deleteButton);

    // Confirm the deletion in the confirmation dialog.
    const confirmButton = await screen.findByRole('button', { name: 'Delete' });
    await user.click(confirmButton);

    // The DELETE must have gone out...
    await waitFor(() => expect(deleteRequests(requests())).toHaveLength(1));
    expect(deleteRequests(requests())[0].url).toBe(`/ai/chat/sessions/${SESSION_ID}`);

    // ...and its failure must reach the user via the error banner rather than
    // being silently swallowed.
    expect(await screen.findByRole('alert')).toHaveTextContent('Failed to delete conversation');
  });
});
