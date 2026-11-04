/// <reference types="vitest/globals" />
/**
 * Bulk person-month entry (round-3 audit #23).
 *
 * - Units load after the page mounts. Their default count of 0 used to be
 *   flagged "must be 0 or more" because the count map was seeded only once,
 *   at mount, while the unit list was still empty.
 * - Month names come from `Intl` in the UI language instead of English keys.
 */
import { fireEvent, render, screen } from '@testing-library/react';
import i18n from 'i18next';
import type { BulkEntryUnit } from '../types';
import { BulkEntryPage, monthName } from './BulkEntryPage';

const units: BulkEntryUnit[] = [
  { unitId: 'u1', unitDesignation: 'A-1', personCount: 0 },
  { unitId: 'u2', unitDesignation: 'A-2', personCount: 3, currentPersonCount: 3 },
];

afterEach(async () => {
  await i18n.changeLanguage('en');
});

function renderPage(onSubmit = vi.fn(), initialUnits: BulkEntryUnit[] = units) {
  const props = {
    buildingName: 'Building',
    initialYear: 2026,
    initialMonth: 11,
    onSubmit,
    onBack: () => {},
  };
  const utils = render(<BulkEntryPage {...props} units={initialUnits} />);
  return { ...utils, props, onSubmit };
}

it('accepts units that load after mount with a count of 0', () => {
  const { rerender, props, onSubmit, container } = renderPage(vi.fn(), []);
  rerender(<BulkEntryPage {...props} units={units} />);

  fireEvent.submit(container.querySelector('form') as HTMLFormElement);
  expect(container.querySelector('tr.bg-red-50')).toBeNull();

  // Confirmation dialog → confirm (last button in the dialog).
  const buttons = screen.getAllByRole('button');
  fireEvent.click(buttons[buttons.length - 1]);
  expect(onSubmit).toHaveBeenCalledWith({
    year: 2026,
    month: 11,
    entries: [
      { unitId: 'u1', unitDesignation: 'A-1', personCount: 0 },
      { unitId: 'u2', unitDesignation: 'A-2', personCount: 3 },
    ],
  });
});

it('submits edited counts', () => {
  const { container, onSubmit } = renderPage();
  const inputs = container.querySelectorAll('input[type="number"]');
  fireEvent.change(inputs[0], { target: { value: '2' } });
  fireEvent.submit(container.querySelector('form') as HTMLFormElement);
  const buttons = screen.getAllByRole('button');
  fireEvent.click(buttons[buttons.length - 1]);
  expect(onSubmit.mock.calls[0][0].entries[0].personCount).toBe(2);
});

it('lists month names in the UI language', async () => {
  await i18n.changeLanguage('sk');
  const { container } = renderPage();
  const select = container.querySelector('#bulk-month') as HTMLSelectElement;
  expect(select.options).toHaveLength(12);
  expect(select.options[0].textContent).toBe('Január');
  expect(select.options[6].textContent).toBe('Júl');
});

it('monthName uses Intl and capitalizes', () => {
  expect(monthName(11, 'en')).toBe('November');
  expect(monthName(1, 'de')).toBe('Januar');
  expect(monthName(11, 'cs')).toBe('Listopad');
});
