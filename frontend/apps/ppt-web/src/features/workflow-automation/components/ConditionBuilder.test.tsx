/// <reference types="vitest/globals" />
/**
 * ConditionBuilder number-input zero-coercion regression tests.
 *
 * History: the numeric value input rendered `value={condition.value as number}`
 * with `onChange={… Number(e.target.value) …}`. Because `Number('') === 0`,
 * clearing the field silently coerced it to 0 — an empty field and a legitimate
 * 0 became indistinguishable, and a cleared input snapped back to "0".
 *
 * These tests lock in the fixed contract: a typed 0 is preserved as the number
 * 0 (and displayed), while an emptied number field stays empty ('') instead of
 * being coerced to 0.
 */
import type { TriggerCondition } from '@ppt/api-client';
import { fireEvent, render, screen } from '@testing-library/react';
import { useState } from 'react';
import { describe, expect, it } from 'vitest';
import { ConditionBuilder } from './ConditionBuilder';

// ConditionBuilder is fully controlled (onChange replaces the whole array), so
// drive it through a stateful harness and surface the first condition's value
// shape for assertion.
function Harness({ initial }: { initial: TriggerCondition[] }) {
  const [conditions, setConditions] = useState<TriggerCondition[]>(initial);
  return (
    <div>
      <ConditionBuilder conditions={conditions} onChange={setConditions} />
      <span data-testid="value-type">{typeof conditions[0]?.value}</span>
      <span data-testid="value-json">{JSON.stringify(conditions[0]?.value)}</span>
    </div>
  );
}

const numberCondition = (value: TriggerCondition['value']): TriggerCondition[] => [
  { field: 'payment.amount', operator: 'equals', value },
];

describe('ConditionBuilder number input zero-coercion', () => {
  it('preserves a legitimate 0 as the number 0', () => {
    render(<Harness initial={numberCondition(5)} />);
    const input = screen.getByRole('spinbutton') as HTMLInputElement;

    fireEvent.change(input, { target: { value: '0' } });

    expect(screen.getByTestId('value-type')).toHaveTextContent('number');
    expect(screen.getByTestId('value-json')).toHaveTextContent('0');
    expect(input.value).toBe('0');
  });

  it('keeps an emptied number field empty instead of coercing it to 0', () => {
    render(<Harness initial={numberCondition(5)} />);
    const input = screen.getByRole('spinbutton') as HTMLInputElement;

    fireEvent.change(input, { target: { value: '' } });

    // Regression guard: old code did `Number('') === 0`, so the cleared field
    // became the number 0 and the input snapped back to "0".
    expect(screen.getByTestId('value-json')).toHaveTextContent('""');
    expect(input.value).toBe('');
  });

  it('displays a stored 0 rather than rendering an empty field', () => {
    render(<Harness initial={numberCondition(0)} />);
    const input = screen.getByRole('spinbutton') as HTMLInputElement;

    expect(input.value).toBe('0');
  });
});
