import { generateIdempotencyKey } from './idempotencyKey';

describe('generateIdempotencyKey', () => {
  it('produces an RFC-4122 v4-shaped string', () => {
    const key = generateIdempotencyKey();
    expect(key).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/);
  });

  it('is unique across calls (so each report dedupes independently)', () => {
    const keys = new Set(Array.from({ length: 1000 }, () => generateIdempotencyKey()));
    expect(keys.size).toBe(1000);
  });
});
