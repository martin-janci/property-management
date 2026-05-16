/**
 * MfaWrapper — behaviour tests (Phase 6 B6).
 *
 * The wrapper is the enrollment gate in front of the admin sub-tree.
 * It hits GET /api/v1/auth/mfa/status, then either:
 *   - renders children (enrolled), OR
 *   - renders MfaEnrollmentScreen (not enrolled).
 *
 * All network calls are stubbed via `vi.stubGlobal('fetch', ...)`.
 * The `useAuth` hook is mocked so this file has no dependency on real
 * auth context providers.
 */

/// <reference types="vitest/globals" />
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// ---------- Mock useAuth --------------------------------------------------
vi.mock('../../contexts', () => ({
  useAuth: () => ({ getAccessToken: () => 'test-token' }),
}));

// ---------- Mock react-i18next -------------------------------------------
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (_key: string, opts?: { defaultValue?: string }) =>
      opts?.defaultValue ?? _key,
  }),
}));

// ---------- Helpers -------------------------------------------------------

/** Build a Response-like stub — fresh object each call so json() is reusable */
function makeResponse(body: unknown, ok = true): Response {
  return {
    ok,
    status: ok ? 200 : 500,
    json: async () => body,
  } as unknown as Response;
}

function enrollStartResponse(): Response {
  return makeResponse({
    uri: 'otpauth://totp/test%40example.com?secret=ABC&issuer=Test',
    secret: 'ABC123',
  });
}

function codesResponse(): Response {
  return makeResponse({
    message: 'Enrolled',
    recovery_codes: [
      'AAAA1111', 'BBBB2222', 'CCCC3333', 'DDDD4444', 'EEEE5555',
      'FFFF6666', 'GGGG7777', 'HHHH8888', 'IIII9999', 'JJJJ0000',
    ],
  });
}

// ---------- Import after mocks -------------------------------------------
// eslint-disable-next-line import/first
import { MfaWrapper } from './MfaWrapper';

// ---------- Tests ----------------------------------------------------------

describe('MfaWrapper', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('shows a loading state initially', () => {
    // fetch never resolves — keeps component in loading state.
    vi.stubGlobal('fetch', vi.fn().mockReturnValue(new Promise(() => {})));
    render(<MfaWrapper><span>admin content</span></MfaWrapper>);
    expect(screen.getByText(/Loading/i)).toBeInTheDocument();
  });

  it('renders children when MFA is enrolled', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue(makeResponse({ enabled: true }))
    );
    render(<MfaWrapper><span>admin content</span></MfaWrapper>);
    await waitFor(() =>
      expect(screen.getByText('admin content')).toBeInTheDocument()
    );
    expect(screen.queryByText(/two-factor/i)).toBeNull();
  });

  it('renders the enrollment screen when MFA is NOT enrolled', async () => {
    // 1st call → status; 2nd call → enroll/start (from MfaEnrollmentScreen on mount)
    vi.stubGlobal(
      'fetch',
      vi.fn()
        .mockResolvedValueOnce(makeResponse({ enabled: false }))
        .mockResolvedValue(enrollStartResponse())
    );

    render(<MfaWrapper><span>admin content</span></MfaWrapper>);

    await waitFor(() =>
      expect(
        screen.getByText(/Set up two-factor authentication/i)
      ).toBeInTheDocument()
    );

    expect(screen.queryByText('admin content')).toBeNull();
  });

  it('transitions to children after successful enrollment', async () => {
    const user = userEvent.setup();
    vi.stubGlobal(
      'fetch',
      vi.fn()
        // 1) status → not enrolled
        .mockResolvedValueOnce(makeResponse({ enabled: false }))
        // 2) enroll/start
        .mockResolvedValueOnce(enrollStartResponse())
        // 3) enroll/verify
        .mockResolvedValueOnce(codesResponse())
    );

    render(<MfaWrapper><span>admin content</span></MfaWrapper>);

    // Wait for enrollment screen AND the Next button (only visible after enroll/start loads).
    await waitFor(() =>
      expect(screen.getByRole('button', { name: /Next/i })).toBeInTheDocument()
    );

    // Advance to verify step.
    await user.click(screen.getByRole('button', { name: /Next/i }));

    // Type a valid 6-digit code.
    await user.type(screen.getByLabelText(/verification code/i), '123456');
    await user.click(screen.getByRole('button', { name: /Verify/i }));

    // Recovery codes step.
    await waitFor(() =>
      expect(screen.getByText(/Save your recovery codes/i)).toBeInTheDocument()
    );

    // Confirm codes saved.
    await user.click(
      screen.getByRole('button', { name: /I've saved these codes/i })
    );

    // Children now visible.
    await waitFor(() =>
      expect(screen.getByText('admin content')).toBeInTheDocument()
    );
  });

  it('defaults to showing enrollment when the status fetch rejects', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn()
        // 1) status → rejects
        .mockRejectedValueOnce(new Error('network'))
        // 2) enroll/start → succeeds (rendered after fallback)
        .mockResolvedValue(enrollStartResponse())
    );

    render(<MfaWrapper><span>admin content</span></MfaWrapper>);

    await waitFor(() =>
      expect(
        screen.getByText(/Set up two-factor authentication/i)
      ).toBeInTheDocument()
    );
    expect(screen.queryByText('admin content')).toBeNull();
  });
});
