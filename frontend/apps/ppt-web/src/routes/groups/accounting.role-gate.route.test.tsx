/**
 * Accounting is operator tooling: residents and anonymous visitors must get
 * the Access-Denied / login surface, not the invoice and bank-statement pages
 * (which previously rendered for anyone and only failed with API 403s).
 */
import { render, screen } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { describe, expect, it, vi } from 'vitest';
import type { AuthContextValue } from '../../contexts/AuthContext';
import { accountingRoutes } from './accounting';

const authState = vi.hoisted(() => ({ current: null as unknown as AuthContextValue }));

vi.mock('../../contexts/AuthContext', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../contexts/AuthContext')>();
  return { ...actual, useAuth: () => authState.current };
});

vi.mock('../lazyRoutes', () => ({
  AccountingInvoiceManagementPage: () => <div data-testid="invoices-page" />,
  PaymentMatchingPage: () => <div data-testid="matching-page" />,
}));

function authed(role: string | null): AuthContextValue {
  return {
    login: vi.fn(),
    loginWithSsoCode: vi.fn(),
    logout: vi.fn(),
    refreshToken: vi.fn(),
    getAccessToken: () => null,
    setUser: vi.fn(),
    isAuthenticated: role !== null,
    isLoading: false,
    user: role === null ? null : ({ id: 'u1', email: 'x@y.z', role } as AuthContextValue['user']),
  };
}

function renderAt(entry: string) {
  return render(
    <MemoryRouter initialEntries={[entry]}>
      <Routes>
        <Route path="/login" element={<div data-testid="login-page" />} />
        {accountingRoutes()}
      </Routes>
    </MemoryRouter>
  );
}

describe('accounting routes role gate', () => {
  it.each(['tenant', 'owner', 'resident'])('denies the %s role', (role) => {
    authState.current = authed(role);
    renderAt('/accounting/invoices');
    expect(screen.queryByTestId('invoices-page')).toBeNull();
  });

  it('sends anonymous visitors to login', () => {
    authState.current = authed(null);
    renderAt('/accounting/payment-matching');
    expect(screen.getByTestId('login-page')).toBeTruthy();
  });

  it('admits a manager', () => {
    authState.current = authed('manager');
    renderAt('/accounting/invoices');
    expect(screen.getByTestId('invoices-page')).toBeTruthy();
  });
});
