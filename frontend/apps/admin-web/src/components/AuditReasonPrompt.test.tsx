import { render, renderHook, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { AuditReasonPrompt, useAuditReasonValid } from './AuditReasonPrompt';

// ---------------------------------------------------------------------------
// useAuditReasonValid hook tests
// ---------------------------------------------------------------------------

describe('useAuditReasonValid', () => {
  it('returns false when value is empty', () => {
    const { result } = renderHook(() => useAuditReasonValid('', 20));
    expect(result.current).toBe(false);
  });

  it('returns false when value is below minLength', () => {
    const { result } = renderHook(() => useAuditReasonValid('short', 20));
    expect(result.current).toBe(false);
  });

  it('returns true when value meets minLength (varied chars)', () => {
    // 'abcdefghij...' repeated — varied chars, not single repeated
    const varied = 'abcdefghijklmnopqrst'; // 20 varied chars
    const { result } = renderHook(() => useAuditReasonValid(varied, 20));
    expect(result.current).toBe(true);
  });

  it('returns true when value exceeds minLength (varied chars)', () => {
    const varied = 'The reason for this change is to enable rollout to cohort A.';
    const { result } = renderHook(() => useAuditReasonValid(varied, 20));
    expect(result.current).toBe(true);
  });

  it('returns false for single repeated character (meets length)', () => {
    const { result } = renderHook(() => useAuditReasonValid('a'.repeat(25), 20));
    expect(result.current).toBe(false);
  });

  it('returns false for single repeated character (any char)', () => {
    const { result } = renderHook(() => useAuditReasonValid('x'.repeat(30), 20));
    expect(result.current).toBe(false);
  });

  it('returns true for a varied string meeting minLength', () => {
    const { result } = renderHook(() =>
      useAuditReasonValid('Reason for toggling this flag: rollout complete', 20)
    );
    expect(result.current).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// AuditReasonPrompt component tests
// ---------------------------------------------------------------------------

describe('AuditReasonPrompt', () => {
  const defaultProps = {
    action: 'feature_flag_toggle' as const,
    value: '',
    onChange: vi.fn(),
  };

  // --- Rendering ---

  it('renders the "Audit reason" label', () => {
    render(<AuditReasonPrompt {...defaultProps} />);
    expect(screen.getByText('Audit reason')).toBeDefined();
  });

  it('renders textarea', () => {
    render(<AuditReasonPrompt {...defaultProps} />);
    expect(screen.getByRole('textbox', { name: 'Audit reason' })).toBeDefined();
  });

  // --- Char counter ---

  it('shows counter as "0 / 20" for feature_flag_toggle (default minLength 20)', () => {
    render(<AuditReasonPrompt {...defaultProps} value="" />);
    expect(screen.getByText('0 / 20')).toBeDefined();
  });

  it('updates counter as value changes', () => {
    const { rerender } = render(<AuditReasonPrompt {...defaultProps} value="" />);
    expect(screen.getByText('0 / 20')).toBeDefined();

    rerender(<AuditReasonPrompt {...defaultProps} value="hello" />);
    expect(screen.getByText('5 / 20')).toBeDefined();
  });

  it('uses custom minLength in counter', () => {
    render(<AuditReasonPrompt {...defaultProps} action="tenant_purge" minLength={15} value="" />);
    expect(screen.getByText('0 / 15')).toBeDefined();
  });

  it('shows counter "30 / 30" when value meets default purge minLength', () => {
    render(
      <AuditReasonPrompt
        {...defaultProps}
        action="tenant_purge"
        value={'x'.repeat(10) + 'y'.repeat(20)}
      />
    );
    expect(screen.getByText('30 / 30')).toBeDefined();
  });

  // --- Validation hints ---

  it('shows "Needs N more characters" hint when below minLength and has typed', async () => {
    render(<AuditReasonPrompt {...defaultProps} value="short" />);
    expect(screen.getByText('Needs 15 more characters')).toBeDefined();
  });

  it('uses singular "character" when 1 more needed', () => {
    // 19 chars when minLength is 20 → "Needs 1 more character"
    render(<AuditReasonPrompt {...defaultProps} value={`${'a'.repeat(18)}b`} />);
    expect(screen.getByText('Needs 1 more character')).toBeDefined();
  });

  it('shows no hint when empty (user has not started typing)', () => {
    render(<AuditReasonPrompt {...defaultProps} value="" />);
    expect(screen.queryByRole('alert')).toBeNull();
  });

  it('shows single repeated char hint when value is all same character', () => {
    render(<AuditReasonPrompt {...defaultProps} value={'a'.repeat(25)} />);
    expect(screen.getByText("Reason can't be a single repeated character")).toBeDefined();
  });

  it('shows no hint when value is valid', () => {
    const validValue = 'Reason for toggling flag: deploy cohort A complete';
    render(<AuditReasonPrompt {...defaultProps} value={validValue} />);
    expect(screen.queryByRole('alert')).toBeNull();
  });

  // --- Risk badge color ---

  it('shows "High risk" badge for tenant_purge (red)', () => {
    render(<AuditReasonPrompt {...defaultProps} action="tenant_purge" />);
    const badge = screen.getByText('High risk');
    expect(badge).toBeDefined();
    expect(badge.className).toContain('ppt-arp__badge--red');
  });

  it('shows "High risk" badge for principal_kind_escalate (red)', () => {
    render(<AuditReasonPrompt {...defaultProps} action="principal_kind_escalate" />);
    const badge = screen.getByText('High risk');
    expect(badge.className).toContain('ppt-arp__badge--red');
  });

  it('shows "Medium risk" badge for feature_flag_toggle (amber)', () => {
    render(<AuditReasonPrompt {...defaultProps} action="feature_flag_toggle" />);
    const badge = screen.getByText('Medium risk');
    expect(badge.className).toContain('ppt-arp__badge--amber');
  });

  it('shows "Medium risk" badge for maintenance_mode_on (amber)', () => {
    render(<AuditReasonPrompt {...defaultProps} action="maintenance_mode_on" />);
    const badge = screen.getByText('Medium risk');
    expect(badge.className).toContain('ppt-arp__badge--amber');
  });

  it('shows "Medium risk" badge for force_update_floor_bump (amber)', () => {
    render(<AuditReasonPrompt {...defaultProps} action="force_update_floor_bump" />);
    const badge = screen.getByText('Medium risk');
    expect(badge.className).toContain('ppt-arp__badge--amber');
  });

  // --- onChange ---

  it('calls onChange when user types', async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<AuditReasonPrompt {...defaultProps} onChange={onChange} />);
    const ta = screen.getByRole('textbox', { name: 'Audit reason' });
    await user.type(ta, 'hello');
    expect(onChange).toHaveBeenCalled();
  });

  // --- Hidden validity input ---

  it('hidden input has data-valid=false when value invalid', () => {
    const { container } = render(<AuditReasonPrompt {...defaultProps} value="short" />);
    const hidden = container.querySelector('input[type="hidden"]') as HTMLInputElement;
    expect(hidden).not.toBeNull();
    expect(hidden.dataset.valid).toBe('false');
  });

  it('hidden input has data-valid=true when value valid', () => {
    const validValue = 'A clear and concise reason that meets the minimum';
    const { container } = render(<AuditReasonPrompt {...defaultProps} value={validValue} />);
    const hidden = container.querySelector('input[type="hidden"]') as HTMLInputElement;
    expect(hidden.dataset.valid).toBe('true');
  });

  // --- Per-action default minLengths ---

  it('uses minLength 30 for tenant_purge', () => {
    render(<AuditReasonPrompt {...defaultProps} action="tenant_purge" value="" />);
    expect(screen.getByText('0 / 30')).toBeDefined();
  });

  it('uses minLength 20 for maintenance_mode_on', () => {
    render(<AuditReasonPrompt {...defaultProps} action="maintenance_mode_on" value="" />);
    expect(screen.getByText('0 / 20')).toBeDefined();
  });
});
