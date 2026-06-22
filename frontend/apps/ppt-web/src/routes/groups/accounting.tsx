/**
 * Accounting route group (Native Accounting MVP).
 */

import { Route } from 'react-router-dom';
import { AccountingInvoiceManagementPage, PaymentMatchingPage } from '../lazyRoutes';

/** Accounting routes. */
export function accountingRoutes() {
  return (
    <>
      <Route path="/accounting/invoices" element={<AccountingInvoiceManagementPage />} />
      <Route path="/accounting/payment-matching" element={<PaymentMatchingPage />} />
    </>
  );
}
