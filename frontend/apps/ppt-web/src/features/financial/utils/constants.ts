/**
 * Shared constants for financial components.
 *
 * Human-readable labels live in the locale bundles; these constants only carry
 * the enum values and their i18n keys (`financial.invoiceStatus.*`,
 * `financial.paymentMethod.*`).
 */

import type { BudgetStatus, InvoiceStatus, PaymentMethod } from '@ppt/api-client';

/**
 * Tailwind CSS class names for invoice status badges.
 */
export const INVOICE_STATUS_STYLES: Record<InvoiceStatus, string> = {
  draft: 'bg-gray-100 text-gray-800',
  sent: 'bg-blue-100 text-blue-800',
  paid: 'bg-green-100 text-green-800',
  partial: 'bg-yellow-100 text-yellow-800',
  overdue: 'bg-red-100 text-red-800',
  cancelled: 'bg-gray-100 text-gray-500',
  void: 'bg-gray-100 text-gray-500 line-through',
};

/**
 * Invoice statuses in display order (labels: `financial.invoiceStatus.<status>`).
 */
export const INVOICE_STATUSES: InvoiceStatus[] = [
  'draft',
  'sent',
  'paid',
  'partial',
  'overdue',
  'cancelled',
  'void',
];

/** i18n key for an invoice status label. */
export function invoiceStatusKey(status: string): string {
  return `financial.invoiceStatus.${status}`;
}

/**
 * Payment methods in display order (labels: `financial.paymentMethod.<method>`).
 */
export const PAYMENT_METHODS: PaymentMethod[] = [
  'bank_transfer',
  'card',
  'cash',
  'check',
  'online',
  'direct_debit',
  'other',
];

/** i18n key for a payment method label. */
export function paymentMethodKey(method: string): string {
  return `financial.paymentMethod.${method}`;
}

/**
 * Tailwind CSS class names for budget status badges (Epic 24 lifecycle).
 */
export const BUDGET_STATUS_STYLES: Record<BudgetStatus, string> = {
  draft: 'bg-gray-100 text-gray-800',
  pending_approval: 'bg-yellow-100 text-yellow-800',
  approved: 'bg-blue-100 text-blue-800',
  active: 'bg-green-100 text-green-800',
  closed: 'bg-purple-100 text-purple-800',
};
