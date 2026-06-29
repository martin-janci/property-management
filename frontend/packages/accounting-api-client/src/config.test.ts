/**
 * Unit tests for @ppt/accounting-api-client config resolution.
 *
 * Covers EPIC-ACC-17 (resource-server wiring): base-URL precedence, trailing
 * slash normalisation, and the Bearer-token header assembly that every
 * authenticated accounting-server call relies on.
 *
 * NOTE (FE-Scaffold): test bodies are written but NOT run in Phase 1 — Verify
 * (Phase 2) runs them. They are environment-agnostic (no jsdom): `window` is
 * stubbed on globalThis and torn down per test.
 */

import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import {
  apiUrl,
  authHeaders,
  configureAccountingApi,
  DEFAULT_ACCOUNTING_API_URL,
  getApiBase,
  getAuthToken,
  resetAccountingApi,
} from './config';

// Reset the SDK singleton between tests for isolation.
function resetConfig() {
  resetAccountingApi();
}

describe('getApiBase', () => {
  const originalEnv = process.env.NEXT_PUBLIC_ACCOUNTING_API_URL;

  beforeEach(() => {
    delete (globalThis as { window?: unknown }).window;
    delete process.env.NEXT_PUBLIC_ACCOUNTING_API_URL;
  });

  afterEach(() => {
    delete (globalThis as { window?: unknown }).window;
    if (originalEnv === undefined) delete process.env.NEXT_PUBLIC_ACCOUNTING_API_URL;
    else process.env.NEXT_PUBLIC_ACCOUNTING_API_URL = originalEnv;
  });

  it('falls back to the localhost:8082 dev default (CONTRACT §8)', () => {
    resetConfig();
    expect(getApiBase()).toBe(DEFAULT_ACCOUNTING_API_URL);
    expect(getApiBase()).toBe('http://localhost:8082');
  });

  it('prefers an explicit configureAccountingApi baseUrl over env', () => {
    process.env.NEXT_PUBLIC_ACCOUNTING_API_URL = 'https://env.example';
    configureAccountingApi({ baseUrl: 'https://explicit.example' });
    expect(getApiBase()).toBe('https://explicit.example');
  });

  it('strips trailing slashes from the resolved base URL', () => {
    configureAccountingApi({ baseUrl: 'https://acc.example/' });
    expect(getApiBase()).toBe('https://acc.example');
  });

  it('reads the build-time env var when no explicit baseUrl is set', () => {
    resetConfig();
    process.env.NEXT_PUBLIC_ACCOUNTING_API_URL = 'https://api.acc.dev';
    expect(getApiBase()).toBe('https://api.acc.dev');
  });

  it('prefers window.__ENV__ over the build-time env var', () => {
    resetConfig();
    process.env.NEXT_PUBLIC_ACCOUNTING_API_URL = 'https://baked.example';
    (globalThis as { window?: unknown }).window = {
      __ENV__: { NEXT_PUBLIC_ACCOUNTING_API_URL: 'https://runtime.example' },
    };
    expect(getApiBase()).toBe('https://runtime.example');
  });
});

describe('apiUrl', () => {
  beforeEach(() => {
    delete (globalThis as { window?: unknown }).window;
    configureAccountingApi({ baseUrl: 'http://localhost:8082' });
  });

  it('prefixes paths with /api/v1', () => {
    expect(apiUrl('/invoices')).toBe('http://localhost:8082/api/v1/invoices');
  });

  it('tolerates a path without a leading slash', () => {
    expect(apiUrl('contacts')).toBe('http://localhost:8082/api/v1/contacts');
  });
});

describe('auth token + headers', () => {
  it('returns null when no token provider yields a token', () => {
    configureAccountingApi({ getToken: () => null });
    expect(getAuthToken()).toBeNull();
  });

  it('reads the token lazily via the configured provider', () => {
    let current: string | null = 'tok-1';
    configureAccountingApi({ getToken: () => current });
    expect(getAuthToken()).toBe('tok-1');
    current = 'tok-2';
    expect(getAuthToken()).toBe('tok-2');
  });

  it('omits Authorization when unauthenticated but always sets JSON content-type', () => {
    configureAccountingApi({ getToken: () => null });
    const headers = authHeaders();
    expect(headers.get('Content-Type')).toBe('application/json');
    expect(headers.has('Authorization')).toBe(false);
  });

  it('attaches a Bearer token when present', () => {
    configureAccountingApi({ getToken: () => 'jwt-abc' });
    const headers = authHeaders();
    expect(headers.get('Authorization')).toBe('Bearer jwt-abc');
  });
});
