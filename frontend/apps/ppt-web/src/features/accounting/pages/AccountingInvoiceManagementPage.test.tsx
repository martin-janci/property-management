/**
 * Regression test for AccountingInvoiceManagementPage error surfacing.
 *
 * The create and delete invoice mutations previously had no `onError`, so a
 * failed create/delete failed silently — the user saw nothing. This test pins
 * the fix: both mutations now surface the failure through the toast system
 * (via `parseApiError` + `useToast`). It fails on `dev` (no `onError` handler
 * ⇒ `showToast` is never called) and passes with the handlers wired in.
 */

/// <reference types="vitest/globals" />
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type { ReactElement } from 'react';
import { MemoryRouter } from 'react-router-dom';
import { AccountingInvoiceManagementPage } from './AccountingInvoiceManagementPage';

const invoicesApiList = vi.fn();
const contactsApiList = vi.fn();
const invoicesApiCreate = vi.fn();
const invoicesApiDelete = vi.fn();

vi.mock('../../../hooks/useOrganization', () => ({
  useOrganization: () => ({ organizationId: 'org-1' }),
}));

const invoicesApiGet = vi.fn();
const invoicesApiListItems = vi.fn();
const invoicesApiUpdate = vi.fn();

vi.mock('@ppt/api-client', () => ({
  invoicesApiGet: (args: unknown) => invoicesApiGet(args),
  invoicesApiListItems: (args: unknown) => invoicesApiListItems(args),
  invoicesApiUpdate: (args: unknown) => invoicesApiUpdate(args),
  invoicesApiList: () => invoicesApiList(),
  contactsApiList: () => contactsApiList(),
  invoicesApiCreate: (args: unknown) => invoicesApiCreate(args),
  invoicesApiDelete: (args: unknown) => invoicesApiDelete(args),
}));

const mockShowToast = vi.fn();
// Keep the real ConfirmationDialog (the delete flow now routes through it) and
// only stub useToast so the error-surfacing assertions can observe the call.
vi.mock('../../../components', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../../../components')>()),
  useToast: () => ({ showToast: mockShowToast }),
}));

// Wire shape: snake_case, decimals as strings (rust_decimal).
const invoice = {
  id: 'inv-1',
  contact_id: 'contact-1',
  number: 'INV-001',
  due_date: '2026-09-01',
  issue_date: '2026-08-01',
  currency: 'CZK',
  total_amount: '121.00',
  base_amount: '100.00',
  vat_amount: '21.00',
  paid_amount: '0',
  status: 'issued',
};

const contact = { id: 'contact-1', name: 'Acme s.r.o.' };

function renderPage(): ReactElement {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return (
    <QueryClientProvider client={queryClient}>
      <MemoryRouter>
        <AccountingInvoiceManagementPage />
      </MemoryRouter>
    </QueryClientProvider>
  );
}

describe('AccountingInvoiceManagementPage — error surfacing', () => {
  beforeEach(() => {
    invoicesApiList.mockResolvedValue({ data: [invoice] });
    contactsApiList.mockResolvedValue({ data: [contact] });
    invoicesApiCreate.mockReset();
    invoicesApiDelete.mockReset();
    mockShowToast.mockClear();
  });

  it('shows an error toast when deleting an invoice fails', async () => {
    invoicesApiDelete.mockRejectedValue(new Error('Delete exploded'));
    render(renderPage());

    // Wait for the list to render the invoice row, then click its delete icon.
    await screen.findByText('INV-001');
    // The invoice list is the <ul> that holds the row (the sub-nav is a list too).
    const list = screen.getByText('INV-001').closest('ul') as HTMLElement;
    await userEvent.click(within(list).getByRole('button'));

    // The native confirm() is gone — a ConfirmationDialog opens instead. Confirm
    // the deletion via its danger button to trigger the mutation.
    const dialog = await screen.findByRole('alertdialog');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Delete' }));

    await waitFor(() =>
      expect(mockShowToast).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'error',
          title: 'Failed to delete invoice',
          // The raw client/server `Error.message` ("Delete exploded") is no
          // longer surfaced to the user — `parseApiError` maps it to the safe,
          // localized default copy. The page still owns the error `title`.
          message: 'An unexpected error occurred.',
        })
      )
    );
  });

  it('shows an error toast when creating an invoice fails', async () => {
    invoicesApiCreate.mockRejectedValue(new Error('Create exploded'));
    const { container } = render(renderPage());

    // Open the create form (header "Create Invoice" button).
    await userEvent.click(screen.getByRole('button', { name: /create invoice/i }));
    await screen.findByText('New Invoice');

    // The form labels are not associated with their inputs, so target the
    // required controls by DOM position (contact select, number input, due date).
    const contactSelect = container.querySelector('select') as HTMLSelectElement;
    await userEvent.selectOptions(contactSelect, 'contact-1');

    const numberInput = container.querySelector('input[type="text"]') as HTMLInputElement;
    fireEvent.change(numberInput, { target: { value: 'INV-002' } });

    const dateInputs = Array.from(
      container.querySelectorAll('input[type="date"]')
    ) as HTMLInputElement[];
    const dueDate = dateInputs.find((d) => d.value === '') ?? dateInputs[dateInputs.length - 1];
    fireEvent.change(dueDate, { target: { value: '2026-12-31' } });

    // Give the single line item a description so it survives the form's
    // empty-line-items guard and the create mutation actually fires (the point
    // of this test is the mutation's onError toast, not the client-side guard).
    const descInput = container.querySelector(
      'input[placeholder="Item description"]'
    ) as HTMLInputElement;
    fireEvent.change(descInput, { target: { value: 'Consulting' } });

    // Submit — the in-form submit button is the one with type="submit".
    const submit = screen
      .getAllByRole('button', { name: /create invoice/i })
      .find((b) => (b as HTMLButtonElement).type === 'submit') as HTMLButtonElement;
    await userEvent.click(submit);

    await waitFor(() =>
      expect(mockShowToast).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'error',
          title: 'Failed to create invoice',
          // Raw `Error.message` ("Create exploded") is mapped to the safe,
          // localized default copy rather than shown verbatim to the user.
          message: 'An unexpected error occurred.',
        })
      )
    );
  });

  it('sends the snake_case wire body with the session tenant_id', async () => {
    invoicesApiCreate.mockResolvedValue({ data: invoice });
    const { container } = render(renderPage());
    await userEvent.click(screen.getByRole('button', { name: /create invoice/i }));
    await screen.findByText('New Invoice');
    await userEvent.selectOptions(
      container.querySelector('select') as HTMLSelectElement,
      'contact-1'
    );
    fireEvent.change(container.querySelector('input[type="text"]') as HTMLInputElement, {
      target: { value: 'INV-003' },
    });
    const dateInputs = Array.from(
      container.querySelectorAll('input[type="date"]')
    ) as HTMLInputElement[];
    const dueDate = dateInputs.find((d) => d.value === '') ?? dateInputs[dateInputs.length - 1];
    fireEvent.change(dueDate, { target: { value: '2026-12-31' } });
    fireEvent.change(
      container.querySelector('input[placeholder="Item description"]') as HTMLInputElement,
      { target: { value: 'Consulting' } }
    );
    const submit = screen
      .getAllByRole('button', { name: /create invoice/i })
      .find((b) => (b as HTMLButtonElement).type === 'submit') as HTMLButtonElement;
    await userEvent.click(submit);

    await waitFor(() => expect(invoicesApiCreate).toHaveBeenCalledTimes(1));
    const { body } = invoicesApiCreate.mock.calls[0][0] as { body: Record<string, unknown> };
    expect(body).toMatchObject({
      tenant_id: 'org-1',
      contact_id: 'contact-1',
      number: 'INV-003',
      due_date: '2026-12-31',
      items: [expect.objectContaining({ description: 'Consulting', qty: '1', unit_price: '0' })],
    });
    expect(body).not.toHaveProperty('contactId');
  });

  it('opens the invoice detail and records a full payment as paid', async () => {
    invoicesApiGet.mockResolvedValue({ data: invoice });
    invoicesApiListItems.mockResolvedValue({
      data: [
        {
          id: 'item-1',
          description: 'Consulting',
          qty: '1',
          unit_price: '100.00',
          vat_rate: '21',
          total_amount: '121.00',
        },
      ],
    });
    invoicesApiUpdate.mockResolvedValue({ data: { ...invoice, status: 'paid' } });
    render(renderPage());

    await userEvent.click(await screen.findByText('INV-001'));
    await screen.findByText('Consulting');
    await userEvent.click(
      screen.getAllByRole('button', { name: /record payment/i }).at(-1) as HTMLElement
    );

    await waitFor(() =>
      expect(invoicesApiUpdate).toHaveBeenCalledWith(
        expect.objectContaining({
          path: { id: 'inv-1' },
          body: { paid_amount: '121.00', status: 'paid' },
        })
      )
    );
  });
});
