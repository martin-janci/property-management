/**
 * LanguageSwitcher Component Tests
 *
 * Tests for language switching functionality in mobile app (Epic 111).
 */

import { fireEvent, render, screen } from '@testing-library/react-native';
import { LanguageSwitcher } from './LanguageSwitcher';

// Get the mocked changeLanguage function
const mockChangeLanguage = jest.fn();
jest.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: {
      language: 'en',
      changeLanguage: mockChangeLanguage,
    },
  }),
}));

describe('LanguageSwitcher', () => {
  beforeEach(() => {
    mockChangeLanguage.mockClear();
  });

  it('renders language button', async () => {
    await render(<LanguageSwitcher />);
    expect(screen.getByText(/English/)).toBeOnTheScreen();
  });

  it('displays flag emoji with language name', async () => {
    await render(<LanguageSwitcher />);
    expect(screen.getByText(/🇬🇧/)).toBeOnTheScreen();
  });

  it('opens modal when button is pressed', async () => {
    await render(<LanguageSwitcher />);

    const button = screen.getByText(/English/);
    await fireEvent.press(button);

    // Modal should show language options
    expect(screen.getByText('Slovenčina')).toBeOnTheScreen();
    expect(screen.getByText('Čeština')).toBeOnTheScreen();
    expect(screen.getByText('Deutsch')).toBeOnTheScreen();
  });

  it('shows all language flags in modal', async () => {
    await render(<LanguageSwitcher />);

    const button = screen.getByText(/English/);
    await fireEvent.press(button);

    expect(screen.getByText('🇸🇰')).toBeOnTheScreen();
    expect(screen.getByText('🇨🇿')).toBeOnTheScreen();
    expect(screen.getByText('🇩🇪')).toBeOnTheScreen();
  });

  it('calls changeLanguage when language option is pressed', async () => {
    await render(<LanguageSwitcher />);

    // Open modal
    const button = screen.getByText(/English/);
    await fireEvent.press(button);

    // Select Slovak
    const skOption = screen.getByText('Slovenčina');
    await fireEvent.press(skOption);

    expect(mockChangeLanguage).toHaveBeenCalledWith('sk');
  });

  it('shows cancel button in modal', async () => {
    await render(<LanguageSwitcher />);

    const button = screen.getByText(/English/);
    await fireEvent.press(button);

    // Translation key for cancel
    expect(screen.getByText('common.cancel')).toBeOnTheScreen();
  });

  it('shows modal title with translation key', async () => {
    await render(<LanguageSwitcher />);

    const button = screen.getByText(/English/);
    await fireEvent.press(button);

    // Translation key for settings.language
    expect(screen.getByText('settings.language')).toBeOnTheScreen();
  });
});
