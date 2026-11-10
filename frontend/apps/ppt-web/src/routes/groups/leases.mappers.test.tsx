/**
 * Regression tests for the lease API↔UI mapper currency handling.
 *
 * `leases.tsx` owns the API↔UI mappers. The backend lease wire shapes
 * (`Lease`, `LeaseSummary`, `LeasePayment`, `LeaseStatistics`, ...) carry NO
 * currency field, so the UI cannot derive a per-lease currency from the
 * payload. Historically the mappers hard-coded `'EUR'` for every monetary
 * value, which mislabels amounts in PLN / HUF / CZK markets (a code-review
 * finding).
 *
 * Until the API models currency per entity, the default is sourced from the
 * deployment-level `VITE_DEFAULT_CURRENCY` env var (falling back to EUR). These
 * tests pin that behaviour: the mapper output currency must follow the
 * configured default, not a baked-in 'EUR'.
 */
import type {
  Lease as ApiLease,
  LeasePayment as ApiLeasePayment,
  LeaseStatistics as ApiLeaseStatistics,
  LeaseSummary as ApiLeaseSummary,
} from '@ppt/api-client';
import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  mapLeaseSummaryToUi,
  mapLeaseToUi,
  mapPaymentToUi,
  mapStatisticsToUi,
  resolveDefaultCurrency,
} from './leases';

const apiSummary: ApiLeaseSummary = {
  id: 'lease-1',
  unit_id: 'unit-1',
  unit_name: '1A',
  building_name: 'Maple Court',
  tenant_name: 'Jan Kowalski',
  tenant_email: 'jan@example.com',
  start_date: '2026-01-01',
  end_date: '2026-12-31',
  monthly_rent: '2500.00',
  status: 'active',
  days_until_expiry: 90,
};

const apiPayment: ApiLeasePayment = {
  id: 'pay-1',
  lease_id: 'lease-1',
  organization_id: 'org-1',
  due_date: '2026-02-01',
  amount: '2500.00',
  payment_type: 'rent',
  is_late: false,
  created_at: '2026-01-15T00:00:00Z',
  updated_at: '2026-01-15T00:00:00Z',
};

const apiStatistics: ApiLeaseStatistics = {
  total_leases: 10,
  active_leases: 8,
  pending_signatures: 1,
  expiring_soon: 2,
  total_applications: 3,
  pending_applications: 1,
  total_monthly_rent: '20000.00',
  occupancy_rate: 0.8,
};

const apiLease: ApiLease = {
  id: 'lease-1',
  organization_id: 'org-1',
  unit_id: 'unit-1',
  landlord_name: 'Acme Sp. z o.o.',
  tenant_name: 'Jan Kowalski',
  tenant_email: 'jan@example.com',
  start_date: '2026-01-01',
  end_date: '2026-12-31',
  term_months: 12,
  is_fixed_term: true,
  monthly_rent: '2500.00',
  security_deposit: '2500.00',
  rent_due_day: 1,
  parking_spaces: 0,
  storage_units: 0,
  pets_allowed: false,
  smoking_allowed: false,
  document_version: 1,
  status: 'active',
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-01T00:00:00Z',
};

afterEach(() => {
  vi.unstubAllEnvs();
});

describe('resolveDefaultCurrency', () => {
  it('falls back to EUR when VITE_DEFAULT_CURRENCY is unset', () => {
    vi.stubEnv('VITE_DEFAULT_CURRENCY', '');
    expect(resolveDefaultCurrency()).toBe('EUR');
  });

  it('honours a supported ISO 4217 code (case-insensitive, trimmed)', () => {
    vi.stubEnv('VITE_DEFAULT_CURRENCY', 'pln');
    expect(resolveDefaultCurrency()).toBe('PLN');
    vi.stubEnv('VITE_DEFAULT_CURRENCY', ' HUF ');
    expect(resolveDefaultCurrency()).toBe('HUF');
  });

  it('ignores an unsupported code and falls back to EUR', () => {
    vi.stubEnv('VITE_DEFAULT_CURRENCY', 'XYZ');
    expect(resolveDefaultCurrency()).toBe('EUR');
  });
});

describe('lease mappers honour the configured default currency', () => {
  it('labels PLN amounts as PLN instead of hard-coded EUR', () => {
    vi.stubEnv('VITE_DEFAULT_CURRENCY', 'PLN');
    expect(mapLeaseSummaryToUi(apiSummary).currency).toBe('PLN');
    expect(mapPaymentToUi(apiPayment).currency).toBe('PLN');
    expect(mapLeaseToUi(apiLease).currency).toBe('PLN');
    expect(mapStatisticsToUi(apiStatistics).currency).toBe('PLN');
  });

  it('labels HUF amounts as HUF', () => {
    vi.stubEnv('VITE_DEFAULT_CURRENCY', 'HUF');
    expect(mapLeaseSummaryToUi(apiSummary).currency).toBe('HUF');
    expect(mapPaymentToUi(apiPayment).currency).toBe('HUF');
  });

  it('defaults to EUR for Eurozone deployments (unset env)', () => {
    vi.stubEnv('VITE_DEFAULT_CURRENCY', '');
    expect(mapLeaseSummaryToUi(apiSummary).currency).toBe('EUR');
    expect(mapStatisticsToUi(apiStatistics).currency).toBe('EUR');
  });
});
