/**
 * Accounting route group (Native Accounting MVP).
 */

import { Route } from 'react-router-dom';
import { AccountingInvoiceManagementPage } from '../lazyRoutes';

/** Accounting routes. */
export function accountingRoutes() {
  return (
    <>
      <Route path="/accounting/invoices" element={<AccountingInvoiceManagementPage />} />
    </>
  );
}
