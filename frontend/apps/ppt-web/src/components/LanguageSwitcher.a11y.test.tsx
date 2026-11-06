/// <reference types="vitest/globals" />
/**
 * LanguageSwitcher Accessibility Tests (Epic 125, Story 125.2)
 *
 * Tests WCAG 2.1 AA compliance for the LanguageSwitcher component including:
 * - aria-label for screen readers
 * - Keyboard accessibility
 * - Focus indicators
 */

import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { axe } from 'vitest-axe';
import { LanguageSwitcher } from './LanguageSwitcher';

// Mock i18next
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    i18n: {
      language: 'en',
      changeLanguage: vi.fn(),
    },
    t: (key: string) => (key === 'aria.selectLanguage' ? 'Select language' : key),
  }),
}));

// Mock locale data
vi.mock('../i18n', () => ({
  locales: ['en', 'sk', 'cs', 'de', 'hu', 'pl'],
  localeNames: {
    en: 'English',
    sk: 'Slovak',
    cs: 'Czech',
    de: 'German',
    hu: 'Hungarian',
    pl: 'Polish',
  },
  localeFlags: {
    en: '',
    sk: '',
    cs: '',
    de: '',
    hu: '',
    pl: '',
  },
}));

describe('LanguageSwitcher Accessibility', () => {
  it('should have no accessibility violations', async () => {
    const { container } = render(<LanguageSwitcher />);

    const results = await axe(container);
    expect(results).toHaveNoViolations();
  });

  it('should have accessible label', () => {
    render(<LanguageSwitcher />);

    const select = screen.getByRole('combobox', { name: /select language/i });
    expect(select).toBeInTheDocument();
    expect(select).toHaveAttribute('aria-label', 'Select language');
  });

  it('should be keyboard accessible', async () => {
    const user = userEvent.setup();
    render(<LanguageSwitcher />);

    const select = screen.getByRole('combobox');

    // Tab to focus the select
    await user.tab();
    expect(select).toHaveFocus();

    // Options should list all available languages
    expect(select.querySelectorAll('option')).toHaveLength(6);
  });

  it('should have all language options', () => {
    render(<LanguageSwitcher />);

    const select = screen.getByRole('combobox');
    const options = select.querySelectorAll('option');

    const optionTexts = Array.from(options).map((opt) => opt.textContent?.trim());
    expect(optionTexts).toContain('English');
    expect(optionTexts).toContain('Slovak');
    expect(optionTexts).toContain('Czech');
    expect(optionTexts).toContain('German');
    expect(optionTexts).toContain('Hungarian');
    expect(optionTexts).toContain('Polish');
  });

  it('each option should have a distinct value', () => {
    render(<LanguageSwitcher />);

    const select = screen.getByRole('combobox');
    const options = select.querySelectorAll('option');
    const values = Array.from(options).map((opt) => opt.value);

    // All values should be unique
    const uniqueValues = new Set(values);
    expect(uniqueValues.size).toBe(values.length);
  });
});
