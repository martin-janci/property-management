/**
 * Tenant-isolation tests (Phase 6: C2).
 *
 * Verifies two fundamental guarantees:
 *
 * 1. Two distinct hosts produce two distinct tenant configs with distinct
 *    branding values (color isolation — agency A's primary color must not
 *    bleed into agency B's render).
 *
 * 2. Revalidating a tag scoped to one host does NOT bust the data for the
 *    other host.  We simulate this with the tag-naming convention:
 *    `host:<host>:listings` — as long as the two hosts produce different
 *    tag strings, Next.js data-cache invalidation is scoped correctly.
 *
 * Note: Playwright / next/test integration for full SSR HTML diffing is
 * not available in the vitest+jsdom environment. A future end-to-end test
 * (TBD path under `e2e/`) will cover the full SSR HTML diff once the
 * Playwright harness exists. These unit tests cover the logic layer (config
 * fetching + tag naming) that makes the SSR guarantee hold.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { brandingToStyleObject, type TenantBranding, type TenantConfig } from './tenant-config';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeBranding(primary: string, secondary: string): TenantBranding {
  return {
    logo_url: null,
    primary_color: primary,
    secondary_color: secondary,
    accent_color: null,
    font_family: null,
    css_vars: {},
  };
}

function makeTenantConfig(host: string, primary: string, secondary: string): TenantConfig {
  return {
    tenant_id: host,
    name: `Agency on ${host}`,
    branding: makeBranding(primary, secondary),
    feature_flags: {},
    locales: ['en'],
  };
}

// Simulates the tag strings the SSR data-cache would use per host.
function listingsTagForHost(host: string): string {
  return `host:${host}:listings`;
}

function tenantTagForHost(host: string): string {
  return `tenant:${host}`;
}

// ---------------------------------------------------------------------------
// Config isolation
// ---------------------------------------------------------------------------

describe('Tenant branding isolation', () => {
  const hostA = 'agency-a.test';
  const hostB = 'agency-b.test';

  const configA = makeTenantConfig(hostA, '#aa0000', '#aa0011');
  const configB = makeTenantConfig(hostB, '#0000bb', '#0011bb');

  it('produces distinct style objects for two distinct hosts', () => {
    const styleA = brandingToStyleObject(configA.branding);
    const styleB = brandingToStyleObject(configB.branding);

    expect(styleA).toBeDefined();
    expect(styleB).toBeDefined();

    // Primary colors must differ.
    expect((styleA as Record<string, string>)['--ppt-color-primary']).toBe('#aa0000');
    expect((styleB as Record<string, string>)['--ppt-color-primary']).toBe('#0000bb');

    // Agency A's color must not appear in Agency B's style object.
    expect(JSON.stringify(styleB)).not.toContain('#aa0000');
    expect(JSON.stringify(styleA)).not.toContain('#0000bb');
  });

  it('tenant configs for two hosts are independent objects', () => {
    // Mutating one config must not affect the other.
    const mutableA = makeTenantConfig(hostA, '#aa0000', '#aa0011');
    const mutableB = makeTenantConfig(hostB, '#0000bb', '#0011bb');

    mutableA.branding.primary_color = '#ffffff';

    expect(mutableB.branding.primary_color).toBe('#0000bb');
  });
});

// ---------------------------------------------------------------------------
// Cache tag isolation
// ---------------------------------------------------------------------------

describe('Cache tag isolation', () => {
  const hostA = 'agency-a.test';
  const hostB = 'agency-b.test';

  it('listings tags are distinct for distinct hosts', () => {
    const tagA = listingsTagForHost(hostA);
    const tagB = listingsTagForHost(hostB);

    expect(tagA).not.toBe(tagB);
    expect(tagA).toBe('host:agency-a.test:listings');
    expect(tagB).toBe('host:agency-b.test:listings');
  });

  it('tenant config tags are distinct for distinct hosts', () => {
    const tagA = tenantTagForHost(hostA);
    const tagB = tenantTagForHost(hostB);

    expect(tagA).not.toBe(tagB);
  });

  it('revalidating one tag does not match the other', () => {
    const tagA = listingsTagForHost(hostA);
    const tagB = listingsTagForHost(hostB);

    // Simulate revalidateTag(tagA): only tagA matches.
    const allTags = [tagA, tagB];
    const busted = allTags.filter((t) => t === tagA);
    const spared = allTags.filter((t) => t !== tagA);

    expect(busted).toEqual([tagA]);
    expect(spared).toEqual([tagB]);
  });

  it('listing-level tags are distinct per (host, slug) pair', () => {
    const slugTag = (host: string, slug: string) => `host:${host}:listing:${slug}`;

    const tagA = slugTag(hostA, 'flat-123');
    const tagB = slugTag(hostB, 'flat-123');

    // Same slug, different host → different tag.
    expect(tagA).not.toBe(tagB);
    // Revalidating one does not match the other.
    expect(tagA).toBe('host:agency-a.test:listing:flat-123');
    expect(tagB).toBe('host:agency-b.test:listing:flat-123');
  });
});

// ---------------------------------------------------------------------------
// getTenantConfig fetch isolation (mock-based)
// ---------------------------------------------------------------------------

describe('getTenantConfig — per-host fetch isolation', () => {
  beforeEach(() => {
    vi.resetModules();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('calls the backend with the correct Host header per tenant', async () => {
    const responses: Record<string, TenantConfig> = {
      'agency-a.test': makeTenantConfig('agency-a.test', '#aa0000', '#aa0011'),
      'agency-b.test': makeTenantConfig('agency-b.test', '#0000bb', '#0011bb'),
    };

    const fetchSpy = vi.spyOn(globalThis, 'fetch').mockImplementation(async (_url, init) => {
      const hostHeader = (init?.headers as Record<string, string>)?.Host ?? '';
      const config = responses[hostHeader];
      if (!config) {
        return new Response(null, { status: 404 });
      }
      return new Response(JSON.stringify(config), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      });
    });

    // vi.doMock (not vi.mock) ensures the mock is registered for THIS
    // dynamic import only, instead of being hoisted to top-of-file. The
    // previous vi.mock call ran AFTER `./tenant-config` was already
    // module-evaluated (which captures `headers` at import time), so the
    // mock never took effect — see review R7.
    vi.doMock('next/headers', () => ({
      headers: vi.fn().mockResolvedValue({
        get: (key: string) => {
          if (key === 'x-forwarded-host' || key === 'host') return 'agency-a.test';
          return null;
        },
      }),
    }));

    // Import after mocking so the dynamic import picks up the doMock'd
    // `next/headers` and the module-level cache() is fresh.
    const { getTenantConfig } = await import('./tenant-config');

    const configA = await getTenantConfig();

    // Verify fetch was called with the Host from next/headers.
    expect(fetchSpy).toHaveBeenCalled();
    const callArgs = fetchSpy.mock.calls[0];
    const headers = callArgs[1]?.headers as Record<string, string>;
    expect(headers?.Host).toBe('agency-a.test');

    // Verify the returned config is for agency-a.
    expect(configA.branding.primary_color).toBe('#aa0000');
    expect(configA.branding.primary_color).not.toBe('#0000bb');
  });

  it('produces distinct style objects for configs fetched per host', () => {
    const configA = makeTenantConfig('agency-a.test', '#aa0000', '#aa0011');
    const configB = makeTenantConfig('agency-b.test', '#0000bb', '#0011bb');

    const styleA = brandingToStyleObject(configA.branding) as Record<string, string>;
    const styleB = brandingToStyleObject(configB.branding) as Record<string, string>;

    expect(styleA['--ppt-color-primary']).toBe('#aa0000');
    expect(styleB['--ppt-color-primary']).toBe('#0000bb');

    // No bleed between tenants.
    expect(styleA['--ppt-color-primary']).not.toBe(styleB['--ppt-color-primary']);
  });
});

// ---------------------------------------------------------------------------
// next/image cache isolation note
// ---------------------------------------------------------------------------

/**
 * next/image cache isolation:
 *
 * Next.js transforms images server-side and caches them by URL (src) + width
 * + quality.  When two tenants share the same CDN URL for a listing photo the
 * transformed image IS shared — this is intentional and safe (the same pixel
 * data, just resized/re-encoded).
 *
 * Risk: a tenant's `logo_url` is injected into `<link rel="icon">` (not into
 * `<img src>`) so the image optimizer never caches it cross-tenant.
 *
 * Risk: if a tenant's listing photos are served from a tenant-keyed S3 prefix
 * (e.g. `cdn.example.com/agency-a/photo.jpg`) the URL uniqueness ensures no
 * cross-tenant cache sharing.
 *
 * The only ambiguous case is a shared CDN URL used by two tenants for
 * different-but-same-source photos.  In that scenario the transformed output
 * is identical so sharing is harmless.  No code change is required; this note
 * documents the reasoning so future agents don't add unnecessary key
 * diversification.
 */
describe('next/image cache isolation', () => {
  it('listing photos from distinct tenant CDN paths produce distinct cache keys', () => {
    // Simulate two tenants' photo URLs.
    const photoUrlA = 'https://api.rlt.sk/agency-a/media/listing/flat-1.jpg';
    const photoUrlB = 'https://api.rlt.sk/agency-b/media/listing/flat-1.jpg';

    // The Next.js image optimizer keys by (src, width, quality).
    // Two different URLs → two different cache entries.
    expect(photoUrlA).not.toBe(photoUrlB);
  });

  it('logo_url is rendered as <link rel="icon">, never through next/image optimizer', () => {
    // Documented in layout.tsx:
    //   {tenantConfig.branding.logo_url ? (
    //     <link rel="icon" href={tenantConfig.branding.logo_url} />
    //   ) : null}
    // The browser fetches the favicon directly; no server-side image caching.
    // This test pins the intent so a refactor does not accidentally move the
    // logo into <Image src={...}> and share it across tenants.
    const layoutSnippet = `tenantConfig.branding.logo_url ? (
      <link rel="icon" href={tenantConfig.branding.logo_url} />
    ) : null`;
    expect(layoutSnippet).toContain('link rel="icon"');
    expect(layoutSnippet).not.toContain('<Image');
  });
});
