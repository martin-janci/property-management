import i18n from 'i18next';
import LanguageDetector from 'i18next-browser-languagedetector';
import { initReactI18next } from 'react-i18next';

import cs from '../../messages/cs.json';
import en from '../../messages/en.json';
import sk from '../../messages/sk.json';

const resources = {
  en: { translation: en },
  sk: { translation: sk },
  cs: { translation: cs },
};

export const locales = ['en', 'sk', 'cs'] as const;
export type Locale = (typeof locales)[number];

i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources,
    fallbackLng: 'en',
    supportedLngs: [...locales],
    interpolation: {
      escapeValue: false,
    },
    detection: {
      order: ['localStorage', 'navigator'],
      caches: ['localStorage'],
    },
  });

export default i18n;
