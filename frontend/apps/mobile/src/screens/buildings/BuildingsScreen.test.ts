import { type ApiBuildingSummary, parseBuildingSummaries, toUiBuilding } from './BuildingsScreen';

const validSummary: ApiBuildingSummary = {
  id: 'b-1',
  name: 'Lipová Residence',
  street: 'Lipová 5',
  city: 'Bratislava',
  postal_code: '81101',
  total_floors: 7,
  status: 'active',
  unit_count: 36,
};

describe('parseBuildingSummaries', () => {
  it('accepts the wrapped `{ buildings: [...] }` envelope', () => {
    expect(parseBuildingSummaries({ buildings: [validSummary], total: 1 })).toEqual([validSummary]);
  });

  it('accepts a bare array (defensive)', () => {
    expect(parseBuildingSummaries([validSummary])).toEqual([validSummary]);
  });

  // The screen no longer ships a MOCK_BUILDINGS array — it renders whatever the
  // network returns. An unexpected top-level shape must degrade to the empty
  // state, not crash `.map(toUiBuilding)`.
  it.each([
    ['null', null],
    ['undefined', undefined],
    ['a number', 42],
    ['a string', 'unexpected'],
    ['an object without buildings', { error: 'boom' }],
    ['an object whose buildings is not an array', { buildings: 'nope' }],
  ])('returns [] for an unexpected top-level shape: %s', (_label, input) => {
    expect(parseBuildingSummaries(input)).toEqual([]);
  });

  it('drops malformed entries instead of throwing', () => {
    const mixed = [
      validSummary,
      null,
      'garbage',
      { id: 'no-street' }, // missing street/city/status
      { ...validSummary, id: 123 }, // wrong field type
    ];
    expect(parseBuildingSummaries({ buildings: mixed })).toEqual([validSummary]);
  });
});

describe('toUiBuilding', () => {
  it('maps the api summary onto the UI building shape', () => {
    expect(toUiBuilding(validSummary)).toEqual({
      id: 'b-1',
      name: 'Lipová Residence',
      address: 'Lipová 5, 81101, Bratislava',
      unitCount: 36,
      floors: 7,
      status: 'active',
    });
  });

  it('falls back to the street when name is missing or blank', () => {
    expect(toUiBuilding({ ...validSummary, name: null }).name).toBe('Lipová 5');
    expect(toUiBuilding({ ...validSummary, name: '   ' }).name).toBe('Lipová 5');
  });

  it.each([
    ['null', null],
    ['undefined', undefined],
  ])('defaults a missing unit_count (%s) to 0', (_label, value) => {
    expect(
      toUiBuilding({ ...validSummary, unit_count: value as number | null | undefined }).unitCount
    ).toBe(0);
  });
});
