/**
 * RailsEditor — TDD test suite (Task 3, Layout Editor MVP)
 *
 * 4 scenarios:
 *   1. toggling reorderable calls onChange with reorderable:true
 *   2. checking hideable for a type adds it; unchecking removes it
 *   3. whitelist text input: typing "title, limit" calls onChange with parsed list;
 *      clearing removes the key
 *   4. resync: rerender with different prop_whitelist → input shows new value
 *
 * react-i18next is mocked so t() returns defaultValue (or the key if none).
 */

import { fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { Rails } from './api';
import { RailsEditor } from './RailsEditor';

// ---------------------------------------------------------------------------
// Mock react-i18next
// ---------------------------------------------------------------------------
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (_k: string, o?: { defaultValue?: string }) => o?.defaultValue ?? _k,
  }),
  Trans: ({ children }: { children: React.ReactNode }) => children,
}));

// ---------------------------------------------------------------------------
// Fixture data
// ---------------------------------------------------------------------------

const SECTION_TYPES = ['faq.v1', 'promo.v1', 'gallery.v1'];

const BASE_RAILS: Rails = {
  hideable: [],
  mode_editable: [],
  reorderable: false,
  prop_whitelist: {},
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function buildProps(overrides: Partial<Parameters<typeof RailsEditor>[0]> = {}) {
  return {
    rails: BASE_RAILS,
    sectionTypes: SECTION_TYPES,
    onChange: vi.fn(),
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// Scenario 1 — reorderable checkbox
// ---------------------------------------------------------------------------

describe('Scenario 1: reorderable checkbox', () => {
  it('toggling reorderable calls onChange with reorderable:true', async () => {
    const onChange = vi.fn();
    const user = userEvent.setup();
    render(<RailsEditor {...buildProps({ onChange })} />);

    const checkbox = screen.getByTestId('reorderable-checkbox') as HTMLInputElement;
    expect(checkbox.checked).toBe(false);

    await user.click(checkbox);

    expect(onChange).toHaveBeenCalledOnce();
    const next: Rails = onChange.mock.calls[0][0];
    expect(next.reorderable).toBe(true);
  });

  it('unchecking reorderable (when true) calls onChange with reorderable:false', async () => {
    const onChange = vi.fn();
    const user = userEvent.setup();
    render(
      <RailsEditor {...buildProps({ onChange, rails: { ...BASE_RAILS, reorderable: true } })} />
    );

    const checkbox = screen.getByTestId('reorderable-checkbox') as HTMLInputElement;
    expect(checkbox.checked).toBe(true);

    await user.click(checkbox);

    expect(onChange).toHaveBeenCalledOnce();
    const next: Rails = onChange.mock.calls[0][0];
    expect(next.reorderable).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// Scenario 2 — hideable checkboxes per type
// ---------------------------------------------------------------------------

describe('Scenario 2: hideable checkboxes', () => {
  it('checking hideable for a type adds it to hideable array', async () => {
    const onChange = vi.fn();
    const user = userEvent.setup();
    render(<RailsEditor {...buildProps({ onChange })} />);

    const checkbox = screen.getByTestId('hideable-faq.v1') as HTMLInputElement;
    expect(checkbox.checked).toBe(false);

    await user.click(checkbox);

    expect(onChange).toHaveBeenCalledOnce();
    const next: Rails = onChange.mock.calls[0][0];
    expect(next.hideable).toContain('faq.v1');
  });

  it('unchecking hideable for a type removes it from hideable array', async () => {
    const onChange = vi.fn();
    const user = userEvent.setup();
    render(
      <RailsEditor
        {...buildProps({
          onChange,
          rails: { ...BASE_RAILS, hideable: ['faq.v1', 'promo.v1'] },
        })}
      />
    );

    const checkbox = screen.getByTestId('hideable-faq.v1') as HTMLInputElement;
    expect(checkbox.checked).toBe(true);

    await user.click(checkbox);

    expect(onChange).toHaveBeenCalledOnce();
    const next: Rails = onChange.mock.calls[0][0];
    expect(next.hideable).not.toContain('faq.v1');
    expect(next.hideable).toContain('promo.v1');
  });
});

// ---------------------------------------------------------------------------
// Scenario 3 — prop_whitelist text input
// ---------------------------------------------------------------------------

describe('Scenario 3: prop_whitelist text input', () => {
  it('typing "title, limit" and blurring calls onChange with prop_whitelist containing the list', () => {
    const onChange = vi.fn();
    render(<RailsEditor {...buildProps({ onChange })} />);

    const input = screen.getByTestId('whitelist-faq.v1') as HTMLInputElement;

    fireEvent.change(input, { target: { value: 'title, limit' } });
    fireEvent.blur(input);

    expect(onChange).toHaveBeenCalledOnce();
    const next: Rails = onChange.mock.calls[0][0];
    expect(next.prop_whitelist['faq.v1']).toEqual(['title', 'limit']);
  });

  it('clearing the whitelist input removes the key from prop_whitelist', () => {
    const onChange = vi.fn();
    render(
      <RailsEditor
        {...buildProps({
          onChange,
          rails: { ...BASE_RAILS, prop_whitelist: { 'faq.v1': ['title'] } },
        })}
      />
    );

    const input = screen.getByTestId('whitelist-faq.v1') as HTMLInputElement;

    fireEvent.change(input, { target: { value: '' } });
    fireEvent.blur(input);

    expect(onChange).toHaveBeenCalledOnce();
    const next: Rails = onChange.mock.calls[0][0];
    expect(Object.prototype.hasOwnProperty.call(next.prop_whitelist, 'faq.v1')).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// Scenario 4 — resync: external rails prop update wins when not editing
// ---------------------------------------------------------------------------

describe('Scenario 4: whitelist input resyncs on external rails prop change', () => {
  it('shows updated whitelist when rails prop changes externally (not editing)', () => {
    const props = buildProps({
      rails: { ...BASE_RAILS, prop_whitelist: { 'faq.v1': ['title'] } },
    });
    const { rerender } = render(<RailsEditor {...props} />);

    const input = screen.getByTestId('whitelist-faq.v1') as HTMLInputElement;
    expect(input.value).toBe('title');

    // External update (e.g. draft reload)
    rerender(
      <RailsEditor
        {...props}
        rails={{ ...BASE_RAILS, prop_whitelist: { 'faq.v1': ['title', 'limit'] } }}
      />
    );

    expect((screen.getByTestId('whitelist-faq.v1') as HTMLInputElement).value).toBe('title, limit');
  });
});
