/**
 * Regression test for AccountingInvoiceForm NaN line-total guard.
 *
 * Clearing the Qty (or Unit Price) number input makes `parseFloat('')` return
 * `NaN`. That `NaN` lands in item state and the old `handleSubmit` guard — which
 * only checks for empty required fields — let it through, so `onSubmit` fired
 * with a NaN quantity and the persisted invoice carried NaN line totals.
 *
 * This test clears the Qty field and submits. It fails on `dev` (onSubmit is
 * called with `qty: NaN`) and passes once the parse is guarded so a non-finite
 * quantity/unit price can never reach `onSubmit`.
 */

import type { AccountingContact } from '@ppt/api-client';
/// <reference types="vitest/globals" />
import { fireEvent, render } from '@testing-library/react';
import { AccountingInvoiceForm } from './AccountingInvoiceForm';

const contacts = [{ id: 'contact-1', name: 'Acme s.r.o.' }] as AccountingContact[];

describe('AccountingInvoiceForm — NaN line-total guard', () => {
  it('never submits a non-finite qty/unitPrice when the qty field is cleared', () => {
    const onSubmit = vi.fn();
    const { container } = render(
      <AccountingInvoiceForm contacts={contacts} onSubmit={onSubmit} onCancel={() => {}} />
    );

    // Fill the required fields so the submit guard does not short-circuit.
    const contactSelect = container.querySelector('select') as HTMLSelectElement;
    fireEvent.change(contactSelect, { target: { value: 'contact-1' } });

    const numberInput = container.querySelector('input[type="text"]') as HTMLInputElement;
    fireEvent.change(numberInput, { target: { value: 'INV-100' } });

    const dateInputs = Array.from(
      container.querySelectorAll('input[type="date"]')
    ) as HTMLInputElement[];
    const dueDate = dateInputs.find((d) => d.value === '') ?? dateInputs[dateInputs.length - 1];
    fireEvent.change(dueDate, { target: { value: '2026-12-31' } });

    // Give the single line item a description so it survives the submit filter.
    const descInput = container.querySelector(
      'input[placeholder="Item description"]'
    ) as HTMLInputElement;
    fireEvent.change(descInput, { target: { value: 'Consulting' } });

    // Clear the Qty field — parseFloat('') === NaN.
    const qtyInput = container.querySelectorAll('input[type="number"]')[0] as HTMLInputElement;
    fireEvent.change(qtyInput, { target: { value: '' } });

    fireEvent.submit(container.querySelector('form') as HTMLFormElement);

    // The guard must ensure no non-finite qty/unitPrice ever reaches onSubmit.
    const nanSubmitted = onSubmit.mock.calls.some(([payload]) =>
      (payload.items as { qty: number; unitPrice: number }[]).some(
        (i) => !Number.isFinite(i.qty) || !Number.isFinite(i.unitPrice)
      )
    );
    expect(nanSubmitted).toBe(false);
  });
});
