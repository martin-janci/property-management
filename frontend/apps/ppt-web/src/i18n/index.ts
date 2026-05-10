import i18n from 'i18next';
import LanguageDetector from 'i18next-browser-languagedetector';
import { initReactI18next } from 'react-i18next';
// Import translation files
import cs from '../../messages/cs.json';
import de from '../../messages/de.json';
import en from '../../messages/en.json';
import hu from '../../messages/hu.json';
import pl from '../../messages/pl.json';
import sk from '../../messages/sk.json';
import { defaultLocale, locales } from './config';

const resources = {
  en: { translation: en },
  sk: { translation: sk },
  cs: { translation: cs },
  de: { translation: de },
  pl: { translation: pl },
  hu: { translation: hu },
};

i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources,
    fallbackLng: defaultLocale,
    supportedLngs: [...locales],
    interpolation: {
      escapeValue: false, // React already escapes values
    },
    detection: {
      order: ['localStorage', 'navigator', 'htmlTag'],
      caches: ['localStorage'],
      lookupLocalStorage: 'ppt-language',
    },
  });

export default i18n;
export { defaultLocale, type Locale, localeFlags, localeNames, locales } from './config';
