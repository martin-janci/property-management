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
import { AccountingInvoiceManagementPage } from './AccountingInvoiceManagementPage';

const invoicesApiList = vi.fn();
const contactsApiList = vi.fn();
const invoicesApiCreate = vi.fn();
const invoicesApiDelete = vi.fn();

vi.mock('@ppt/api-client', () => ({
  invoicesApiList: () => invoicesApiList(),
  contactsApiList: () => contactsApiList(),
  invoicesApiCreate: (args: unknown) => invoicesApiCreate(args),
  invoicesApiDelete: (args: unknown) => invoicesApiDelete(args),
}));

const mockShowToast = vi.fn();
vi.mock('../../../components', () => ({
  useToast: () => ({ showToast: mockShowToast }),
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
};

const contact = { id: 'contact-1', name: 'Acme s.r.o.' };

function renderPage(): ReactElement {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return (
    <QueryClientProvider client={queryClient}>
      <AccountingInvoiceManagementPage />
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
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);
    try {
      render(renderPage());

      // Wait for the list to render the invoice row, then click its delete icon.
      await screen.findByText('INV-001');
      const list = screen.getByRole('list');
      await userEvent.click(within(list).getByRole('button'));

      await waitFor(() =>
        expect(mockShowToast).toHaveBeenCalledWith(
          expect.objectContaining({
            type: 'error',
            title: 'Failed to delete invoice',
            message: 'Delete exploded',
          })
        )
      );
    } finally {
      confirmSpy.mockRestore();
    }
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
          message: 'Create exploded',
        })
      )
    );
  });
});
