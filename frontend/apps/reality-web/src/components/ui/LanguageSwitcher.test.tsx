/**
 * LanguageSwitcher Component Tests (Epic 111).
 *
 * The original test asserted a native <select> (role=combobox). The
 * component was rewritten as a custom popover (trigger button + listbox
 * div with role=option entries) — these tests assert the new interaction
 * model: trigger click opens the menu, options become visible, clicking
 * one calls router.replace().
 */

import { act, fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { LanguageSwitcher } from './LanguageSwitcher';

// Mock i18n config
vi.mock('../../i18n', () => ({
  locales: ['en', 'sk', 'cs', 'de'],
  localeNames: {
    en: 'English',
    sk: 'Slovenčina',
    cs: 'Čeština',
    de: 'Deutsch',
  },
  localeFlags: {
    en: '🇬🇧',
    sk: '🇸🇰',
    cs: '🇨🇿',
    de: '🇩🇪',
  },
}));

// next-intl's useLocale is read inside the component — mock returns 'en'.
vi.mock('next-intl', () => ({
  useLocale: () => 'en',
}));

// Mock routing
const mockReplace = vi.fn();
vi.mock('../../i18n/routing', () => ({
  useRouter: () => ({
    replace: mockReplace,
    push: vi.fn(),
  }),
  usePathname: () => '/listings',
}));

describe('LanguageSwitcher', () => {
  beforeEach(() => {
    mockReplace.mockClear();
  });

  it('renders a trigger button with the active locale code', () => {
    render(<LanguageSwitcher />);
    const trigger = screen.getByRole('button', { name: /language: english/i });
    expect(trigger).toBeInTheDocument();
    // Code label inside the trigger uses uppercase locale
    expect(trigger.textContent).toMatch(/EN/);
  });

  it('does not show the listbox until the trigger is clicked', () => {
    render(<LanguageSwitcher />);
    expect(screen.queryByRole('listbox')).not.toBeInTheDocument();
  });

  it('opens the listbox on trigger click and shows all four locales', () => {
    render(<LanguageSwitcher />);
    fireEvent.click(screen.getByRole('button', { name: /language: english/i }));
    const listbox = screen.getByRole('listbox');
    expect(listbox).toBeInTheDocument();
    const options = screen.getAllByRole('option');
    expect(options).toHaveLength(4);
    expect(screen.getByText(/English/)).toBeInTheDocument();
    expect(screen.getByText(/Slovenčina/)).toBeInTheDocument();
    expect(screen.getByText(/Čeština/)).toBeInTheDocument();
    expect(screen.getByText(/Deutsch/)).toBeInTheDocument();
  });

  it('marks the active locale option as aria-selected', () => {
    render(<LanguageSwitcher />);
    fireEvent.click(screen.getByRole('button', { name: /language: english/i }));
    const active = screen.getAllByRole('option').find((el) => el.textContent?.includes('English'));
    expect(active).toHaveAttribute('aria-selected', 'true');
  });

  it('calls router.replace with the chosen locale and closes the menu', () => {
    render(<LanguageSwitcher />);
    fireEvent.click(screen.getByRole('button', { name: /language: english/i }));
    const sk = screen.getAllByRole('option').find((el) => el.textContent?.includes('Slovenčina'));
    expect(sk).toBeTruthy();
    act(() => {
      fireEvent.click(sk!);
    });
    expect(mockReplace).toHaveBeenCalledWith('/listings', { locale: 'sk' });
    expect(screen.queryByRole('listbox')).not.toBeInTheDocument();
  });

  it('closes the menu when Escape is pressed', () => {
    render(<LanguageSwitcher />);
    fireEvent.click(screen.getByRole('button', { name: /language: english/i }));
    expect(screen.getByRole('listbox')).toBeInTheDocument();
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(screen.queryByRole('listbox')).not.toBeInTheDocument();
  });
});
