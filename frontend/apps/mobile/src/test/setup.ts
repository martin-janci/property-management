/**
 * Jest test setup file for Mobile App
 *
 * Configures testing environment:
 * - Mock implementations for React Native APIs
 * - Mock implementations for Expo modules
 * - i18next mock for translation testing
 *
 * The process timezone is pinned to UTC in `jest.config.js` (it must be set
 * before the worker's ICU data is initialised, which is too late from here).
 */

// matchers are auto-registered in @testing-library/react-native >= 13

// Mock react-i18next
jest.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: {
      language: 'en',
      changeLanguage: jest.fn(),
    },
  }),
  initReactI18next: {
    type: '3rdParty',
    init: jest.fn(),
  },
}));

// Mock i18n config
jest.mock('../i18n', () => ({
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
