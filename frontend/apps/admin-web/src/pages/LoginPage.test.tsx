import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { AdminAuthProvider } from '../auth/AdminAuthContext';
import {
  __clearAnalyticsSinks,
  type AnalyticsEvent,
  registerAnalyticsSink,
} from '../lib/analytics';
import { LoginPage } from './LoginPage';

describe('LoginPage', () => {
  let captured: AnalyticsEvent[];

  beforeEach(() => {
    captured = [];
    registerAnalyticsSink((e) => captured.push(e));
  });

  afterEach(() => {
    __clearAnalyticsSinks();
  });

  it('calls login API and stores token on success', async () => {
    const login = vi.fn().mockResolvedValue({ accessToken: 'tk' });
    const user = userEvent.setup();
    render(
      <MemoryRouter>
        <AdminAuthProvider>
          <LoginPage loginFn={login} />
        </AdminAuthProvider>
      </MemoryRouter>
    );
    await user.type(screen.getByLabelText(/email/i), 'admin@example.com');
    await user.type(screen.getByLabelText(/password/i), 'secret');
    await user.click(screen.getByRole('button', { name: /sign in/i }));
    expect(login).toHaveBeenCalledWith({ email: 'admin@example.com', password: 'secret' });
  });

  it('emits signup.logged_in on a successful (non-MFA) login', async () => {
    const login = vi.fn().mockResolvedValue({ accessToken: 'tk' });
    const user = userEvent.setup();
    render(
      <MemoryRouter>
        <AdminAuthProvider>
          <LoginPage loginFn={login} />
        </AdminAuthProvider>
      </MemoryRouter>
    );
    await user.type(screen.getByLabelText(/email/i), 'admin@example.com');
    await user.type(screen.getByLabelText(/password/i), 'secret');
    await user.click(screen.getByRole('button', { name: /sign in/i }));

    const events = captured.filter((e) => e.event === 'signup.logged_in');
    expect(events).toHaveLength(1);
    expect(events[0].properties).toMatchObject({ method: 'email_password' });
  });

  it('does not emit signup.logged_in when the server defers for MFA', async () => {
    const login = vi.fn().mockResolvedValue({ accessToken: '', mfaRequired: true });
    const user = userEvent.setup();
    render(
      <MemoryRouter>
        <AdminAuthProvider>
          <LoginPage loginFn={login} />
        </AdminAuthProvider>
      </MemoryRouter>
    );
    await user.type(screen.getByLabelText(/email/i), 'admin@example.com');
    await user.type(screen.getByLabelText(/password/i), 'secret');
    await user.click(screen.getByRole('button', { name: /sign in/i }));

    expect(captured.some((e) => e.event === 'signup.logged_in')).toBe(false);
  });
});
