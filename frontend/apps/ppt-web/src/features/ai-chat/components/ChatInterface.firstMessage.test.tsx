/**
 * ChatInterface — first-message path (code-review finding:
 * ai-chat-first-message-settimeout-race).
 *
 * Regression: when no session is active, `handleSendMessage` created the
 * session, called `setCurrentSessionId`, then deferred the actual send inside a
 * hardcoded `setTimeout(() => sendMessage.mutateAsync(...), 100)`. That deferred
 * call executed OUTSIDE the surrounding try/catch, so a failed first send became
 * an unhandled promise rejection instead of being surfaced to the user.
 *
 * This is the primary entry point of the feature — AiChatPage mounts
 * ChatInterface with no initialSessionId, so EVERY first interaction (typed or
 * via a suggested question) took this branch.
 *
 * The fix threads the newly-created session id through explicitly and awaits the
 * send inside the same try/catch, with no timer. These tests drive the real
 * hooks through a recording axios adapter. The failure test is the regression
 * guard: on the pre-fix version the rejected first send escapes as an unhandled
 * rejection; after the fix it is caught and surfaced.
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

const NEW_SESSION_ID = 's-new';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const translations: Record<string, string> = {
        'aiChat.inputPlaceholder': 'Type your message...',
        'aiChat.messageInput': 'Message input',
        'aiChat.send': 'Send message',
        'aiChat.sendError': 'Failed to send message',
      };
      return translations[key] || key;
    },
  }),
}));

interface RecordedRequest {
  url?: string;
  method?: string;
  data: unknown;
}

/**
 * Routing adapter: answers the handful of AI-chat endpoints ChatInterface hits
 * during a first message and records every outbound request. `sendStatus`
 * controls the message POST status so the failure path can be exercised.
 */
function routingAdapter(sendStatus = 200): {
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
    seen.push({ url, method, data: config.data });

    if (method === 'get' && url === '/ai/chat/sessions') {
      return reply(config, { sessions: [] }, 200);
    }
    if (method === 'post' && url === '/ai/chat/sessions') {
      return reply(config, { id: NEW_SESSION_ID, title: 'hi', context: {} }, 200);
    }
    if (method === 'get' && /\/ai\/chat\/sessions\/.+\/messages$/.test(url)) {
      return reply(config, { messages: [] }, 200);
    }
    if (method === 'post' && /\/ai\/chat\/sessions\/.+\/messages$/.test(url)) {
      return reply(
        config,
        {
          userMessage: { id: 'u1', content: 'hi' },
          assistantMessage: { id: 'a1', content: 'hello' },
        },
        sendStatus
      );
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

/** The message POST requests recorded by the adapter. */
function messagePosts(requests: RecordedRequest[]): RecordedRequest[] {
  return requests.filter((r) => r.method === 'post' && /\/messages$/.test(r.url || ''));
}

describe('ChatInterface — first message with no active session', () => {
  beforeAll(() => {
    // jsdom does not implement scrollIntoView; the message-list effect calls it
    // once messages render after a successful send.
    Element.prototype.scrollIntoView = vi.fn();
  });

  beforeEach(() => {
    resetApiClient();
  });

  afterEach(() => {
    resetApiClient();
  });

  it('sends the typed first message to the newly-created session', async () => {
    const user = userEvent.setup();
    const instance = configureApiClient({ getToken: () => 'tok' });
    const { adapter, requests } = routingAdapter();
    instance.defaults.adapter = adapter;

    render(<ChatInterface showSidebar={false} />, { wrapper: createWrapper() });

    await user.type(screen.getByLabelText('Message input'), 'What faults are open?{Enter}');

    await waitFor(() => expect(messagePosts(requests())).toHaveLength(1));

    expect(messagePosts(requests())[0].url).toBe(`/ai/chat/sessions/${NEW_SESSION_ID}/messages`);
    // The create-session POST must precede the message POST.
    const postOrder = requests()
      .filter((r) => r.method === 'post')
      .map((r) => r.url);
    expect(postOrder).toEqual([
      '/ai/chat/sessions',
      `/ai/chat/sessions/${NEW_SESSION_ID}/messages`,
    ]);
  });

  it('routes a suggested-question first message to the new session too', async () => {
    const user = userEvent.setup();
    const instance = configureApiClient({ getToken: () => 'tok' });
    const { adapter, requests } = routingAdapter();
    instance.defaults.adapter = adapter;

    render(<ChatInterface showSidebar={false} />, { wrapper: createWrapper() });

    // Suggested questions render on the welcome screen (no session yet).
    const [firstSuggestion] = await screen.findAllByRole('button', {
      name: /aiChat\.suggested\./,
    });
    await user.click(firstSuggestion);

    await waitFor(() => expect(messagePosts(requests())).toHaveLength(1));
    expect(messagePosts(requests())[0].url).toBe(`/ai/chat/sessions/${NEW_SESSION_ID}/messages`);
  });

  it('surfaces a failed first send without leaking an unhandled rejection', async () => {
    const unhandled: unknown[] = [];
    const onUnhandled = (reason: unknown) => {
      unhandled.push(reason);
    };
    process.on('unhandledRejection', onUnhandled);
    try {
      const user = userEvent.setup();
      const instance = configureApiClient({ getToken: () => 'tok' });
      // 403 (non-retryable) so the error surfaces immediately.
      const { adapter, requests } = routingAdapter(403);
      instance.defaults.adapter = adapter;

      render(<ChatInterface showSidebar={false} />, { wrapper: createWrapper() });

      await user.type(screen.getByLabelText('Message input'), 'Will this fail?{Enter}');

      await waitFor(() => expect(messagePosts(requests())).toHaveLength(1));
      expect(messagePosts(requests())[0].url).toBe(`/ai/chat/sessions/${NEW_SESSION_ID}/messages`);

      // The failure must reach the user via the error banner...
      expect(await screen.findByRole('alert')).toHaveTextContent('Failed to send message');

      // ...and must NOT escape the handler as an unhandled promise rejection.
      // Pre-fix, the deferred setTimeout send rejected outside handleSendMessage's
      // try/catch; give any escaped rejection a generous window to surface.
      await new Promise((resolve) => setTimeout(resolve, 150));
      expect(unhandled).toEqual([]);
    } finally {
      process.off('unhandledRejection', onUnhandled);
    }
  });
});
