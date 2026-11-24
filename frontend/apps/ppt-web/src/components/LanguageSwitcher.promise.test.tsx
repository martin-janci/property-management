/// <reference types="vitest/globals" />
/**
 * LanguageSwitcher error-handling tests.
 *
 * `i18n.changeLanguage()` returns a Promise (it persists the chosen locale to
 * localStorage and may load resources). If it rejects, the rejection must be
 * caught rather than left to float as an unhandled rejection.
 */

import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { LanguageSwitcher } from './LanguageSwitcher';

const changeLanguage = vi.fn();

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    i18n: {
      language: 'en',
      changeLanguage,
    },
    t: (key: string) => (key === 'aria.selectLanguage' ? 'Select language' : key),
  }),
}));

vi.mock('../i18n', () => ({
  locales: ['en', 'sk'],
  localeNames: { en: 'English', sk: 'Slovak' },
  localeFlags: { en: '', sk: '' },
}));

describe('LanguageSwitcher error handling', () => {
  afterEach(() => {
    vi.clearAllMocks();
  });

  it('catches a rejected changeLanguage instead of leaving it unhandled', async () => {
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});
    // Created lazily at call time so the component's .catch attaches in the
    // same microtask — an eager Promise.reject would settle as "unhandled"
    // before the handler runs, regardless of the fix.
    changeLanguage.mockImplementation(() => Promise.reject(new Error('locale load failed')));

    const user = userEvent.setup();
    render(<LanguageSwitcher />);

    await user.selectOptions(screen.getByRole('combobox'), 'sk');

    // Flush the microtask queue so the rejection (and its .catch) settles.
    await Promise.resolve();
    await Promise.resolve();

    expect(changeLanguage).toHaveBeenCalledWith('sk');
    // The handler must observe the rejection; a floating promise never logs.
    expect(consoleError).toHaveBeenCalled();

    consoleError.mockRestore();
  });
});
