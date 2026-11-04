/// <reference types="vitest/globals" />
import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { AccountingSubNav } from './AccountingSubNav';

vi.mock('react-i18next', () => ({ useTranslation: () => ({ t: (k: string) => k }) }));

describe('AccountingSubNav', () => {
  it('links invoices and payment matching, marking the current tab', () => {
    render(
      <MemoryRouter initialEntries={['/accounting/payment-matching']}>
        <AccountingSubNav />
      </MemoryRouter>
    );
    expect(screen.getByRole('link', { name: 'accounting.nav.invoices' })).toHaveAttribute(
      'href',
      '/accounting/invoices'
    );
    const matching = screen.getByRole('link', { name: 'accounting.nav.paymentMatching' });
    expect(matching).toHaveAttribute('href', '/accounting/payment-matching');
    expect(matching).toHaveAttribute('aria-current', 'page');
  });
});
