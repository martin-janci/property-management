import { describe, expect, it, vi } from 'vitest';
import { fetchResolvedLayout } from './api';

vi.mock('../lib/fetch', () => ({
  authenticatedFetchJson: vi.fn(),
}));

import { authenticatedFetchJson } from '../lib/fetch';

const mockFetch = vi.mocked(authenticatedFetchJson);

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
