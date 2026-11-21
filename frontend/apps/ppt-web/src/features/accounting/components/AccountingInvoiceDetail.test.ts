/// <reference types="vitest/globals" />
import { paymentUpdate } from './AccountingInvoiceDetail';

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
});
