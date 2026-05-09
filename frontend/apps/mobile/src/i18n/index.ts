import AsyncStorage from '@react-native-async-storage/async-storage';
import * as Localization from 'expo-localization';
import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
// Import translation files
import cs from '../locales/cs.json';
import de from '../locales/de.json';
import en from '../locales/en.json';
import hu from '../locales/hu.json';
import pl from '../locales/pl.json';
import sk from '../locales/sk.json';
import { defaultLocale, locales } from './config';

const LANGUAGE_STORAGE_KEY = 'ppt-language';

const resources = {
  en: { translation: en },
  sk: { translation: sk },
  cs: { translation: cs },
  de: { translation: de },
  pl: { translation: pl },
  hu: { translation: hu },
};

/**
 * Get the device's preferred language or fall back to default.
 */
function getDeviceLanguage(): string {
  const deviceLocale = Localization.getLocales()[0]?.languageCode ?? defaultLocale;
  // Check if device language is supported
  if (locales.includes(deviceLocale as (typeof locales)[number])) {
    return deviceLocale;
  }
  return defaultLocale;
}

/**
 * Language detector for React Native with AsyncStorage persistence.
 */
const languageDetector = {
  type: 'languageDetector' as const,
  async: true,
  detect: async (callback: (lng: string) => void) => {
    try {
      // Try to get saved language preference
      const savedLanguage = await AsyncStorage.getItem(LANGUAGE_STORAGE_KEY);
      if (savedLanguage && locales.includes(savedLanguage as (typeof locales)[number])) {
        callback(savedLanguage);
        return;
      }
    } catch {
      // Fall through to device language
    }
    // Use device language or default
    callback(getDeviceLanguage());
  },
  init: () => {},
  cacheUserLanguage: async (lng: string) => {
    try {
      await AsyncStorage.setItem(LANGUAGE_STORAGE_KEY, lng);
    } catch {
      // Silently fail - language will be re-detected on next load
    }
  },
};

i18n
  .use(languageDetector)
  .use(initReactI18next)
  .init({
    resources,
    fallbackLng: defaultLocale,
    supportedLngs: [...locales],
    interpolation: {
      escapeValue: false, // React Native handles escaping
    },
    react: {
      useSuspense: false, // Disable suspense for React Native
    },
  });

export default i18n;
export { defaultLocale, type Locale, localeFlags, localeNames, locales } from './config';
