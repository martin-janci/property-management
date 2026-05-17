import { act, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';

import { AdminAuthProvider, useAdminAuth } from './AdminAuthContext';

function Probe() {
  const auth = useAdminAuth();
  return (
    <div>
      <span data-testid="authed">{String(auth.isAuthenticated)}</span>
      <button type="button" onClick={() => auth.setToken('t')}>
        login
      </button>
      <button type="button" onClick={() => auth.logout()}>
        logout
      </button>
    </div>
  );
}

describe('AdminAuthContext', () => {
  afterEach(() => sessionStorage.clear());

  it('starts unauthenticated', () => {
    render(
      <AdminAuthProvider>
        <Probe />
      </AdminAuthProvider>
    );
    expect(screen.getByTestId('authed').textContent).toBe('false');
  });

  it('becomes authenticated after setToken', async () => {
    render(
      <AdminAuthProvider>
        <Probe />
      </AdminAuthProvider>
    );
    await act(async () => screen.getByText('login').click());
    expect(screen.getByTestId('authed').textContent).toBe('true');
  });

  it('clears on logout', async () => {
    render(
      <AdminAuthProvider>
        <Probe />
      </AdminAuthProvider>
    );
    await act(async () => screen.getByText('login').click());
    await act(async () => screen.getByText('logout').click());
    expect(screen.getByTestId('authed').textContent).toBe('false');
  });
});
