import { afterEach, describe, expect, it, vi } from 'vitest';
import { LayoutApiError, listScreens, publish, putDraft } from './api';

function okJson(body: unknown) {
  return new Response(JSON.stringify(body), { status: 200 });
}

describe('layout-editor api', () => {
  afterEach(() => vi.unstubAllGlobals());

  it('listScreens sends bearer and returns rows; 404 → empty list', async () => {
    const fetchMock = vi.fn().mockResolvedValue(okJson([{ screen: 'ppt/dashboard' }]));
    vi.stubGlobal('fetch', fetchMock);
    const rows = await listScreens('tok');
    expect(rows).toEqual([{ screen: 'ppt/dashboard' }]);
    const [url, init] = fetchMock.mock.calls[0];
    expect(String(url)).toBe('/api/v1/platform-admin/layout/screens');
    expect((init.headers as Record<string, string>).Authorization).toBe('Bearer tok');
    expect(init.credentials).toBe('include');

    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response('', { status: 404 })));
    await expect(listScreens('tok')).resolves.toEqual([]);
  });

  it('putDraft PUTs the exact body shape', async () => {
    const fetchMock = vi.fn().mockResolvedValue(okJson({}));
    vi.stubGlobal('fetch', fetchMock);
    const config = { screen: 'ppt/dashboard', version: 0, sections: [] };
    await putDraft('tok', 'ppt/dashboard', config);
    const [url, init] = fetchMock.mock.calls[0];
    expect(String(url)).toBe('/api/v1/platform-admin/layout/draft');
    expect(init.method).toBe('PUT');
    expect(JSON.parse(init.body as string)).toEqual({ screen: 'ppt/dashboard', config });
  });

  it('publish surfaces 422 errors as LayoutApiError.errors', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(
      new Response(JSON.stringify({ errors: ['required section gallery.v1 is hidden'] }), {
        status: 422,
      }),
    ));
    const err = await publish('tok', 'ppt/dashboard').catch((e: unknown) => e);
    expect(err).toBeInstanceOf(LayoutApiError);
    expect((err as LayoutApiError).status).toBe(422);
    expect((err as LayoutApiError).errors).toEqual(['required section gallery.v1 is hidden']);
  });
});
