import {
  type ApiLeaseSummary,
  parseLeaseSummaries,
  toUiLease,
  toUiLeaseStatus,
} from './LeasesScreen';

const validSummary: ApiLeaseSummary = {
  id: 'l-1',
  unit_name: 'Apt 3B',
  building_name: 'Lipová Residence',
  tenant_name: 'Anna Novak',
  start_date: '2024-09-01',
  end_date: '2026-08-31',
  monthly_rent: '850.00',
  status: 'active',
  days_until_expiry: 70,
};

describe('parseLeaseSummaries', () => {
  it('accepts the wrapped `{ items: [...] }` envelope', () => {
    expect(parseLeaseSummaries({ items: [validSummary], total: 1 })).toEqual([validSummary]);
  });

  it('accepts a bare array (defensive)', () => {
    expect(parseLeaseSummaries([validSummary])).toEqual([validSummary]);
  });

  // The screen no longer ships a MOCK_LEASES array — it renders whatever the
  // network returns. An unexpected shape must degrade to the empty state.
  it.each([
    ['null', null],
    ['undefined', undefined],
    ['a number', 42],
    ['a string', 'unexpected'],
    ['an object without items', { error: 'boom' }],
    ['an object whose items is not an array', { items: 'nope' }],
  ])('returns [] for an unexpected top-level shape: %s', (_label, input) => {
    expect(parseLeaseSummaries(input)).toEqual([]);
  });

  it('drops malformed entries instead of throwing', () => {
    const mixed = [
      validSummary,
      null,
      'garbage',
      { id: 'no-unit' }, // missing unit_name/building_name/status
      { ...validSummary, id: 123 }, // wrong field type
    ];
    expect(parseLeaseSummaries({ items: mixed })).toEqual([validSummary]);
  });
});

describe('toUiLeaseStatus', () => {
  it('maps active to active', () => {
    expect(toUiLeaseStatus('active')).toBe('active');
  });

  it.each([['pending'], ['draft'], ['pending_signature']])('maps %s to "pending"', (status) => {
    expect(toUiLeaseStatus(status)).toBe('pending');
  });

  it.each([['terminated'], ['cancelled']])('maps %s to "terminated"', (status) => {
    expect(toUiLeaseStatus(status)).toBe('terminated');
  });

  it.each([['expired'], ['ended']])('maps %s to "expired"', (status) => {
    expect(toUiLeaseStatus(status)).toBe('expired');
  });

  it.each([['unknown'], [''], ['whatever']])(
    'maps unrecognised status %s to "pending"',
    (status) => {
      expect(toUiLeaseStatus(status)).toBe('pending');
    }
  );
});

describe('toUiLease', () => {
  it('maps the api summary onto the UI lease shape', () => {
    expect(toUiLease(validSummary)).toEqual({
      id: 'l-1',
      unitLabel: 'Apt 3B',
      buildingName: 'Lipová Residence',
      role: 'tenant',
      rentAmount: 850,
      currency: 'EUR',
      startsAt: '2024-09-01',
      endsAt: '2026-08-31',
      status: 'active',
      counterpartyName: 'Anna Novak',
    });
  });

  it('parses a Decimal rent serialised as a JSON string', () => {
    expect(toUiLease({ ...validSummary, monthly_rent: '1234.56' }).rentAmount).toBeCloseTo(1234.56);
  });

  it('accepts a numeric rent verbatim', () => {
    expect(toUiLease({ ...validSummary, monthly_rent: 999 }).rentAmount).toBe(999);
  });

  it.each([
    ['empty string', ''],
    ['non-numeric', 'free'],
  ])('defaults an unparseable rent (%s) to 0 instead of NaN', (_label, value) => {
    expect(toUiLease({ ...validSummary, monthly_rent: value }).rentAmount).toBe(0);
  });
});
