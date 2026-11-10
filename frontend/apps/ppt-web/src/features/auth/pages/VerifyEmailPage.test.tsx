/// <reference types="vitest/globals" />
import { AuthError } from '@ppt/api-client';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { StrictMode } from 'react';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { statusFromError, VerifyEmailPage } from './VerifyEmailPage';

const verifyEmail = vi.fn();
const resendVerification = vi.fn();

vi.mock('../authApiClient', () => ({
  getAuthApi: () => ({ verifyEmail, resendVerification }),
}));

function renderAt(url: string) {
  return render(
    <StrictMode>
      <MemoryRouter initialEntries={[url]}>
        <VerifyEmailPage />
      </MemoryRouter>
    </StrictMode>
  );
}

describe('VerifyEmailPage', () => {
  beforeEach(() => {
    verifyEmail.mockReset();
    resendVerification.mockReset();
  });

  it('verifies the token exactly once (single-use token, StrictMode) and offers sign-in', async () => {
    verifyEmail.mockResolvedValue(undefined);
    renderAt('/verify-email?token=abc');
    expect(await screen.findByText('E-mail verified')).toBeInTheDocument();
    expect(verifyEmail).toHaveBeenCalledTimes(1);
    expect(verifyEmail).toHaveBeenCalledWith('abc');
    expect(screen.getByRole('link', { name: 'Sign in' })).toHaveAttribute('href', '/login');
  });

  it('shows the expired state and re-sends a link', async () => {
    verifyEmail.mockRejectedValue(new AuthError('expired', 'TOKEN_EXPIRED'));
    resendVerification.mockResolvedValue(undefined);
    const { container } = renderAt('/verify-email?token=old');
    expect(await screen.findByText(/has expired/)).toBeInTheDocument();

    const input = container.querySelector<HTMLInputElement>('#verify-email-address');
    if (!input) throw new Error('missing email input');
    fireEvent.change(input, { target: { value: 'ada@example.com' } });
    fireEvent.click(screen.getByRole('button', { name: 'Send a new link' }));

    await waitFor(() => expect(resendVerification).toHaveBeenCalledWith('ada@example.com'));
    expect(await screen.findByText(/a new link is on its way/)).toBeInTheDocument();
  });

  it('treats a missing token as an invalid link without calling the API', () => {
    renderAt('/verify-email');
    expect(screen.getByText('Verification failed')).toBeInTheDocument();
    expect(verifyEmail).not.toHaveBeenCalled();
  });
});

describe('statusFromError', () => {
  it('maps the server codes', () => {
    expect(statusFromError(new AuthError('x', 'TOKEN_EXPIRED'))).toBe('expired');
    expect(statusFromError(new AuthError('x', 'INVALID_TOKEN' as never))).toBe('invalid');
    expect(statusFromError(new Error('boom'))).toBe('error');
  });
});
