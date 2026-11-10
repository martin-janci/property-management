/// <reference types="vitest/globals" />
/**
 * AccountingInvoiceList row interactivity (nav-fixes): accounting-server
 * invoices have no detail view yet, so without `onViewInvoice` rows must not
 * look clickable (previously a silent no-op handler).
 */
import type { AccountingInvoice } from '@ppt/api-client';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { AccountingInvoiceList } from './AccountingInvoiceList';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (k: string) => k, i18n: { language: 'en' } }),
}));

const invoice = {
  id: 'inv-1',
  number: 'INV-001',
  dueDate: '2026-09-01',
  issueDate: '2026-08-01',
  currency: 'CZK',
  totalAmount: 121,
  vatAmount: 21,
  status: 'issued',
} as unknown as AccountingInvoice;

function row() {
  return screen.getByText('INV-001').closest('li')?.firstElementChild as HTMLElement;
}

describe('AccountingInvoiceList', () => {
  it('rows are not clickable when no onViewInvoice is given', () => {
    render(<AccountingInvoiceList invoices={[invoice]} />);
    expect(row().className).not.toContain('cursor-pointer');
  });

  it('rows call onViewInvoice when provided', async () => {
    const onView = vi.fn();
    render(<AccountingInvoiceList invoices={[invoice]} onViewInvoice={onView} />);
    expect(row().className).toContain('cursor-pointer');
    await userEvent.click(row());
    expect(onView).toHaveBeenCalledWith('inv-1');
  });
});
