import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import { MfaWindowChip } from './MfaWindowChip';

describe('MfaWindowChip', () => {
  // --- State rendering ---

  it('renders unverified state with correct text', () => {
    render(<MfaWindowChip state="unverified" />);
    expect(screen.getByText('MFA · not verified')).toBeDefined();
  });

  it('renders verified state with MM:SS formatted time', () => {
    // 5 minutes 30 seconds = 330000 ms
    render(<MfaWindowChip state="verified" msRemaining={330_000} />);
    expect(screen.getByText('MFA verified · 05:30')).toBeDefined();
  });

  it('renders verified state with zero-padded minutes', () => {
    // 10 minutes = 600000 ms
    render(<MfaWindowChip state="verified" msRemaining={600_000} />);
    expect(screen.getByText('MFA verified · 10:00')).toBeDefined();
  });

  it('renders expiring state with M:SS formatted time (no leading zero on minutes)', () => {
    // 1 minute 5 seconds = 65000 ms
    render(<MfaWindowChip state="expiring" msRemaining={65_000} />);
    expect(screen.getByText('MFA expires · 1:05')).toBeDefined();
  });

  it('renders expiring state at 0 seconds remaining', () => {
    render(<MfaWindowChip state="expiring" msRemaining={0} />);
    expect(screen.getByText('MFA expires · 0:00')).toBeDefined();
  });

  // --- Aria labels ---

  it('has correct aria-label for unverified', () => {
    render(<MfaWindowChip state="unverified" />);
    const el = screen.getByRole('button');
    expect(el.getAttribute('aria-label')).toContain('MFA not verified');
  });

  it('has correct aria-label for verified', () => {
    render(<MfaWindowChip state="verified" msRemaining={300_000} />);
    const el = screen.getByRole('button');
    expect(el.getAttribute('aria-label')).toContain('MFA verified');
    expect(el.getAttribute('aria-label')).toContain('05:00');
  });

  it('has correct aria-label for expiring', () => {
    render(<MfaWindowChip state="expiring" msRemaining={90_000} />);
    const el = screen.getByRole('button');
    expect(el.getAttribute('aria-label')).toContain('expiring');
    expect(el.getAttribute('aria-label')).toContain('1:30');
  });

  // --- Pulsing dot ---

  it('does not render dot for unverified state', () => {
    const { container } = render(<MfaWindowChip state="unverified" />);
    expect(container.querySelector('.ppt-mfa-chip__dot')).toBeNull();
  });

  it('renders dot for verified state', () => {
    const { container } = render(<MfaWindowChip state="verified" msRemaining={300_000} />);
    expect(container.querySelector('.ppt-mfa-chip__dot')).not.toBeNull();
  });

  it('renders dot for expiring state', () => {
    const { container } = render(<MfaWindowChip state="expiring" msRemaining={60_000} />);
    expect(container.querySelector('.ppt-mfa-chip__dot')).not.toBeNull();
  });

  // --- Click handler ---

  it('calls onClick when clicked', async () => {
    const user = userEvent.setup();
    const handler = vi.fn();
    render(<MfaWindowChip state="unverified" onClick={handler} />);
    await user.click(screen.getByRole('button'));
    expect(handler).toHaveBeenCalledTimes(1);
  });

  it('does not throw when clicked without onClick', async () => {
    const user = userEvent.setup();
    render(<MfaWindowChip state="unverified" />);
    // Should not throw
    await user.click(screen.getByRole('button'));
  });

  it('calls onClick on Enter key press', async () => {
    const user = userEvent.setup();
    const handler = vi.fn();
    render(<MfaWindowChip state="verified" msRemaining={300_000} onClick={handler} />);
    const btn = screen.getByRole('button');
    btn.focus();
    await user.keyboard('{Enter}');
    expect(handler).toHaveBeenCalledTimes(1);
  });

  // --- CSS classes ---

  it('applies correct CSS class for each state', () => {
    const { rerender, container } = render(<MfaWindowChip state="unverified" />);
    expect(container.querySelector('.ppt-mfa-chip--unverified')).not.toBeNull();

    rerender(<MfaWindowChip state="verified" msRemaining={300_000} />);
    expect(container.querySelector('.ppt-mfa-chip--verified')).not.toBeNull();

    rerender(<MfaWindowChip state="expiring" msRemaining={60_000} />);
    expect(container.querySelector('.ppt-mfa-chip--expiring')).not.toBeNull();
  });

  it('adds clickable class when onClick provided', () => {
    const { container } = render(<MfaWindowChip state="unverified" onClick={() => {}} />);
    expect(container.querySelector('.ppt-mfa-chip--clickable')).not.toBeNull();
  });

  it('does not add clickable class when onClick absent', () => {
    const { container } = render(<MfaWindowChip state="unverified" />);
    expect(container.querySelector('.ppt-mfa-chip--clickable')).toBeNull();
  });

  // --- Time formatting edge cases ---

  it('rounds up partial seconds in verified label', () => {
    // 5001 ms → ceil(5.001) = 6 seconds
    render(<MfaWindowChip state="verified" msRemaining={5_001} />);
    expect(screen.getByText('MFA verified · 00:06')).toBeDefined();
  });

  it('shows 00:01 for verified at exactly 1000ms', () => {
    render(<MfaWindowChip state="verified" msRemaining={1_000} />);
    expect(screen.getByText('MFA verified · 00:01')).toBeDefined();
  });
});
