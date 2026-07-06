import { type ApiMeter, parseMeters, toUiCommodity, toUiMeter } from './MetersScreen';

const validMeter: ApiMeter = {
  id: 'm-1',
  meter_number: 'CW-001',
  meter_type: 'cold_water',
  unit_of_measure: 'm³',
  current_reading: '142.4',
  last_reading_date: '2026-03-31',
};

describe('parseMeters', () => {
  it('accepts a bare array (the unit route returns Vec<Meter>)', () => {
    expect(parseMeters([validMeter])).toEqual([validMeter]);
  });

  it('accepts a `{ meters: [...] }` envelope (defensive)', () => {
    expect(parseMeters({ meters: [validMeter] })).toEqual([validMeter]);
  });

  it.each([
    ['null', null],
    ['undefined', undefined],
    ['a number', 42],
    ['a string', 'nope'],
    ['an object without meters', { error: 'boom' }],
  ])('returns [] for an unexpected top-level shape: %s', (_label, input) => {
    expect(parseMeters(input)).toEqual([]);
  });

  it('drops malformed rows', () => {
    expect(parseMeters([validMeter, null, 'x', { noId: true }])).toEqual([validMeter]);
  });
});

describe('toUiCommodity', () => {
  it.each([
    ['cold_water', 'water_cold'],
    ['hot_water', 'water_hot'],
    ['water', 'water'],
    ['gas', 'gas'],
    ['heat', 'heat'],
    ['solar', 'heat'],
    ['electricity', 'electricity'],
    ['unknown', 'electricity'],
    [null, 'electricity'],
  ])('maps meter_type %s → %s', (input, expected) => {
    expect(toUiCommodity(input as string | null)).toBe(expected);
  });
});

describe('toUiMeter', () => {
  it('maps an api meter onto the UI shape', () => {
    expect(toUiMeter(validMeter)).toEqual({
      id: 'm-1',
      label: 'CW-001',
      commodity: 'water_cold',
      unit: 'm³',
      lastReading: 142.4,
      lastReadingAt: '2026-03-31',
    });
  });

  it('handles a missing current reading', () => {
    const mapped = toUiMeter({ ...validMeter, current_reading: null });
    expect(mapped.lastReading).toBeNull();
  });

  it('falls back to location then a generic label when meter_number is absent', () => {
    expect(toUiMeter({ id: 'm-2', location: 'Basement' }).label).toBe('Basement');
    expect(toUiMeter({ id: 'm-3' }).label).toBe('Meter');
  });
});
