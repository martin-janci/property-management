/// <reference types="vitest/globals" />
/**
 * TwoFactorAuthPage recovery-codes UI tests (Epic 9, Story 9.2 — gap-9-2).
 *
 * Covers the recovery-codes management surface:
 *   - the 10 codes are listed after a successful enable,
 *   - "Copy all" writes the whole set to the clipboard,
 *   - the exhausted-codes warning shows when backupCodesRemaining === 0,
 *   - the low-codes warning shows below the threshold,
 *   - regenerate surfaces the new codes.
 */

import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { TwoFactorAuthPage } from './TwoFactorAuthPage';

// ── Mocks ──

const showToast = vi.fn();

vi.mock('../../../components/Toast', () => ({
  useToast: () => ({ showToast }),
}));

vi.mock('../../../contexts/AuthContext', () => ({
  useAuth: () => ({ isAuthenticated: true, isLoading: false }),
}));

vi.mock('qrcode', () => ({
  default: { toCanvas: vi.fn().mockResolvedValue(undefined) },
}));

vi.mock('@ppt/api-client', () => ({
  useMfaStatus: vi.fn(),
  useMfaSetup: vi.fn(),
  useMfaVerify: vi.fn(),
  useMfaDisable: vi.fn(),
  useMfaRegenerateBackupCodes: vi.fn(),
}));

import {
  useMfaDisable,
  useMfaRegenerateBackupCodes,
  useMfaSetup,
  useMfaStatus,
  useMfaVerify,
} from '@ppt/api-client';

const mockStatus = vi.mocked(useMfaStatus);
const mockSetup = vi.mocked(useMfaSetup);
const mockVerify = vi.mocked(useMfaVerify);
const mockDisable = vi.mocked(useMfaDisable);
const mockRegen = vi.mocked(useMfaRegenerateBackupCodes);

const CODES = Array.from({ length: 10 }, (_, i) => `CODE-${String(i).padStart(4, '0')}`);

// biome-ignore lint/suspicious/noExplicitAny: test stubs intentionally partial
function mut(overrides: Record<string, unknown> = {}): any {
  return { mutateAsync: vi.fn(), isPending: false, data: undefined, ...overrides };
}

// biome-ignore lint/suspicious/noExplicitAny: test stubs intentionally partial
function status(data: Record<string, unknown> | undefined, isLoading = false): any {
  return { data, isLoading };
}

beforeEach(() => {
  vi.clearAllMocks();
  mockSetup.mockReturnValue(mut());
  mockVerify.mockReturnValue(mut());
  mockDisable.mockReturnValue(mut());
  mockRegen.mockReturnValue(mut());
  // jsdom has no clipboard by default.
  Object.assign(navigator, {
    clipboard: { writeText: vi.fn().mockResolvedValue(undefined) },
  });
});

describe('TwoFactorAuthPage — recovery codes', () => {
  it('shows the 10 recovery codes after enabling MFA and copies them all', async () => {
    mockStatus.mockReturnValue(status({ enabled: false, backupCodesRemaining: 0 }));
    mockVerify.mockReturnValue(
      mut({ mutateAsync: vi.fn().mockResolvedValue({ enabled: true, recoveryCodes: CODES }) })
    );
    mockSetup.mockReturnValue(mut({ data: { secret: 'SECRET', qrUri: 'otpauth://x' } }));

    render(<TwoFactorAuthPage />);

    // Start setup, then verify.
    fireEvent.click(screen.getByText('Set up two-factor authentication'));
    // setup mutation has data, page renders the verify form.
    const input = await screen.findByLabelText('Verification code');
    fireEvent.change(input, { target: { value: '123456' } });
    fireEvent.click(screen.getByText('Verify and enable'));

    // All 10 codes should be rendered.
    await waitFor(() => {
      expect(screen.getByText('CODE-0000')).toBeInTheDocument();
    });
    for (const c of CODES) {
      expect(screen.getByText(c)).toBeInTheDocument();
    }

    // Copy all writes the newline-joined codes to the clipboard.
    fireEvent.click(screen.getByText('Copy all'));
    await waitFor(() => {
      expect(navigator.clipboard.writeText).toHaveBeenCalledWith(CODES.join('\n'));
    });
  });

  it('shows the exhausted-codes warning when no codes remain', () => {
    mockStatus.mockReturnValue(status({ enabled: true, backupCodesRemaining: 0 }));
    render(<TwoFactorAuthPage />);
    expect(screen.getByRole('alert')).toHaveTextContent('no recovery codes left');
    expect(screen.getByText('Regenerate recovery codes')).toBeInTheDocument();
  });

  it('shows the low-codes warning below the threshold', () => {
    mockStatus.mockReturnValue(status({ enabled: true, backupCodesRemaining: 2 }));
    render(<TwoFactorAuthPage />);
    expect(screen.getByText(/Only 2 recovery codes left/)).toBeInTheDocument();
  });

  it('does not warn when plenty of codes remain', () => {
    mockStatus.mockReturnValue(status({ enabled: true, backupCodesRemaining: 9 }));
    render(<TwoFactorAuthPage />);
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    expect(screen.queryByText(/recovery codes left/)).not.toBeInTheDocument();
    expect(screen.getByText(/Recovery codes remaining: 9/)).toBeInTheDocument();
  });

  it('surfaces newly regenerated codes', async () => {
    mockStatus.mockReturnValue(status({ enabled: true, backupCodesRemaining: 1 }));
    mockRegen.mockReturnValue(
      mut({ mutateAsync: vi.fn().mockResolvedValue({ backupCodes: CODES, message: 'ok' }) })
    );
    render(<TwoFactorAuthPage />);

    fireEvent.click(screen.getByText('Regenerate recovery codes'));
    const input = await screen.findByLabelText('Authenticator code');
    fireEvent.change(input, { target: { value: '654321' } });
    fireEvent.click(
      screen.getByText('Regenerate recovery codes', { selector: 'button[type="submit"]' })
    );

    await waitFor(() => {
      expect(screen.getByText('CODE-0009')).toBeInTheDocument();
    });
  });
});
