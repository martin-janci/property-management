/**
 * Accounting route group (Native Accounting MVP).
 */

import { Route } from 'react-router-dom';
import { ProtectedRoute } from '../../components';
import { AccountingInvoiceManagementPage, PaymentMatchingPage } from '../lazyRoutes';
import { MANAGER_ROLES } from '../shared';

/** Accounting routes. */
export function accountingRoutes() {
  return (
    <>
      <Route
        path="/accounting/invoices"
        element={
          <ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>
            <AccountingInvoiceManagementPage />
          </ProtectedRoute>
        }
      />
      <Route
        path="/accounting/payment-matching"
        element={
          <ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>
            <PaymentMatchingPage />
          </ProtectedRoute>
        }
      />
    </>
  );
}
