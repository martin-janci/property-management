/**
 * MfaChallengeModal — behaviour tests (N9).
 *
 * The modal lives in `@ppt/admin-ui` but has no test harness wired in
 * that package; ppt-web already has Vitest + Testing Library so we
 * exercise it from here. Covers the three states the api-client cares
 * about:
 *
 *   1. opens, accepts a 6-digit code, calls onSuccess on a passing verifier.
 *   2. shows an error and stays open when the verifier returns false.
 *   3. cancel button calls onCancel.
 *
 * The TOTP verify endpoint itself (`POST /api/v1/auth/mfa/verify`) is
 * stubbed via the injected `onVerify` prop — the modal never touches
 * fetch directly.
 */

import { MfaChallengeModal } from '@ppt/admin-ui';
/// <reference types="vitest/globals" />
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

describe('MfaChallengeModal', () => {
  it('does not render when closed', () => {
    render(
      <MfaChallengeModal
        open={false}
        onVerify={async () => true}
        onSuccess={() => undefined}
        onCancel={() => undefined}
      />
    );
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('verifies a valid code and calls onSuccess', async () => {
    const user = userEvent.setup();
    const onVerify = vi.fn().mockResolvedValue(true);
    const onSuccess = vi.fn();
    render(
      <MfaChallengeModal
        open
        onVerify={onVerify}
        onSuccess={onSuccess}
        onCancel={() => undefined}
        actionLabel="Suspend agency"
      />
    );
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(screen.getByText(/Suspend agency/)).toBeInTheDocument();

    await user.type(screen.getByLabelText(/verification code/i), '123456');
    await user.click(screen.getByRole('button', { name: /verify/i }));

    await waitFor(() => expect(onVerify).toHaveBeenCalledWith('123456'));
    await waitFor(() => expect(onSuccess).toHaveBeenCalledTimes(1));
  });

  it('shows an error when the verifier returns false', async () => {
    const user = userEvent.setup();
    const onVerify = vi.fn().mockResolvedValue(false);
    const onSuccess = vi.fn();
    render(
      <MfaChallengeModal
        open
        onVerify={onVerify}
        onSuccess={onSuccess}
        onCancel={() => undefined}
      />
    );
    await user.type(screen.getByLabelText(/verification code/i), '999999');
    await user.click(screen.getByRole('button', { name: /verify/i }));

    expect(await screen.findByRole('alert')).toHaveTextContent(/did not work/i);
    expect(onSuccess).not.toHaveBeenCalled();
  });

  it('rejects a non-6-digit code without calling the verifier', async () => {
    const user = userEvent.setup();
    const onVerify = vi.fn();
    render(
      <MfaChallengeModal
        open
        onVerify={onVerify}
        onSuccess={() => undefined}
        onCancel={() => undefined}
      />
    );
    await user.type(screen.getByLabelText(/verification code/i), '12');
    await user.click(screen.getByRole('button', { name: /verify/i }));

    expect(onVerify).not.toHaveBeenCalled();
    expect(screen.getByRole('alert')).toHaveTextContent(/6-digit/);
  });

  it('cancel button invokes onCancel', async () => {
    const user = userEvent.setup();
    const onCancel = vi.fn();
    render(
      <MfaChallengeModal
        open
        onVerify={async () => true}
        onSuccess={() => undefined}
        onCancel={onCancel}
      />
    );
    await user.click(screen.getByRole('button', { name: /cancel/i }));
    expect(onCancel).toHaveBeenCalledTimes(1);
  });
});
