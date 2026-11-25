const mockApiRequest = jest.fn();

jest.mock('../../hooks/useApi', () => ({
  apiRequest: (...args: unknown[]) => mockApiRequest(...args),
}));
jest.mock('../../contexts/AuthContext', () => ({
  useAuth: () => ({ user: null }),
}));

import { fetchMyMeters, parseMyUnitIds } from './useMyMeters';

describe('parseMyUnitIds', () => {
  it('reads unit ids from a MyUnit[] payload and drops malformed rows', () => {
    expect(
      parseMyUnitIds([
        { unit: { id: 'u-1', buildingId: 'b-1' } },
        { unit: { id: 'u-1' } },
        { unit: {} },
        null,
        { unit: { id: 'u-2' } },
      ])
    ).toEqual(['u-1', 'u-2']);
  });

  it('returns [] for a non-array payload', () => {
    expect(parseMyUnitIds({ units: [] })).toEqual([]);
    expect(parseMyUnitIds(null)).toEqual([]);
  });
});

describe('fetchMyMeters', () => {
  beforeEach(() => mockApiRequest.mockReset());

  it('resolves the units from /users/me/units and loads the meters of each', async () => {
    mockApiRequest.mockImplementation(async (path: string) => {
      if (path === '/api/v1/users/me/units')
        return [{ unit: { id: 'u-1' } }, { unit: { id: 'u-2' } }];
      if (path === '/api/v1/meters/units/u-1') return [{ id: 'm-1' }];
      if (path === '/api/v1/meters/units/u-2') return { meters: [{ id: 'm-2' }] };
      throw new Error(`unexpected ${path}`);
    });

    await expect(fetchMyMeters(undefined)).resolves.toEqual({
      unitIds: ['u-1', 'u-2'],
      meters: [{ id: 'm-1' }, { id: 'm-2' }],
    });
  });

  it('skips the units lookup when the session already knows the unit', async () => {
    mockApiRequest.mockResolvedValue([{ id: 'm-9' }]);

    await expect(fetchMyMeters('u-9')).resolves.toEqual({
      unitIds: ['u-9'],
      meters: [{ id: 'm-9' }],
    });
    expect(mockApiRequest).toHaveBeenCalledTimes(1);
    expect(mockApiRequest.mock.calls[0][0]).toBe('/api/v1/meters/units/u-9');
  });

  it('reports no units (and loads nothing) when the user has none', async () => {
    mockApiRequest.mockResolvedValue([]);

    await expect(fetchMyMeters(undefined)).resolves.toEqual({ unitIds: [], meters: [] });
    expect(mockApiRequest).toHaveBeenCalledTimes(1);
  });
});
