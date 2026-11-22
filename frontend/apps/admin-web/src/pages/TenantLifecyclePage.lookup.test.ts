import { afterEach, describe, expect, it, vi } from 'vitest';
import { lookupTenantBySlug } from './TenantLifecyclePage';

afterEach(() => vi.restoreAllMocks());

describe('lookupTenantBySlug', () => {
  it('resolves the exact slug to the real organization id', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({
        organizations: [
          { id: 'o-2', slug: 'acme-north', name: 'Acme North' },
          { id: 'o-1', slug: 'acme', name: 'Acme' },
        ],
      }),
    } as Response);

    const tenant = await lookupTenantBySlug('acme', 'tok');
    expect(tenant).toMatchObject({ id: 'o-1', slug: 'acme', name: 'Acme', softDeletedAt: null });
    const [url] = vi.mocked(fetch).mock.calls[0] as [string];
    expect(url).toContain('/api/v1/platform-admin/organizations?search=acme');
  });

  it('returns null when no organization has that slug', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ organizations: [] }),
    } as Response);
    expect(await lookupTenantBySlug('nope', null)).toBeNull();
  });
});
