/**
 * PersonMonthsScreen — header + validation-Alert i18n regression test
 * (code-review-mobile-rn-outages-news-header-chip-i18n).
 *
 * The screen rendered its header title/subtitle, the Save/Cancel/Edit/Bulk
 * controls, the "{n} person(s)" count, and the invalid-value `Alert.alert`
 * as hardcoded English literals — the screen did not even import
 * react-i18next. This suite locks the contract: those strings are now sourced
 * from i18n keys via `t()`.
 *
 * Mirrors FaultsListScreen.i18n.test.tsx: render layer asserts the screen
 * renders the i18n KEYS (setup mocks `t(key) => key`) and that the old
 * literals are gone; the Alert is checked via an `Alert.alert` spy; resource
 * layer asserts the locale JSON carries the new keys.
 */

import { fireEvent, render, screen } from '@testing-library/react-native';
import { Alert } from 'react-native';
import cs from '../../locales/cs.json';
import de from '../../locales/de.json';
import en from '../../locales/en.json';
import hu from '../../locales/hu.json';
import pl from '../../locales/pl.json';
import sk from '../../locales/sk.json';
import { PersonMonthsScreen } from './PersonMonthsScreen';

// The old hardcoded header/control literals that must no longer be rendered.
const HARDCODED = [
  'Person-months',
  'Declare how many people lived in your unit each month for cost allocation.',
  'Save',
  'Cancel',
  'Edit',
  'Bulk entry…',
];

describe('PersonMonthsScreen header + control i18n', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('renders header title/subtitle keys, not hardcoded literals', () => {
    render(<PersonMonthsScreen />);
    expect(screen.getByText('personMonths.title')).toBeTruthy();
    expect(screen.getByText('personMonths.subtitle')).toBeTruthy();
  });

  it('renders control keys (edit/bulk) via common/personMonths, not literals', () => {
    render(<PersonMonthsScreen />);
    // One draft entry is editable -> one edit link.
    expect(screen.getByText('common.edit')).toBeTruthy();
    expect(screen.getByText('personMonths.bulkEntry')).toBeTruthy();
    for (const literal of HARDCODED) {
      expect(screen.queryByText(literal)).toBeNull();
    }
  });

  it('raises the invalid-value Alert via i18n keys, not hardcoded literals', () => {
    const alertSpy = jest.spyOn(Alert, 'alert').mockImplementation(() => {});
    render(<PersonMonthsScreen />);
    fireEvent.press(screen.getByText('common.edit'));
    fireEvent.changeText(screen.getByDisplayValue('3'), '99');
    fireEvent.press(screen.getByText('common.save'));
    expect(alertSpy).toHaveBeenCalledWith(
      'personMonths.invalidTitle',
      'personMonths.invalidMessage'
    );
    alertSpy.mockRestore();
  });
});

describe('personMonths keys resolve to real, locale-distinct translations', () => {
  it('en carries the expected English copy', () => {
    expect(en.personMonths.title).toBe('Person-months');
    expect(en.personMonths.subtitle).toBe(
      'Declare how many people lived in your unit each month for cost allocation.'
    );
    expect(en.personMonths.invalidTitle).toBe('Invalid value');
    expect(en.personMonths.invalidMessage).toBe('Enter a whole number between 0 and 20.');
    expect(en.personMonths.bulkEntry).toBe('Bulk entry…');
    expect(en.personMonths.personCount_one).toContain('{{count}}');
    expect(en.personMonths.personCount_other).toContain('{{count}}');
  });

  it('every locale defines the keys and translates the title/subtitle away from English', () => {
    for (const locale of [sk, cs, de, pl, hu]) {
      for (const key of ['title', 'subtitle', 'invalidTitle', 'invalidMessage', 'bulkEntry']) {
        const value = (locale.personMonths as Record<string, string>)[key];
        expect(typeof value).toBe('string');
        expect(value.length).toBeGreaterThan(0);
      }
      expect(locale.personMonths.subtitle).not.toBe(en.personMonths.subtitle);
      expect(locale.personMonths.personCount_one).toContain('{{count}}');
      expect(locale.personMonths.personCount_other).toContain('{{count}}');
    }
  });
});
