/// <reference types="vitest/globals" />
import { paymentUpdate, toCents } from './AccountingInvoiceDetail';

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
