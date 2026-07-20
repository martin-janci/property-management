import { afterEach, describe, expect, it, vi } from 'vitest';
import { listingRegistry, registryManifest } from '../components/listings/sections/registry';
import { DEFAULT_LISTING_DETAIL_LAYOUT, getResolvedLayout } from './layout';
import layoutManifest from './layout-manifest.json';

describe('getResolvedLayout', () => {
  afterEach(() => vi.unstubAllGlobals());

  it('returns fetched layout on success', async () => {
    const payload = {
      screen: 'reality/listing-detail',
      version: 3,
      sections: [{ type: 'gallery.v1', presentation: 'visible' }],
    };
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue(new Response(JSON.stringify(payload), { status: 200 }))
    );
    await expect(getResolvedLayout(null)).resolves.toEqual(payload);
  });

  it('passes only global tag when host is null', async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      new Response(
        JSON.stringify({
          screen: 'reality/listing-detail',
          version: 1,
          sections: [],
        }),
        { status: 200 }
      )
    );
    vi.stubGlobal('fetch', fetchMock);
    await getResolvedLayout(null);
    const [, options] = fetchMock.mock.calls[0] as [string, { next?: { tags?: string[] } }];
    expect(options.next?.tags).toEqual(['layout:listing-detail']);
  });

  it('passes only the global tag when host is provided (host tag family was never revalidated)', async () => {
    const host = 'example.rlt.sk';
    const fetchMock = vi.fn().mockResolvedValue(
      new Response(
        JSON.stringify({
          screen: 'reality/listing-detail',
          version: 1,
          sections: [],
        }),
        { status: 200 }
      )
    );
    vi.stubGlobal('fetch', fetchMock);
    await getResolvedLayout(host);
    const [, options] = fetchMock.mock.calls[0] as [string, { next?: { tags?: string[] } }];
    expect(options.next?.tags).toEqual(['layout:listing-detail']);
  });

  it('drops malformed elements (null / non-object / missing string type) instead of crashing', async () => {
    const payload = {
      screen: 'reality/listing-detail',
      version: 3,
      sections: [
        null,
        42,
        'string',
        { presentation: 'visible' },
        { type: 7, presentation: 'visible' },
        { type: 'gallery.v1', presentation: 'visible' },
      ],
    };
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue(new Response(JSON.stringify(payload), { status: 200 }))
    );
    const result = await getResolvedLayout(null);
    expect(result.sections).toEqual([{ type: 'gallery.v1', presentation: 'visible' }]);
  });

  it('clamps sections to 100 entries', async () => {
    const payload = {
      screen: 'reality/listing-detail',
      version: 3,
      sections: Array.from({ length: 150 }, (_, i) => ({
        type: `s${i}`,
        presentation: 'visible',
      })),
    };
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue(new Response(JSON.stringify(payload), { status: 200 }))
    );
    const result = await getResolvedLayout(null);
    expect(result.sections).toHaveLength(100);
    expect(result.sections[99]?.type).toBe('s99');
  });

  it('falls back to the default layout on non-2xx', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response('nope', { status: 404 })));
    await expect(getResolvedLayout(null)).resolves.toEqual(DEFAULT_LISTING_DETAIL_LAYOUT);
  });

  it('falls back to the default layout on network error', async () => {
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new Error('down')));
    await expect(getResolvedLayout(null)).resolves.toEqual(DEFAULT_LISTING_DETAIL_LAYOUT);
  });

  it('falls back on malformed payload', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue(new Response(JSON.stringify({ nope: true }), { status: 200 }))
    );
    await expect(getResolvedLayout(null)).resolves.toEqual(DEFAULT_LISTING_DETAIL_LAYOUT);
  });

  it('default layout lists all eight sections visible in base order', () => {
    expect(DEFAULT_LISTING_DETAIL_LAYOUT.sections.map((s) => s.type)).toEqual([
      'gallery.v1',
      'listing-header.v1',
      'key-details.v1',
      'description.v1',
      'features.v1',
      'additional-info.v1',
      'resources.v1',
      'agent-contact.v1',
    ]);
  });
});

describe('layout-manifest consistency', () => {
  it('manifest matches registry exactly', () => {
    const manifest = registryManifest(listingRegistry);
    expect(manifest).toEqual(registryManifest(listingRegistry));
    expect(layoutManifest).toEqual(manifest);
  });

  it('every DEFAULT_LISTING_DETAIL_LAYOUT section type exists in the registry', () => {
    for (const section of DEFAULT_LISTING_DETAIL_LAYOUT.sections) {
      expect(
        listingRegistry,
        `section type "${section.type}" from DEFAULT_LISTING_DETAIL_LAYOUT is missing from listingRegistry`
      ).toHaveProperty(section.type);
    }
  });
});
