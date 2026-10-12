/**
 * Tests for `applyAuthHeaders` — the single source of truth for the
 * authenticated header set shared by the generated-client interceptor and the
 * raw-`fetch` primitive (#3006).
 */

import { afterEach, describe, expect, it } from 'vitest';
import { applyAuthHeaders, bearerAuthorization } from './apply-headers';
import { clearOrgProvider, setOrgProvider } from './org-provider';
import { clearTokenProvider, setTokenProvider } from './token-provider';

afterEach(() => {
  clearTokenProvider();
  clearOrgProvider();
});

describe('bearerAuthorization', () => {
  it('formats the token from the provider as a Bearer credential', () => {
    setTokenProvider(() => 'tok-123');
    expect(bearerAuthorization()).toBe('Bearer tok-123');
  });

  it('returns undefined when no token provider is registered', () => {
    clearTokenProvider();
    expect(bearerAuthorization()).toBeUndefined();
  });

  it('returns undefined when the provider yields no token', () => {
    setTokenProvider(() => null);
    expect(bearerAuthorization()).toBeUndefined();
  });

  it('is the single source of truth backing applyAuthHeaders Authorization', () => {
    setTokenProvider(() => 'tok-shared');
    const headers = new Headers();

    applyAuthHeaders(headers);

    // The header applyAuthHeaders sets is exactly what the shared derivation
    // returns — the two cannot drift because applyAuthHeaders consumes it.
    expect(headers.get('Authorization')).toBe(bearerAuthorization());
    expect(headers.get('Authorization')).toBe('Bearer tok-shared');
  });
});

describe('applyAuthHeaders', () => {
  it('fills Authorization and X-Tenant-ID from the providers', () => {
    setTokenProvider(() => 'tok-123');
    setOrgProvider(() => 'org-abc');
    const headers = new Headers();

    applyAuthHeaders(headers);

    expect(headers.get('Authorization')).toBe('Bearer tok-123');
    expect(headers.get('X-Tenant-ID')).toBe('org-abc');
  });

  it('adds no headers when there is no token or org', () => {
    setTokenProvider(() => null);
    setOrgProvider(() => null);
    const headers = new Headers();

    applyAuthHeaders(headers);

    expect(headers.has('Authorization')).toBe(false);
    expect(headers.has('X-Tenant-ID')).toBe(false);
  });

  it('does not overwrite headers the caller already set', () => {
    setTokenProvider(() => 'tok-from-provider');
    setOrgProvider(() => 'org-from-provider');
    const headers = new Headers({
      Authorization: 'Bearer explicit',
      'X-Tenant-ID': 'org-explicit',
    });

    applyAuthHeaders(headers);

    expect(headers.get('Authorization')).toBe('Bearer explicit');
    expect(headers.get('X-Tenant-ID')).toBe('org-explicit');
  });

  it('fills only the headers that are missing', () => {
    setTokenProvider(() => 'tok-123');
    setOrgProvider(() => 'org-abc');
    const headers = new Headers({ 'X-Tenant-ID': 'org-explicit' });

    applyAuthHeaders(headers);

    expect(headers.get('Authorization')).toBe('Bearer tok-123');
    expect(headers.get('X-Tenant-ID')).toBe('org-explicit');
  });
});
