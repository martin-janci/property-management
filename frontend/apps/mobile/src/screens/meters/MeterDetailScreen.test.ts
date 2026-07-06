import { parseMeterResponse, toUiReadings } from './MeterDetailScreen';

const validResponse = {
  meter: { id: 'm-1', meter_number: 'CW-001', unit_of_measure: 'm³' },
  recent_readings: [
    { id: 'r1', reading: '141.2', reading_date: '2026-02-28' },
    { id: 'r2', reading: '142.4', reading_date: '2026-03-31' },
  ],
};

describe('parseMeterResponse', () => {
  it('parses a well-formed MeterResponse', () => {
    const parsed = parseMeterResponse(validResponse);
    expect(parsed?.meter.id).toBe('m-1');
    expect(parsed?.recent_readings).toHaveLength(2);
  });

  it.each([
    ['null', null],
    ['undefined', undefined],
    ['a string', 'nope'],
    ['no meter', { recent_readings: [] }],
    ['meter without id', { meter: {}, recent_readings: [] }],
  ])('returns null for an unexpected shape: %s', (_label, input) => {
    expect(parseMeterResponse(input)).toBeNull();
  });

  it('drops malformed readings but keeps the meter', () => {
    const parsed = parseMeterResponse({
      meter: { id: 'm-1' },
      recent_readings: [{ id: 'r1', reading: '1', reading_date: '2026-01-01' }, null, 'x', {}],
    });
    expect(parsed?.recent_readings).toHaveLength(1);
  });
});

describe('toUiReadings', () => {
  it('maps readings and sorts newest first', () => {
    const rows = toUiReadings(parseMeterResponse(validResponse)!);
    expect(rows.map((r) => r.id)).toEqual(['r2', 'r1']);
    expect(rows[0]).toEqual({ id: 'r2', value: 142.4, unit: 'm³', takenAt: '2026-03-31' });
  });
});
