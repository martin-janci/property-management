/**
 * Tests for the env-gated frame-ancestors carve-out logic.
 *
 * The buildHeaderEntries function is extracted from next.config.js so it can
 * be tested without importing Next.js plugins or workspace packages.
 */

import { describe, expect, it } from 'vitest';
import { buildHeaderEntries } from '../../next-config-headers.js';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const BASE_CONNECT_SRC = new Set([
  "'self'",
  'https://api.rlt.sk',
  'https://api.staging.rlt.sk',
  'https://api.reality-portal.sk',
  'https://api.reality-portal.cz',
  'https://api.reality-portal.eu',
]);

function getHeaderValue(
  headers: Array<{ key: string; value: string }>,
  key: string
): string | undefined {
  return headers.find((h) => h.key === key)?.value;
}

// ---------------------------------------------------------------------------
// Without LAYOUT_PREVIEW_FRAME_ANCESTORS
// ---------------------------------------------------------------------------

describe('buildHeaderEntries — no layout preview origins', () => {
  const entries = buildHeaderEntries({
    isDev: false,
    connectSrcOrigins: BASE_CONNECT_SRC,
    layoutPreviewOrigins: '',
  });

  it('returns exactly one header entry', () => {
    expect(entries).toHaveLength(1);
  });

  it('blanket entry matches all paths', () => {
    expect(entries[0].source).toBe('/:path*');
  });

  it('blanket entry has no `has` condition', () => {
    expect(entries[0].has).toBeUndefined();
  });

  it('CSP contains frame-ancestors none', () => {
    const csp = getHeaderValue(entries[0].headers, 'Content-Security-Policy');
    expect(csp).toContain("frame-ancestors 'none'");
  });

  it('X-Frame-Options is DENY', () => {
    const xfo = getHeaderValue(entries[0].headers, 'X-Frame-Options');
    expect(xfo).toBe('DENY');
  });
});

describe('buildHeaderEntries — empty string = no carve-out', () => {
  // Note: next.config.js trims LAYOUT_PREVIEW_FRAME_ANCESTORS before passing it
  // to buildHeaderEntries, so an unset or whitespace-only env var becomes ''.
  // buildHeaderEntries checks the truthiness of the origins string; '' is falsy
  // so no carve-out entry is added.
  it('returns one entry when origins is empty string', () => {
    const emptyEntries = buildHeaderEntries({
      isDev: false,
      connectSrcOrigins: BASE_CONNECT_SRC,
      layoutPreviewOrigins: '',
    });
    expect(emptyEntries).toHaveLength(1);
  });
});

// ---------------------------------------------------------------------------
// With LAYOUT_PREVIEW_FRAME_ANCESTORS set
// ---------------------------------------------------------------------------

describe('buildHeaderEntries — with layout preview origins', () => {
  const origins = 'https://admin.example.com http://localhost:3100';
  const entries = buildHeaderEntries({
    isDev: false,
    connectSrcOrigins: BASE_CONNECT_SRC,
    layoutPreviewOrigins: origins,
  });

  it('returns two header entries', () => {
    expect(entries).toHaveLength(2);
  });

  // --- blanket entry (first) ---

  it('blanket entry (first) matches all paths without a has condition', () => {
    expect(entries[0].source).toBe('/:path*');
    expect(entries[0].has).toBeUndefined();
  });

  it('blanket entry CSP still has frame-ancestors none', () => {
    const csp = getHeaderValue(entries[0].headers, 'Content-Security-Policy');
    expect(csp).toContain("frame-ancestors 'none'");
  });

  it('blanket entry still has X-Frame-Options DENY', () => {
    expect(getHeaderValue(entries[0].headers, 'X-Frame-Options')).toBe('DENY');
  });

  // --- carve-out entry (second, last-wins) ---

  it('carve-out entry (second) has layoutPreview query condition', () => {
    const carveOut = entries[1];
    expect(carveOut.has).toEqual([{ type: 'query', key: 'layoutPreview', value: '1' }]);
  });

  it('carve-out entry is scoped to listing-detail routes only', () => {
    const carveOut = entries[1];
    expect(carveOut.source).toBe('/:locale/listings/:slug');
  });

  it('carve-out CSP uses configured frame-ancestors', () => {
    const csp = getHeaderValue(entries[1].headers, 'Content-Security-Policy');
    expect(csp).toContain(`frame-ancestors ${origins}`);
    expect(csp).not.toContain("frame-ancestors 'none'");
  });

  it('carve-out entry does NOT include X-Frame-Options', () => {
    const xfo = entries[1].headers.find((h) => h.key === 'X-Frame-Options');
    expect(xfo).toBeUndefined();
  });

  it('carve-out entry includes other security headers', () => {
    const keys = entries[1].headers.map((h) => h.key);
    expect(keys).toContain('Strict-Transport-Security');
    expect(keys).toContain('Referrer-Policy');
    expect(keys).toContain('X-Content-Type-Options');
    expect(keys).toContain('Permissions-Policy');
  });
});

// ---------------------------------------------------------------------------
// Next.js ordering guarantee: last entry wins for duplicate header keys
//
// This test is documentary — it verifies that the carve-out entry is ordered
// AFTER the blanket entry, so in a Next.js runtime context (where all matching
// entries are applied cumulatively with last-wins), the carve-out's CSP
// frame-ancestors value would overwrite the blanket entry's value.
// ---------------------------------------------------------------------------

describe('header entry ordering (last-wins invariant)', () => {
  it('carve-out entry index is greater than blanket entry index', () => {
    const entries = buildHeaderEntries({
      isDev: false,
      connectSrcOrigins: BASE_CONNECT_SRC,
      layoutPreviewOrigins: 'https://admin.example.com',
    });

    const blanketIdx = entries.findIndex((e) => e.has === undefined);
    const carveOutIdx = entries.findIndex((e) => e.has !== undefined);

    expect(blanketIdx).toBeGreaterThanOrEqual(0);
    expect(carveOutIdx).toBeGreaterThan(blanketIdx);
  });
});

// ---------------------------------------------------------------------------
// Dev mode: unsafe-eval in script-src
// ---------------------------------------------------------------------------

describe('buildHeaderEntries — dev mode', () => {
  it('includes unsafe-eval in script-src in dev', () => {
    const entries = buildHeaderEntries({
      isDev: true,
      connectSrcOrigins: BASE_CONNECT_SRC,
      layoutPreviewOrigins: '',
    });
    const csp = getHeaderValue(entries[0].headers, 'Content-Security-Policy');
    expect(csp).toContain("'unsafe-eval'");
  });

  it('does not include unsafe-eval in script-src in prod', () => {
    const entries = buildHeaderEntries({
      isDev: false,
      connectSrcOrigins: BASE_CONNECT_SRC,
      layoutPreviewOrigins: '',
    });
    const csp = getHeaderValue(entries[0].headers, 'Content-Security-Policy');
    expect(csp).not.toContain("'unsafe-eval'");
  });
});
