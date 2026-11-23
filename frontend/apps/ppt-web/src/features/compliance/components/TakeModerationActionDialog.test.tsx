/// <reference types="vitest/globals" />
/**
 * TakeModerationActionDialog narrowing-guard regression test.
 *
 * History: the action-type `<select>` previously fed its value into the
 * `TakeModerationActionRequest['action_type']` union via an unchecked
 * `e.target.value as ModerationActionType` cast. The `<select>` only renders
 * the four valid options, but `HTMLSelectElement.value` is a plain `string`, so
 * the cast silently trusted whatever reached the handler — a DOM/markup change
 * could push an out-of-union value straight into the regulated moderation
 * payload and the compliance audit trail.
 *
 * The handler now narrows with the runtime guard `isModerationActionType` and
 * ignores values outside the union. These tests lock the guard's contract in
 * (it rejects out-of-union strings) and the happy-path submission.
 */
import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { isModerationActionType, TakeModerationActionDialog } from './TakeModerationActionDialog';

describe('isModerationActionType narrowing guard', () => {
  it('accepts every member of the action_type union', () => {
    for (const valid of ['approve', 'warn', 'restrict', 'remove']) {
      expect(isModerationActionType(valid)).toBe(true);
    }
  });

  it('rejects values outside the union (the cast risk)', () => {
    // `aprove` is the canonical free-text typo the old cast would have let
    // through into the moderation payload; empty/unknown values must be
    // rejected too.
    for (const invalid of ['aprove', 'Approve', 'ignore', 'escalate', 'delete', '']) {
      expect(isModerationActionType(invalid)).toBe(false);
    }
  });
});

describe('TakeModerationActionDialog submission', () => {
  it('submits the selected valid union value with the trimmed rationale', () => {
    const onSubmit = vi.fn();
    render(<TakeModerationActionDialog isOpen onSubmit={onSubmit} onClose={() => {}} />);

    fireEvent.change(screen.getByRole('combobox'), { target: { value: 'remove' } });
    fireEvent.change(screen.getByRole('textbox'), { target: { value: '  spam listing  ' } });
    fireEvent.click(screen.getByRole('button', { name: /submit action/i }));

    expect(onSubmit).toHaveBeenCalledTimes(1);
    expect(onSubmit).toHaveBeenCalledWith('remove', 'spam listing', true);
  });

  it('does not submit when the rationale is empty', () => {
    const onSubmit = vi.fn();
    render(<TakeModerationActionDialog isOpen onSubmit={onSubmit} onClose={() => {}} />);

    fireEvent.click(screen.getByRole('button', { name: /submit action/i }));

    expect(onSubmit).not.toHaveBeenCalled();
  });
});
