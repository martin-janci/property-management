import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../auth', () => ({
  getToken: vi.fn(),
  getOrg: vi.fn(),
}));

import { getOrg } from '../auth';
import { layoutKeys } from './hooks';

const mockGetOrg = vi.mocked(getOrg);

describe('layoutKeys', () => {
  beforeEach(() => {
    mockGetOrg.mockReturnValue('org-123');
  });

  it('includes the active org in resolved keys', () => {
    expect(layoutKeys.resolved('ppt/dashboard', 'web')).toEqual([
      'layout',
      'org-123',
      'resolved',
      'ppt/dashboard',
      'web',
    ]);
  });

  it('includes the active org in tenant keys', () => {
    expect(layoutKeys.tenant('ppt/dashboard')).toEqual([
      'layout',
      'org-123',
      'tenant',
      'ppt/dashboard',
    ]);
  });

  it('changes keys when the org changes (no stale cross-org cache hits)', () => {
    const before = layoutKeys.resolved('ppt/dashboard', 'web');
    mockGetOrg.mockReturnValue('org-456');
    const after = layoutKeys.resolved('ppt/dashboard', 'web');
    expect(before).not.toEqual(after);
    expect(after[1]).toBe('org-456');
  });

  it('falls back to a stable sentinel when no org is set', () => {
    mockGetOrg.mockReturnValue(null);
    expect(layoutKeys.tenant('ppt/dashboard')[1]).toBe('no-org');
  });

  it('keys remain prefixed by layoutKeys.all (so invalidating all covers both)', () => {
    expect(layoutKeys.resolved('a/b', 'web').slice(0, 1)).toEqual([...layoutKeys.all]);
    expect(layoutKeys.tenant('a/b').slice(0, 1)).toEqual([...layoutKeys.all]);
  });
});
