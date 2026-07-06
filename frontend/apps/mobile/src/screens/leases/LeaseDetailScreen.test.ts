import { parseLeaseDetail, toUiLease, toUiPayments } from './LeaseDetailScreen';

const validDetail = {
  lease: {
    id: 'l-1',
    landlord_name: 'Anna Novak',
    tenant_name: 'John Doe',
    start_date: '2024-09-01',
    end_date: '2026-08-31',
    monthly_rent: '850.00',
    status: 'active',
  },
  unit_name: 'Apt 3B',
  building_name: 'Lipová Residence',
  upcoming_payments: [
    { id: 'p1', due_date: '2026-03-01', amount: '850', paid_at: '2026-03-01T09:00:00Z' },
    { id: 'p2', due_date: '2026-04-01', amount: '850', paid_at: null },
  ],
};

describe('parseLeaseDetail', () => {
  it('parses a well-formed LeaseWithDetails', () => {
    const parsed = parseLeaseDetail(validDetail);
    expect(parsed?.lease.id).toBe('l-1');
    expect(parsed?.unit_name).toBe('Apt 3B');
    expect(parsed?.upcoming_payments).toHaveLength(2);
  });

  it.each([
    ['null', null],
    ['a string', 'nope'],
    ['no lease', { unit_name: 'x' }],
    ['lease without id', { lease: {} }],
  ])('returns null for an unexpected shape: %s', (_label, input) => {
    expect(parseLeaseDetail(input)).toBeNull();
  });
});

describe('toUiLease', () => {
  it('maps the api detail onto the UI lease shape', () => {
    const lease = toUiLease(parseLeaseDetail(validDetail)!);
    expect(lease).toEqual({
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

  it('coerces a non-numeric rent to 0', () => {
    const parsed = parseLeaseDetail({
      ...validDetail,
      lease: { ...validDetail.lease, monthly_rent: 'n/a' },
    })!;
    expect(toUiLease(parsed).rentAmount).toBe(0);
  });
});

describe('toUiPayments', () => {
  it('maps payments newest first with period labels and paid state', () => {
    const rows = toUiPayments(parseLeaseDetail(validDetail)!);
    expect(rows.map((r) => r.id)).toEqual(['p2', 'p1']);
    expect(rows[0].paidAt).toBeNull();
    expect(rows[1].paidAt).toBe('2026-03-01T09:00:00Z');
  });
});
