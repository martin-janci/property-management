/**
 * Wire-contract tests for the hand-written admin transport: request bodies
 * must match the api-server's Deserialize structs.
 */

import { afterEach, describe, expect, it, vi } from 'vitest';
import { registerOAuthClient, suspendAgency } from './api';

function okJson(body: unknown): Response {
  return {
    ok: true,
    status: 200,
    headers: new Headers({ 'content-type': 'application/json' }),
    json: async () => body,
    text: async () => JSON.stringify(body),
  } as Response;
}

function sentBody(): Record<string, unknown> {
  const [, init] = vi.mocked(fetch).mock.calls[0] as [RequestInfo, RequestInit];
  return JSON.parse(String(init.body));
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe('admin api request bodies', () => {
  it('suspendAgency sends the required { reason }', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(okJson(null));
    await suspendAgency('a1', 'Unpaid invoices for three months, contract ended');
    expect(sentBody()).toEqual({ reason: 'Unpaid invoices for three months, contract ended' });
  });

  it('registerOAuthClient sends camelCase fields (RegisterClientRequest)', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(okJson({ clientId: 'c1' }));
    await registerOAuthClient({
      name: 'App',
      redirectUris: ['https://app.example/cb'],
      scopes: ['profile'],
      isConfidential: true,
      rotateRefreshTokens: false,
    });
    expect(sentBody()).toMatchObject({
      name: 'App',
      redirectUris: ['https://app.example/cb'],
      scopes: ['profile'],
      isConfidential: true,
      rotateRefreshTokens: false,
    });
    expect(sentBody()).not.toHaveProperty('redirect_uris');
  });
});
