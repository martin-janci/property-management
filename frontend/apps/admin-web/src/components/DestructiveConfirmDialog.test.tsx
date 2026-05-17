import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { DestructiveConfirmDialog } from './DestructiveConfirmDialog';

const defaultProps = {
  open: true,
  title: 'Delete tenant',
  body: <p>This action is irreversible.</p>,
  confirmText: 'acme-corp',
  onConfirm: vi.fn().mockResolvedValue(undefined),
  onCancel: vi.fn(),
};

/** Returns true when element has a truthy disabled attribute or property. */
function isDisabled(el: HTMLElement): boolean {
  return el.hasAttribute('disabled') || (el as HTMLButtonElement).disabled === true;
}

/** Set the confirm input value directly (avoids slow userEvent.type for multi-element forms). */
function fillConfirmInput(value = 'acme-corp') {
  const input = screen.getByRole('textbox', { name: /type.*to confirm/i });
  fireEvent.change(input, { target: { value } });
}

afterEach(() => {
  vi.clearAllMocks();
  vi.useRealTimers();
});

describe('DestructiveConfirmDialog', () => {
  // --- Rendering ---

  it('renders when open=true', () => {
    render(<DestructiveConfirmDialog {...defaultProps} />);
    expect(screen.getByRole('dialog')).toBeDefined();
  });

  it('does not render when open=false', () => {
    render(<DestructiveConfirmDialog {...defaultProps} open={false} />);
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('renders title in dialog', () => {
    render(<DestructiveConfirmDialog {...defaultProps} />);
    expect(screen.getByText('Delete tenant')).toBeDefined();
  });

  it('renders body content', () => {
    render(<DestructiveConfirmDialog {...defaultProps} />);
    expect(screen.getByText('This action is irreversible.')).toBeDefined();
  });

  it('shows confirmText in label', () => {
    render(<DestructiveConfirmDialog {...defaultProps} />);
    expect(screen.getByText('acme-corp')).toBeDefined();
  });

  // --- Confirm button disabled until match ---

  it('confirm button is disabled initially', () => {
    render(<DestructiveConfirmDialog {...defaultProps} />);
    const btn = screen.getByRole('button', { name: /confirm/i });
    expect(isDisabled(btn)).toBe(true);
  });

  it('confirm button enabled after typing exact confirmText', () => {
    render(<DestructiveConfirmDialog {...defaultProps} />);
    fillConfirmInput('acme-corp');
    const btn = screen.getByRole('button', { name: /confirm/i });
    expect(isDisabled(btn)).toBe(false);
  });

  it('confirm button remains disabled with partial match', () => {
    render(<DestructiveConfirmDialog {...defaultProps} />);
    fillConfirmInput('acme-cor');
    const btn = screen.getByRole('button', { name: /confirm/i });
    expect(isDisabled(btn)).toBe(true);
  });

  it('confirm button is case-sensitive (uppercase rejected)', () => {
    render(<DestructiveConfirmDialog {...defaultProps} />);
    fillConfirmInput('ACME-CORP');
    const btn = screen.getByRole('button', { name: /confirm/i });
    expect(isDisabled(btn)).toBe(true);
  });

  // --- Paste prevention ---

  it('prevents paste on confirm input', () => {
    render(<DestructiveConfirmDialog {...defaultProps} />);
    const input = screen.getByRole('textbox', { name: /type.*to confirm/i });
    // The component binds onPaste={e => e.preventDefault()}.
    // fireEvent.paste returns false when preventDefault was called.
    const prevented = !fireEvent.paste(input);
    expect(prevented).toBe(true);
  });

  // --- onConfirm called ---

  it('calls onConfirm when confirm button clicked with matching text', async () => {
    const onConfirm = vi.fn().mockResolvedValue(undefined);
    render(<DestructiveConfirmDialog {...defaultProps} onConfirm={onConfirm} />);
    fillConfirmInput('acme-corp');
    await act(async () => {
      screen.getByRole('button', { name: /confirm/i }).click();
    });
    expect(onConfirm).toHaveBeenCalledTimes(1);
  });

  // --- onCancel ---

  it('calls onCancel when Cancel button clicked', async () => {
    const onCancel = vi.fn();
    render(<DestructiveConfirmDialog {...defaultProps} onCancel={onCancel} />);
    await act(async () => {
      screen.getByRole('button', { name: /cancel/i }).click();
    });
    expect(onCancel).toHaveBeenCalledTimes(1);
  });

  it('calls onCancel on backdrop click', async () => {
    const user = userEvent.setup();
    const onCancel = vi.fn();
    render(<DestructiveConfirmDialog {...defaultProps} onCancel={onCancel} />);
    const backdrop = document.querySelector('.ppt-dcd-backdrop') as HTMLElement;
    await user.click(backdrop);
    expect(onCancel).toHaveBeenCalledTimes(1);
  });

  it('calls onCancel on ESC key', async () => {
    const user = userEvent.setup();
    const onCancel = vi.fn();
    render(<DestructiveConfirmDialog {...defaultProps} onCancel={onCancel} />);
    await user.keyboard('{Escape}');
    expect(onCancel).toHaveBeenCalledTimes(1);
  });

  // --- Delay countdown ---

  it('shows "Wait Ns" button and is disabled immediately after match with delay', () => {
    vi.useFakeTimers();
    const onConfirm = vi.fn().mockResolvedValue(undefined);

    render(<DestructiveConfirmDialog {...defaultProps} onConfirm={onConfirm} delayMs={3000} />);

    fillConfirmInput('acme-corp');

    // Countdown starts immediately — button should show Wait label
    const btn = screen.getByRole('button', { name: /wait/i });
    expect(btn).toBeDefined();
    expect(isDisabled(btn)).toBe(true);
  });

  it('enables confirm button after delay elapses', async () => {
    // Use real timers with a very short delay to avoid fake-timer/waitFor incompatibility
    const onConfirm = vi.fn().mockResolvedValue(undefined);

    render(<DestructiveConfirmDialog {...defaultProps} onConfirm={onConfirm} delayMs={200} />);

    act(() => {
      fillConfirmInput('acme-corp');
    });

    // Wait for the real 200ms delay + interval ticks to expire
    await waitFor(
      () => {
        const btn = screen.queryByRole('button', { name: /^confirm$/i });
        expect(btn).not.toBeNull();
        if (btn) expect(isDisabled(btn)).toBe(false);
      },
      { timeout: 2000 }
    );
  }, 10000);

  // --- reasonAction embeds AuditReasonPrompt ---

  it('renders AuditReasonPrompt when reasonAction provided', () => {
    render(<DestructiveConfirmDialog {...defaultProps} reasonAction="tenant_purge" />);
    expect(screen.getByText('Audit reason')).toBeDefined();
  });

  it('does not render AuditReasonPrompt when reasonAction absent', () => {
    render(<DestructiveConfirmDialog {...defaultProps} />);
    expect(screen.queryByText('Audit reason')).toBeNull();
  });

  it('keeps confirm disabled when reason is too short even if text matches', () => {
    render(<DestructiveConfirmDialog {...defaultProps} reasonAction="feature_flag_toggle" />);
    // Type the confirm text directly
    fillConfirmInput('acme-corp');

    // Reason textarea is still empty — confirm should stay disabled
    const btn = screen.getByRole('button', { name: /confirm/i });
    expect(isDisabled(btn)).toBe(true);
  });

  // --- Custom confirmLabel ---

  it('uses custom confirmLabel', () => {
    render(<DestructiveConfirmDialog {...defaultProps} confirmLabel="Enter the tenant slug" />);
    expect(screen.getByText('Enter the tenant slug')).toBeDefined();
  });

  it('uses default confirmLabel when not provided', () => {
    render(<DestructiveConfirmDialog {...defaultProps} />);
    expect(screen.getByText('Type the name to confirm')).toBeDefined();
  });

  // --- Spinner during pending ---

  it('shows spinner while onConfirm is in flight', async () => {
    let resolve!: () => void;
    const onConfirm = vi.fn(
      () =>
        new Promise<void>((res) => {
          resolve = res;
        })
    );

    render(<DestructiveConfirmDialog {...defaultProps} onConfirm={onConfirm} />);
    fillConfirmInput('acme-corp');

    act(() => {
      screen.getByRole('button', { name: /confirm/i }).click();
    });

    // Spinner should be visible while pending
    await waitFor(() => {
      expect(document.querySelector('.ppt-dcd-spinner')).not.toBeNull();
    });

    // Resolve the promise
    await act(async () => {
      resolve();
    });
    await waitFor(() => {
      expect(document.querySelector('.ppt-dcd-spinner')).toBeNull();
    });
  });
});
