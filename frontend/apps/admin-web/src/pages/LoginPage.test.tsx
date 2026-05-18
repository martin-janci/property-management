import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router-dom';
import { describe, expect, it, vi } from 'vitest';

import { AdminAuthProvider } from '../auth/AdminAuthContext';
import { LoginPage } from './LoginPage';

describe('LoginPage', () => {
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
});
