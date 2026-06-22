/**
 * InvoiceList — PDF-download loading state + double-click guard (#1673).
 *
 * PR #1654 wired the per-row "PDF" download button but rendered it as a bare
 * always-enabled action. These tests pin the follow-up behaviour: while a
 * download is in flight (the route container reports the in-flight invoice id
 * via `downloadingPdfId`), that invoice's PDF button must render disabled +
 * busy so a second click cannot fire a concurrent download.
 */

/// <reference types="vitest/globals" />
import type { Invoice } from '@ppt/api-client';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { InvoiceList } from './InvoiceList';

function makeInvoice(overrides: Partial<Invoice> = {}): Invoice {
  return {
    id: 'inv-1',
    organization_id: 'org-1',
    unit_id: 'unit-1',
    invoice_number: 'INV-0001',
    status: 'sent',
    issue_date: '2026-06-01',
    due_date: '2026-06-15',
    subtotal: 100,
    tax_amount: 20,
    total: 120,
    amount_paid: 0,
    balance_due: 120,
    currency: 'EUR',
    created_at: '2026-06-01T00:00:00Z',
    updated_at: '2026-06-01T00:00:00Z',
    ...overrides,
  };
}

const baseProps = {
  total: 1,
  page: 1,
  pageSize: 10,
  onPageChange: vi.fn(),
  onViewInvoice: vi.fn(),
};

describe('InvoiceList — PDF download loading state (#1673)', () => {
  it('renders an enabled PDF button when no download is in flight', () => {
    render(<InvoiceList {...baseProps} invoices={[makeInvoice()]} onDownloadPdf={vi.fn()} />);

    const pdfBtn = screen.getByRole('button', { name: 'PDF' });
    expect(pdfBtn).toBeEnabled();
    expect(pdfBtn).toHaveAttribute('aria-busy', 'false');
  });

  it('disables + marks busy the PDF button of the invoice currently downloading', () => {
    render(
      <InvoiceList
        {...baseProps}
        invoices={[makeInvoice()]}
        onDownloadPdf={vi.fn()}
        downloadingPdfId="inv-1"
      />
    );

    const pdfBtn = screen.getByRole('button', { name: 'PDF…' });
    expect(pdfBtn).toBeDisabled();
    expect(pdfBtn).toHaveAttribute('aria-busy', 'true');
  });

  it('does not invoke onDownloadPdf when the in-flight button is clicked (double-click guard)', async () => {
    const onDownloadPdf = vi.fn();
    render(
      <InvoiceList
        {...baseProps}
        invoices={[makeInvoice()]}
        onDownloadPdf={onDownloadPdf}
        downloadingPdfId="inv-1"
      />
    );

    // userEvent respects the `disabled` attribute and won't dispatch the click.
    await userEvent.click(screen.getByRole('button', { name: 'PDF…' }));
    expect(onDownloadPdf).not.toHaveBeenCalled();
  });

  it('only disables the downloading invoice, leaving sibling PDF buttons active', () => {
    render(
      <InvoiceList
        {...baseProps}
        total={2}
        invoices={[
          makeInvoice({ id: 'inv-1', invoice_number: 'INV-0001' }),
          makeInvoice({ id: 'inv-2', invoice_number: 'INV-0002' }),
        ]}
        onDownloadPdf={vi.fn()}
        downloadingPdfId="inv-1"
      />
    );

    expect(screen.getByRole('button', { name: 'PDF…' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'PDF' })).toBeEnabled();
  });
});
