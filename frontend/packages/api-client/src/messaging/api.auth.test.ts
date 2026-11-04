/**
 * Messaging requests read the bearer token at request time (round-3 audit
 * #19: `GET /messages/threads/{id}` went out with no Authorization header when
 * the client was created before the token was available, e.g. on a deep link).
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { clearTokenProvider, setTokenProvider } from '../auth/token-provider';
import { createMessagingApi } from './api';

function okJson(body: unknown): Response {
  return new Response(JSON.stringify(body), { status: 200 });
}

function lastHeaders(): Headers {
  const calls = vi.mocked(fetch).mock.calls;
  const init = calls[calls.length - 1]?.[1] as RequestInit | undefined;
  return new Headers(init?.headers);
}

beforeEach(() => {
  vi.stubGlobal(
    'fetch',
    vi.fn(async () => okJson({ thread: {}, messages: [] }))
  );
});

afterEach(() => {
  clearTokenProvider();
  vi.unstubAllGlobals();
});

describe('messaging api auth', () => {
  it('adds the current token when the client was built without one', async () => {
    const api = createMessagingApi({ baseUrl: '' });
    setTokenProvider(() => 'late-token');
    await api.getThread('t1');
    expect(lastHeaders().get('Authorization')).toBe('Bearer late-token');
  });

  it('prefers the live token over a stale one captured at construction', async () => {
    const api = createMessagingApi({ baseUrl: '', accessToken: 'stale' });
    setTokenProvider(() => 'fresh');
    await api.listThreads();
    expect(lastHeaders().get('Authorization')).toBe('Bearer fresh');
  });

  it('falls back to the configured token when no provider is registered', async () => {
    const api = createMessagingApi({ baseUrl: '', accessToken: 'explicit' });
    await api.listThreads();
    expect(lastHeaders().get('Authorization')).toBe('Bearer explicit');
  });
});
