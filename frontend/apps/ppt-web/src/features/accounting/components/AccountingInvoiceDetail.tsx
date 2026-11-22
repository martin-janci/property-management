/**
 * AccountingInvoiceDetail — one native issued invoice with its line items and
 * the status actions the api-server supports through
 * `PATCH /api/v1/accounting/invoices/{id}`: issue a draft, record a (partial)
 * payment. Wire fields are snake_case and amounts are decimal strings.
 */

import type {
  AccountingContact,
  AccountingInvoice,
  AccountingInvoiceItem,
  AccountingUpdateInvoiceRequest,
} from '@ppt/api-client';
import { type FormEvent, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  formatAmount,
  INVOICE_STATUS_I18N_KEYS,
  INVOICE_STATUS_STYLES,
} from './AccountingInvoiceList';

/** Statuses that still expect money in. */
const PAYABLE_STATUSES = new Set(['issued', 'sent', 'overdue', 'partially_paid']);

/**
 * Decimal money string (or number) → integer cents, so amounts compare exactly
 * instead of through JS-float arithmetic. Rounds half-up at the hundredths
 * place; well-formed 2-dp money round-trips unchanged. Returns `NaN` for input
 * that is not a decimal number.
 */
export function toCents(value: string | number): number {
  const str = (typeof value === 'number' ? value.toString() : value).trim();
  const match = /^(-?)(\d*)(?:\.(\d+))?$/.exec(str);
  if (!match || (match[2] === '' && match[3] === undefined)) return Number.NaN;
  const [, sign, whole, frac = ''] = match;
  const cents =
    Number(whole || '0') * 100 +
    Number(frac.slice(0, 2).padEnd(2, '0')) +
    ((frac[2] ?? '') >= '5' ? 1 : 0);
  return sign === '-' ? -cents : cents;
}

/** Integer cents → fixed 2-dp decimal string (same shape as `Number.toFixed(2)`). */
function centsToDecimalString(cents: number): string {
  const abs = Math.abs(cents);
  return `${cents < 0 ? '-' : ''}${Math.floor(abs / 100)}.${String(abs % 100).padStart(2, '0')}`;
}

/**
 * Status + paid amount after recording `amount` on top of what is already paid.
 * Amounts are summed and compared as integer cents so the paid/partially_paid
 * split is exact — no JS-float sum, no `toFixed` rounding, no epsilon. Fully
 * covered → `paid`; otherwise `partially_paid`.
 */
export function paymentUpdate(
  invoice: Pick<AccountingInvoice, 'total_amount' | 'paid_amount'>,
  amount: number
): AccountingUpdateInvoiceRequest {
  const paidCents = toCents(invoice.paid_amount ?? 0) + toCents(amount);
  const totalCents = toCents(invoice.total_amount ?? 0);
  return {
    paid_amount: centsToDecimalString(paidCents),
    status: paidCents >= totalCents ? 'paid' : 'partially_paid',
  };
}

interface AccountingInvoiceDetailProps {
  invoice: AccountingInvoice;
  items: AccountingInvoiceItem[];
  contacts: AccountingContact[];
  isLoadingItems?: boolean;
  isUpdating?: boolean;
  onUpdate: (body: AccountingUpdateInvoiceRequest, successKey: string) => void;
  onBack: () => void;
}

export function AccountingInvoiceDetail({
  invoice,
  items,
  contacts,
  isLoadingItems,
  isUpdating,
  onUpdate,
  onBack,
}: AccountingInvoiceDetailProps) {
  const { t, i18n } = useTranslation();
  const remaining = Math.max(
    0,
    Number(invoice.total_amount ?? 0) - Number(invoice.paid_amount ?? 0)
  );
  const [payment, setPayment] = useState(remaining.toFixed(2));
  const [paymentError, setPaymentError] = useState<string | null>(null);

  const contactName = contacts.find((c) => c.id === invoice.contact_id)?.name ?? '—';
  const formatDate = (d?: string) =>
    d
      ? new Date(d).toLocaleDateString(i18n.language, {
          year: 'numeric',
          month: 'short',
          day: 'numeric',
        })
      : '—';

  const handlePayment = (e: FormEvent) => {
    e.preventDefault();
    const amount = Number.parseFloat(payment);
    if (!Number.isFinite(amount) || amount <= 0) {
      setPaymentError(t('accounting.detail.invalidPayment'));
      return;
    }
    setPaymentError(null);
    onUpdate(paymentUpdate(invoice, amount), 'accounting.detail.paymentRecorded');
  };

  return (
    <div className="max-w-4xl mx-auto space-y-6">
      <button type="button" onClick={onBack} className="text-sm text-blue-600 hover:text-blue-800">
        ← {t('accounting.detail.back')}
      </button>

      <div className="bg-white shadow rounded-lg p-6">
        <div className="flex items-start justify-between">
          <div>
            <h2 className="text-xl font-semibold text-gray-900">
              {t('accounting.detail.title', { number: invoice.number })}
            </h2>
            <p className="text-sm text-gray-500">{contactName}</p>
          </div>
          <span
            className={`px-2 inline-flex text-xs leading-5 font-semibold rounded-full ${INVOICE_STATUS_STYLES[invoice.status]}`}
          >
            {t(INVOICE_STATUS_I18N_KEYS[invoice.status])}
          </span>
        </div>

        <dl className="mt-6 grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
          <div>
            <dt className="text-gray-500">{t('accounting.detail.issueDate')}</dt>
            <dd className="text-gray-900">{formatDate(invoice.issue_date)}</dd>
          </div>
          <div>
            <dt className="text-gray-500">{t('accounting.detail.dueDate')}</dt>
            <dd className="text-gray-900">{formatDate(invoice.due_date)}</dd>
          </div>
          <div>
            <dt className="text-gray-500">{t('accounting.detail.variableSymbol')}</dt>
            <dd className="text-gray-900">{invoice.variable_symbol || '—'}</dd>
          </div>
          <div>
            <dt className="text-gray-500">{t('accounting.detail.paidAmount')}</dt>
            <dd className="text-gray-900">
              {formatAmount(invoice.paid_amount)} / {formatAmount(invoice.total_amount)}{' '}
              {invoice.currency}
            </dd>
          </div>
        </dl>
      </div>

      <div className="bg-white shadow rounded-lg p-6">
        <h3 className="text-lg font-medium text-gray-900 mb-4">{t('accounting.detail.items')}</h3>
        {isLoadingItems ? (
          <div className="flex justify-center py-6">
            <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-blue-600" />
          </div>
        ) : (
          <table className="min-w-full divide-y divide-gray-200 text-sm">
            <thead>
              <tr className="text-left text-gray-500">
                <th className="py-2">{t('accounting.detail.description')}</th>
                <th className="py-2 text-right">{t('accounting.detail.qty')}</th>
                <th className="py-2 text-right">{t('accounting.detail.unitPrice')}</th>
                <th className="py-2 text-right">{t('accounting.detail.vatRate')}</th>
                <th className="py-2 text-right">{t('accounting.detail.lineTotal')}</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {items.map((item) => (
                <tr key={item.id}>
                  <td className="py-2">{item.description}</td>
                  <td className="py-2 text-right">{Number(item.qty)}</td>
                  <td className="py-2 text-right">{formatAmount(item.unit_price)}</td>
                  <td className="py-2 text-right">{Number(item.vat_rate)} %</td>
                  <td className="py-2 text-right font-medium">{formatAmount(item.total_amount)}</td>
                </tr>
              ))}
            </tbody>
            <tfoot className="text-gray-700">
              <tr>
                <td colSpan={4} className="pt-3 text-right">
                  {t('accounting.detail.base')}
                </td>
                <td className="pt-3 text-right">{formatAmount(invoice.base_amount)}</td>
              </tr>
              <tr>
                <td colSpan={4} className="text-right">
                  {t('accounting.detail.vat')}
                </td>
                <td className="text-right">{formatAmount(invoice.vat_amount)}</td>
              </tr>
              <tr className="font-semibold">
                <td colSpan={4} className="text-right">
                  {t('accounting.detail.total')}
                </td>
                <td className="text-right">
                  {formatAmount(invoice.total_amount)} {invoice.currency}
                </td>
              </tr>
            </tfoot>
          </table>
        )}
      </div>

      {invoice.status === 'draft' && (
        <div className="bg-white shadow rounded-lg p-6 flex items-center justify-between">
          <p className="text-sm text-gray-600">{t('accounting.detail.draftHint')}</p>
          <button
            type="button"
            disabled={isUpdating}
            onClick={() => onUpdate({ status: 'issued' }, 'accounting.detail.issued')}
            className="bg-blue-600 text-white px-4 py-2 rounded-md hover:bg-blue-700 disabled:opacity-50"
          >
            {t('accounting.detail.issue')}
          </button>
        </div>
      )}

      {PAYABLE_STATUSES.has(invoice.status) && (
        <form onSubmit={handlePayment} className="bg-white shadow rounded-lg p-6 space-y-3">
          <h3 className="text-lg font-medium text-gray-900">
            {t('accounting.detail.recordPayment')}
          </h3>
          <p className="text-sm text-gray-600">
            {t('accounting.detail.remaining', {
              amount: formatAmount(remaining),
              currency: invoice.currency,
            })}
          </p>
          <div className="flex items-end gap-3">
            <label className="flex-1 text-sm text-gray-700">
              {t('accounting.detail.paymentAmount')}
              <input
                type="number"
                step="0.01"
                min="0"
                value={payment}
                onChange={(e) => setPayment(e.target.value)}
                className="mt-1 block w-full border border-gray-300 rounded-md shadow-sm p-2"
              />
            </label>
            <button
              type="submit"
              disabled={isUpdating}
              className="bg-green-600 text-white px-4 py-2 rounded-md hover:bg-green-700 disabled:opacity-50"
            >
              {t('accounting.detail.recordPayment')}
            </button>
          </div>
          {paymentError && <p className="text-sm text-red-600">{paymentError}</p>}
        </form>
      )}
    </div>
  );
}
