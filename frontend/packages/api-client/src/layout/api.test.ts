import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  fetchResolvedLayout,
  fetchTenantLayout,
  saveTenantLayoutOverride,
  TenantLayoutError,
} from './api';

vi.mock('../lib/fetch', () => ({
  authenticatedFetchJson: vi.fn(),
}));

vi.mock('../auth', () => ({
  getToken: vi.fn(),
  getOrg: vi.fn(),
}));

import { getOrg, getToken } from '../auth';
import { authenticatedFetchJson } from '../lib/fetch';

const mockFetch = vi.mocked(authenticatedFetchJson);
const mockGetToken = vi.mocked(getToken);
const mockGetOrg = vi.mocked(getOrg);

describe('fetchResolvedLayout', () => {
  it('returns a valid ResolvedScreen', async () => {
    mockFetch.mockResolvedValueOnce({
      screen: 'dashboard',
      version: 1,
      sections: [{ type: 'hero', presentation: 'visible' }],
    });
    const result = await fetchResolvedLayout('dashboard');
    expect(result.screen).toBe('dashboard');
    expect(result.sections).toHaveLength(1);
  });

  it('throws when sections is missing', async () => {
    mockFetch.mockResolvedValueOnce({ screen: 'dashboard', version: 1 } as never);
    await expect(fetchResolvedLayout('dashboard')).rejects.toThrow(
      'layout: malformed ResolvedScreen payload'
    );
  });

  it('throws when sections is not an array', async () => {
    mockFetch.mockResolvedValueOnce({
      screen: 'dashboard',
      version: 1,
      sections: 'bad' as never,
    });
    await expect(fetchResolvedLayout('dashboard')).rejects.toThrow(
      'layout: malformed ResolvedScreen payload'
    );
  });

  it('throws when payload is null', async () => {
    mockFetch.mockResolvedValueOnce(null as never);
    await expect(fetchResolvedLayout('dashboard')).rejects.toThrow(
      'layout: malformed ResolvedScreen payload'
    );
  });

  it('passes platform param to the URL', async () => {
    mockFetch.mockResolvedValueOnce({
      screen: 'home',
      version: 2,
      sections: [],
    });
    await fetchResolvedLayout('home', 'mobile');
    expect(mockFetch).toHaveBeenCalledWith('/api/v1/layout/resolved/home?platform=mobile');
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

    const promise = saveTenantLayoutOverride('ppt/dashboard', {});
    await expect(promise).rejects.toThrow(TenantLayoutError);

    try {
      await saveTenantLayoutOverride('ppt/dashboard', {});
    } catch (e) {
      expect(e).toBeInstanceOf(TenantLayoutError);
      const err = e as TenantLayoutError;
      expect(err.status).toBe(422);
      expect(err.errors).toEqual(['field x is required', 'invalid mode']);
    }
  });
});
