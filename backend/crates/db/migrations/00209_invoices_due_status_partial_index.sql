-- Migration: 00209_invoices_due_status_partial_index
-- Partial composite index for the payment-reminder + overdue-transition sweeps
-- (#1769 finding-5 / #1790 finding-4).
--
-- Both scheduler helpers in FinancialRepository filter on the same shape:
--   find_invoices_due_for_reminder:  status IN ('sent','partial')
--                                    AND due_date BETWEEN CURRENT_DATE .. +N
--                                    AND balance_due > 0
--   transition_invoices_to_overdue:  status IN ('sent','partial')
--                                    AND due_date < CURRENT_DATE - N
--                                    AND balance_due > 0
--
-- Only the single-column idx_invoices_status / idx_invoices_due_date existed
-- (migration 00040), so these run as a status-or-date scan that grows with the
-- whole invoices table. A partial composite index keyed on (due_date, status)
-- and filtered to the open-balance rows the sweeps care about matches the hot
-- predicate directly. Mirrors the existing partial-index precedent on the
-- sibling invoice tables (idx_sub_invoices_due, idx_vendor_invoices_due_date).
CREATE INDEX IF NOT EXISTS idx_invoices_due_status
    ON invoices (due_date, status)
    WHERE balance_due > 0;
