/**
 * InvoiceForm — editable line-item list must use a stable per-row key.
 *
 * Code-review finding (`code-review-ppt-web-ui-budgetform-index-key-editable-list`):
 * the editable/reorderable invoice line-item list was keyed by the array index
 * (`key={`item-${index}`}`). Keying an editable list by index makes React
 * reconcile rows by position instead of identity, so inserting or removing a
 * row reuses the DOM node of a *different* item. For controlled inputs the
 * visible `value` still reconciles, but the underlying DOM node identity — and
 * anything tied to it (focus, cursor/selection, IME composition, uncontrolled
 * child state) — follows the slot, not the row.
 *
 * This regression pins the fix via DOM-node identity, which is deterministic in
 * jsdom: we hold a reference to the last row's input, delete the FIRST row, and
 * assert the input still showing that row's value is the *same* HTMLElement.
 * With index keys React reuses the node at the shifted slot, so the reference is
 * detached and the assertion fails; with a stable per-row key the surviving row
 * keeps its node and the assertion passes.
 */

/// <reference types="vitest/globals" />
import type { CreateInvoiceItem } from '@ppt/api-client';
import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { InvoiceForm } from './InvoiceForm';

function item(description: string): CreateInvoiceItem {
  return { description, quantity: 1, unit_price: 10 };
}

describe('InvoiceForm — stable line-item keys (index-key regression)', () => {
  it('keeps a surviving row bound to its own DOM node when an earlier row is removed', () => {
    render(
      <InvoiceForm
        units={[]}
        initialData={{ items: [item('Alpha'), item('Beta'), item('Gamma')] }}
        onSubmit={vi.fn()}
        onCancel={vi.fn()}
      />
    );

    const descriptionsBefore = screen.getAllByLabelText('Description') as HTMLInputElement[];
    expect(descriptionsBefore.map((i) => i.value)).toEqual(['Alpha', 'Beta', 'Gamma']);

    // Hold a reference to the LAST row's input element (value "Gamma").
    const gammaNode = descriptionsBefore.find((i) => i.value === 'Gamma');
    expect(gammaNode).toBeDefined();

    // Remove the FIRST row ("Alpha"). The other rows shift down one slot.
    const removeButtons = screen.getAllByRole('button', { name: 'Remove' });
    fireEvent.click(removeButtons[0]);

    const descriptionsAfter = screen.getAllByLabelText('Description') as HTMLInputElement[];
    expect(descriptionsAfter.map((i) => i.value)).toEqual(['Beta', 'Gamma']);

    // The input that still shows "Gamma" must be the very same DOM node we held
    // before the removal. Index keys reuse the node at the now-shifted slot, so
    // this is a *different* element and the assertion fails without the fix.
    const gammaNodeAfter = descriptionsAfter.find((i) => i.value === 'Gamma');
    expect(gammaNodeAfter).toBe(gammaNode);
  });

  it('binds added rows to fresh nodes without disturbing existing rows', () => {
    render(
      <InvoiceForm
        units={[]}
        initialData={{ items: [item('Alpha')] }}
        onSubmit={vi.fn()}
        onCancel={vi.fn()}
      />
    );

    const alphaBefore = (screen.getAllByLabelText('Description') as HTMLInputElement[]).find(
      (i) => i.value === 'Alpha'
    );
    expect(alphaBefore).toBeDefined();

    fireEvent.click(screen.getByRole('button', { name: /add item/i }));

    const descriptionsAfter = screen.getAllByLabelText('Description') as HTMLInputElement[];
    expect(descriptionsAfter).toHaveLength(2);

    // The pre-existing row keeps its own node after an append.
    const alphaAfter = descriptionsAfter.find((i) => i.value === 'Alpha');
    expect(alphaAfter).toBe(alphaBefore);
  });
});
