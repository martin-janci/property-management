/// <reference types="vitest/globals" />
import { paymentUpdate, remainingAmount, toCents } from './AccountingInvoiceDetail';

describe('toCents', () => {
  it('parses decimal money strings to exact integer cents', () => {
    expect(toCents('121.00')).toBe(12100);
    expect(toCents('0.10')).toBe(10);
    expect(toCents('70')).toBe(7000);
    expect(toCents('.5')).toBe(50);
    expect(toCents(0.2)).toBe(20);
    expect(toCents('-12.34')).toBe(-1234);
  });

  it('rounds half-up at the hundredths place (no float drift)', () => {
    // (1.005).toFixed(2) is '1.00' in JS — decimal-safe rounding gives 101.
    expect(toCents('1.005')).toBe(101);
    expect(toCents(33.335)).toBe(3334);
    expect(toCents('99.994')).toBe(9999);
  });

  it('returns NaN for non-numeric input', () => {
    expect(toCents('')).toBeNaN();
    expect(toCents('abc')).toBeNaN();
  });
});

describe('paymentUpdate', () => {
  it('marks a partially covered invoice as partially_paid and accumulates', () => {
    expect(paymentUpdate({ total_amount: '121.00', paid_amount: '20' }, 50)).toEqual({
      paid_amount: '70.00',
      status: 'partially_paid',
    });
  });

  it('marks a fully covered invoice as paid', () => {
    expect(paymentUpdate({ total_amount: '121.00', paid_amount: '100.00' }, 21)).toEqual({
      paid_amount: '121.00',
      status: 'paid',
    });
  });

  it('classifies an exactly-covered invoice as paid without a float epsilon', () => {
    // 0.10 + 0.20 is 0.30000000000000004 as a JS float; integer cents compare exact.
    expect(paymentUpdate({ total_amount: '0.30', paid_amount: '0.10' }, 0.2)).toEqual({
      paid_amount: '0.30',
      status: 'paid',
    });
  });

  it('accumulates sub-cent amounts with decimal-safe rounding, not toFixed', () => {
    // Old code: Number('33.33') + 33.335 → (66.665).toFixed(2) === '66.66' (float drift).
    // Decimal-safe: 3333 + 3334 cents = 6667 → '66.67'.
    expect(paymentUpdate({ total_amount: '100.00', paid_amount: '33.33' }, 33.335)).toEqual({
      paid_amount: '66.67',
      status: 'partially_paid',
    });
  });
});

describe('remainingAmount', () => {
  it('returns the full total when nothing is paid', () => {
    expect(remainingAmount({ total_amount: '121.00', paid_amount: '0' })).toBe('121.00');
    expect(remainingAmount({ total_amount: '121.00', paid_amount: '0.00' })).toBe('121.00');
  });

  it('subtracts the paid amount in exact integer cents', () => {
    expect(remainingAmount({ total_amount: '121.00', paid_amount: '20.00' })).toBe('101.00');
    expect(remainingAmount({ total_amount: '0.30', paid_amount: '0.10' })).toBe('0.20');
  });

  it('rounds the subtraction decimal-safely instead of through a JS float', () => {
    // Old code: Math.max(0, Number('10.00') - Number('2.125')).toFixed(2)
    //   → (7.875).toFixed(2) === '7.88' (float path rounds the half up).
    // Decimal-safe: 1000 cents − toCents('2.125')=213 → 787 cents → '7.87'.
    expect(remainingAmount({ total_amount: '10.00', paid_amount: '2.125' })).toBe('7.87');
    expect(remainingAmount({ total_amount: '100.00', paid_amount: '0.125' })).toBe('99.87');
  });

  it('clamps an overpaid invoice to 0.00 rather than a negative string', () => {
    expect(remainingAmount({ total_amount: '50.00', paid_amount: '80.00' })).toBe('0.00');
  });

  it('falls back to 0.00 for non-numeric wire data', () => {
    expect(remainingAmount({ total_amount: 'oops', paid_amount: '0' })).toBe('0.00');
  });
});
