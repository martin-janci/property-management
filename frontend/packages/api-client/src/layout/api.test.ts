import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  fetchResolvedLayout,
  fetchTenantLayout,
  saveTenantLayoutOverride,
  TenantLayoutError,
} from './api';

vi.mock('../auth', () => ({
  getToken: vi.fn(),
  getOrg: vi.fn(),
}));

import { getOrg, getToken } from '../auth';

const mockGetToken = vi.mocked(getToken);
const mockGetOrg = vi.mocked(getOrg);

function okJson(payload: unknown) {
  return {
    ok: true,
    json: vi.fn().mockResolvedValue(payload),
  } as unknown as Response;
}

describe('fetchResolvedLayout', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn());
    mockGetToken.mockReturnValue('tok-abc');
    mockGetOrg.mockReturnValue('org-123');
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('returns a valid ResolvedScreen', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(
      okJson({
        screen: 'dashboard',
        version: 1,
        sections: [{ type: 'hero', presentation: 'visible' }],
      })
    );
    const result = await fetchResolvedLayout('dashboard');
    expect(result.screen).toBe('dashboard');
    expect(result.sections).toHaveLength(1);
  });

  it('sends Authorization and X-Tenant-ID headers', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(
      okJson({ screen: 'dashboard', version: 1, sections: [] })
    );
    await fetchResolvedLayout('dashboard');
    expect(fetch).toHaveBeenCalledWith(
      '/api/v1/layout/resolved/dashboard?platform=web',
      expect.objectContaining({
        headers: expect.objectContaining({
          Authorization: 'Bearer tok-abc',
          'X-Tenant-ID': 'org-123',
        }),
      })
    );
  });

  it('omits X-Tenant-ID when getOrg() returns null', async () => {
    mockGetOrg.mockReturnValue(null);
    vi.mocked(fetch).mockResolvedValueOnce(
      okJson({ screen: 'dashboard', version: 1, sections: [] })
    );
    await fetchResolvedLayout('dashboard');
    const [, options] = vi.mocked(fetch).mock.calls[0];
    const headers = (options as RequestInit).headers as Record<string, string>;
    expect(headers['X-Tenant-ID']).toBeUndefined();
  });

  it('throws when sections is missing', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(okJson({ screen: 'dashboard', version: 1 }));
    await expect(fetchResolvedLayout('dashboard')).rejects.toThrow(
      'layout: malformed ResolvedScreen payload'
    );
  });

  it('throws when sections is not an array', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(
      okJson({ screen: 'dashboard', version: 1, sections: 'bad' })
    );
    await expect(fetchResolvedLayout('dashboard')).rejects.toThrow(
      'layout: malformed ResolvedScreen payload'
    );
  });

  it('throws when payload is null', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(okJson(null));
    await expect(fetchResolvedLayout('dashboard')).rejects.toThrow(
      'layout: malformed ResolvedScreen payload'
    );
  });

  it('throws TenantLayoutError on non-2xx', async () => {
    vi.mocked(fetch).mockResolvedValueOnce({
      ok: false,
      status: 400,
      json: vi.fn().mockResolvedValue({}),
    } as unknown as Response);
    const err = await fetchResolvedLayout('dashboard').catch((e) => e);
    expect(err).toBeInstanceOf(TenantLayoutError);
    expect((err as TenantLayoutError).status).toBe(400);
  });

  it('filters out null / non-object / missing-type elements', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(
      okJson({
        screen: 'dashboard',
        version: 1,
        sections: [
          null,
          42,
          'string',
          { presentation: 'visible' },
          { type: 7, presentation: 'visible' },
          { type: 'hero', presentation: 'visible' },
        ],
      })
    );
    const result = await fetchResolvedLayout('dashboard');
    expect(result.sections).toEqual([{ type: 'hero', presentation: 'visible' }]);
  });

  it('clamps sections to 100 entries', async () => {
    const sections = Array.from({ length: 150 }, (_, i) => ({
      type: `s${i}`,
      presentation: 'visible',
    }));
    vi.mocked(fetch).mockResolvedValueOnce(okJson({ screen: 'dashboard', version: 1, sections }));
    const result = await fetchResolvedLayout('dashboard');
    expect(result.sections).toHaveLength(100);
    expect(result.sections[99]?.type).toBe('s99');
  });

  it('passes platform param to the URL', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(okJson({ screen: 'home', version: 2, sections: [] }));
    await fetchResolvedLayout('home', 'mobile');
    expect(fetch).toHaveBeenCalledWith(
      '/api/v1/layout/resolved/home?platform=mobile',
      expect.anything()
    );
  });
});

// ---------------------------------------------------------------------------
// fetchTenantLayout / saveTenantLayoutOverride
// ---------------------------------------------------------------------------

const envelope = {
  override: { override_config: { order: ['a', 'b'] } },
  rails: { hideable: [], mode_editable: [], reorderable: false, prop_whitelist: {} },
  published: null,
  manifest: null,
};

describe('fetchTenantLayout', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn());
    mockGetToken.mockReturnValue('tok-abc');
    mockGetOrg.mockReturnValue('org-123');
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('GETs /api/v1/layout/tenant-override with auth and tenant headers', async () => {
    const mockResponse = {
      ok: true,
      json: vi.fn().mockResolvedValue(envelope),
    };
    vi.mocked(fetch).mockResolvedValue(mockResponse as unknown as Response);

    const result = await fetchTenantLayout('ppt/dashboard');

    expect(fetch).toHaveBeenCalledWith(
      '/api/v1/layout/tenant-override?screen=ppt%2Fdashboard',
      expect.objectContaining({
        headers: expect.objectContaining({
          Authorization: 'Bearer tok-abc',
          'X-Tenant-ID': 'org-123',
        }),
      })
    );
    expect(result).toEqual(envelope);
  });

  it('omits X-Tenant-ID when getOrg() returns null', async () => {
    mockGetOrg.mockReturnValue(null);
    const mockResponse = {
      ok: true,
      json: vi.fn().mockResolvedValue(envelope),
    };
    vi.mocked(fetch).mockResolvedValue(mockResponse as unknown as Response);

    await fetchTenantLayout('ppt/dashboard');

    const [, options] = vi.mocked(fetch).mock.calls[0];
    const headers = (options as RequestInit).headers as Record<string, string>;
    expect(headers['X-Tenant-ID']).toBeUndefined();
  });
});

describe('saveTenantLayoutOverride', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn());
    mockGetToken.mockReturnValue('tok-abc');
    mockGetOrg.mockReturnValue('org-123');
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('PUTs { screen, override_config } to /api/v1/layout/tenant-override', async () => {
    const mockResponse = {
      ok: true,
      json: vi.fn().mockResolvedValue({}),
    };
    vi.mocked(fetch).mockResolvedValue(mockResponse as unknown as Response);

    const override = { order: ['a'] };
    await saveTenantLayoutOverride('ppt/dashboard', override);

    expect(fetch).toHaveBeenCalledWith(
      '/api/v1/layout/tenant-override',
      expect.objectContaining({
        method: 'PUT',
        body: JSON.stringify({ screen: 'ppt/dashboard', override_config: override }),
      })
    );
  });

  it('throws TenantLayoutError with verbatim errors on 422', async () => {
    const mockResponse = {
      ok: false,
      status: 422,
      json: vi.fn().mockResolvedValue({ errors: ['field x is required', 'invalid mode'] }),
    };
    vi.mocked(fetch).mockResolvedValue(mockResponse as unknown as Response);

    const err = await saveTenantLayoutOverride('ppt/dashboard', {}).catch((e) => e);
    expect(err).toBeInstanceOf(TenantLayoutError);
    expect((err as TenantLayoutError).status).toBe(422);
    expect((err as TenantLayoutError).errors).toEqual(['field x is required', 'invalid mode']);
  });
});
