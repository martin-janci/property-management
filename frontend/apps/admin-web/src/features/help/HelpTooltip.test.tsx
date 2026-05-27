/**
 * HelpTooltip component tests.
 *
 * Covers:
 *   - Renders the trigger button with accessible label
 *   - Tooltip bubble is hidden by default
 *   - Tooltip bubble becomes visible on hover (mouseenter)
 *   - Tooltip bubble becomes visible on focus
 *   - Tooltip bubble hides on blur / mouseleave
 *   - role="tooltip" on the bubble
 *   - aria-describedby links button to bubble
 *   - No duplicate style injection
 */

import { fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string) => fallback ?? key,
  }),
}));

import { HelpTooltip } from './HelpTooltip';

afterEach(() => {
  vi.clearAllMocks();
});

describe('HelpTooltip', () => {
  it('renders a button with aria-label="Help"', () => {
    render(<HelpTooltip text="Tooltip content" />);
    const btn = screen.getByRole('button', { name: /help/i });
    expect(btn).toBeDefined();
  });

  it('tooltip bubble is hidden by default', () => {
    const { container } = render(<HelpTooltip text="Hidden tooltip" />);
    const bubble = container.querySelector('.ppt-help-tip__bubble') as HTMLElement;
    expect(bubble.style.visibility).toBe('hidden');
  });

  it('tooltip bubble becomes visible on mouseenter', () => {
    const { container } = render(<HelpTooltip text="Show on hover" />);
    const btn = screen.getByRole('button', { name: /help/i });
    fireEvent.mouseEnter(btn);
    const bubble = container.querySelector('.ppt-help-tip__bubble') as HTMLElement;
    expect(bubble.style.visibility).toBe('visible');
  });

  it('tooltip bubble hides again on mouseleave', () => {
    const { container } = render(<HelpTooltip text="Hide on leave" />);
    const btn = screen.getByRole('button', { name: /help/i });
    fireEvent.mouseEnter(btn);
    fireEvent.mouseLeave(btn);
    const bubble = container.querySelector('.ppt-help-tip__bubble') as HTMLElement;
    expect(bubble.style.visibility).toBe('hidden');
  });

  it('tooltip bubble becomes visible on focus', async () => {
    const user = userEvent.setup();
    const { container } = render(<HelpTooltip text="Show on focus" />);
    const btn = screen.getByRole('button', { name: /help/i });
    await user.tab(); // move focus to the button
    // Ensure the button received focus
    btn.focus();
    fireEvent.focus(btn);
    const bubble = container.querySelector('.ppt-help-tip__bubble') as HTMLElement;
    expect(bubble.style.visibility).toBe('visible');
  });

  it('tooltip bubble hides on blur', () => {
    const { container } = render(<HelpTooltip text="Hide on blur" />);
    const btn = screen.getByRole('button', { name: /help/i });
    fireEvent.focus(btn);
    fireEvent.blur(btn);
    const bubble = container.querySelector('.ppt-help-tip__bubble') as HTMLElement;
    expect(bubble.style.visibility).toBe('hidden');
  });

  it('bubble has role="tooltip"', () => {
    const { container } = render(<HelpTooltip text="Role check" />);
    const bubble = container.querySelector('[role="tooltip"]');
    expect(bubble).not.toBeNull();
  });

  it('button aria-describedby points to the bubble id', () => {
    render(<HelpTooltip text="Aria check" />);
    const btn = screen.getByRole('button', { name: /help/i });
    const describedBy = btn.getAttribute('aria-describedby');
    expect(describedBy).toBeTruthy();
    const bubble = document.getElementById(describedBy as string);
    expect(bubble).not.toBeNull();
    expect(bubble?.textContent).toBe('Aria check');
  });

  it('renders the tooltip text content', () => {
    const { container } = render(<HelpTooltip text="Important context" />);
    const bubble = container.querySelector('[role="tooltip"]');
    expect(bubble?.textContent).toBe('Important context');
  });

  it('does not inject duplicate style element on multiple mounts', () => {
    const { unmount } = render(<HelpTooltip text="First" />);
    render(<HelpTooltip text="Second" />);
    const styleElements = document.querySelectorAll('#ppt-help-tooltip-styles');
    expect(styleElements.length).toBe(1);
    unmount();
  });
});
