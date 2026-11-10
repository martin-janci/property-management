/**
 * Server-side API-base / site-URL resolver — characterization tests.
 *
 * These pin the exact host-inference + resolve behavior that was previously
 * copied into three SSR call sites (listings/[slug]/page.tsx, lib/layout.ts,
 * app/env.js/route.ts) and is now centralized here. The precedence
 * (API_INTERNAL_URL > host inference > NEXT_PUBLIC_API_URL > localhost) is
 * load-bearing: inference must win over a misconfigured public env so a
 * baked-in `localhost:8081` can't take down SSR on staging/prod.
 */

import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { inferApiBaseFromHost, inferSiteUrlFromHost, resolveApiBase } from './server-api-base';

describe('inferApiBaseFromHost', () => {
  it('maps prod *.rlt.sk hosts to the prod API', () => {
    expect(inferApiBaseFromHost('rlt.sk')).toBe('https://api.rlt.sk');
    expect(inferApiBaseFromHost('example.rlt.sk')).toBe('https://api.rlt.sk');
  });

  it('maps staging hosts to the staging API', () => {
    expect(inferApiBaseFromHost('staging.rlt.sk')).toBe('https://api.staging.rlt.sk');
    expect(inferApiBaseFromHost('wt-foo.staging.rlt.sk')).toBe('https://api.staging.rlt.sk');
  });

  it('strips a port and lowercases the Host header before matching', () => {
    expect(inferApiBaseFromHost('Example.RLT.sk:443')).toBe('https://api.rlt.sk');
  });

  it('returns null for unknown hosts, empty string, and null', () => {
    expect(inferApiBaseFromHost('vercel.app')).toBeNull();
    expect(inferApiBaseFromHost('localhost:3001')).toBeNull();
    expect(inferApiBaseFromHost('')).toBeNull();
    expect(inferApiBaseFromHost(null)).toBeNull();
  });
});

describe('inferSiteUrlFromHost', () => {
  it('returns the https origin for known *.rlt.sk hosts (lowercased, port stripped)', () => {
    expect(inferSiteUrlFromHost('Example.RLT.sk:443')).toBe('https://example.rlt.sk');
    expect(inferSiteUrlFromHost('staging.rlt.sk')).toBe('https://staging.rlt.sk');
  });

  it('returns null for unknown hosts and null', () => {
    expect(inferSiteUrlFromHost('vercel.app')).toBeNull();
    expect(inferSiteUrlFromHost(null)).toBeNull();
  });
});

describe('resolveApiBase', () => {
  const originalEnv = { ...process.env };

  beforeEach(() => {
    delete process.env.API_INTERNAL_URL;
    delete process.env.NEXT_PUBLIC_API_URL;
  });

  afterEach(() => {
    process.env = { ...originalEnv };
  });

  it('prefers API_INTERNAL_URL above everything', () => {
    process.env.API_INTERNAL_URL = 'http://api-server:8081';
    process.env.NEXT_PUBLIC_API_URL = 'https://ignored.example';
    expect(resolveApiBase('example.rlt.sk')).toBe('http://api-server:8081');
  });

  it('uses host inference ahead of NEXT_PUBLIC_API_URL for known hosts', () => {
    process.env.NEXT_PUBLIC_API_URL = 'http://localhost:8081';
    expect(resolveApiBase('example.rlt.sk')).toBe('https://api.rlt.sk');
  });

  it('falls back to NEXT_PUBLIC_API_URL for unknown hosts', () => {
    process.env.NEXT_PUBLIC_API_URL = 'https://custom.example';
    expect(resolveApiBase('unknown.example')).toBe('https://custom.example');
    expect(resolveApiBase(null)).toBe('https://custom.example');
  });

  it('falls back to localhost:8081 when nothing else resolves', () => {
    expect(resolveApiBase('unknown.example')).toBe('http://localhost:8081');
    expect(resolveApiBase(null)).toBe('http://localhost:8081');
  });
});
