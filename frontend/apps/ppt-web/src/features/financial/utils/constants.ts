/**
 * Shared constants for financial components.
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
 * Human-readable labels for invoice statuses.
 */
export const INVOICE_STATUS_LABELS: Record<InvoiceStatus, string> = {
  draft: 'Draft',
  sent: 'Sent',
  paid: 'Paid',
  partial: 'Partial',
  overdue: 'Overdue',
  cancelled: 'Cancelled',
  void: 'Void',
};

/**
 * Payment method options with values and labels.
 */
export const PAYMENT_METHODS: { value: PaymentMethod; label: string }[] = [
  { value: 'bank_transfer', label: 'Bank Transfer' },
  { value: 'card', label: 'Card' },
  { value: 'cash', label: 'Cash' },
  { value: 'check', label: 'Check' },
  { value: 'online', label: 'Online Payment' },
  { value: 'direct_debit', label: 'Direct Debit' },
  { value: 'other', label: 'Other' },
];

/**
 * Map of payment method values to labels.
 */
export const PAYMENT_METHOD_LABELS: Record<PaymentMethod, string> = Object.fromEntries(
  PAYMENT_METHODS.map((method) => [method.value, method.label])
) as Record<PaymentMethod, string>;

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
