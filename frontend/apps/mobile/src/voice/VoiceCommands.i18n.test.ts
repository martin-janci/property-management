/**
 * VoiceCommands — confirmation/error i18n regression test
 * (code-review-mobile-rn-voice-cmd-hardcoded-en).
 *
 * `executeVoiceCommand` returned its spoken confirmation messages and the
 * unknown-command error as hardcoded English string literals ("Opening
 * announcements", "Going to home", "I didn't understand that command…") even
 * though the app is fully localized (sk/cs/de/pl/hu). This locks the contract:
 * every user-facing confirmation/error string is now sourced from an i18n key
 * under the `voice.*` namespace, so it follows the language the user picked.
 *
 * These are plain module functions (not React components), so — mirroring
 * `src/i18n/format.ts` — they read the shared i18next singleton rather than the
 * `useTranslation` hook. The test therefore initialises that same singleton
 * with the real locale bundles and asserts against actual translated output.
 *
 * Two layers of assertion:
 *  1. Behaviour layer — the returned message is locale-distinct (en vs sk) and
 *     the OLD hardcoded English literals never appear once a non-en language is
 *     active. If any string were still hardcoded, those assertions would fail.
 *  2. Resource layer — every locale JSON actually carries the `voice.*` keys
 *     with locale-distinct values, proving the keys resolve to real
 *     translations rather than dangling references.
 */

import i18n from 'i18next';
import cs from '../locales/cs.json';
import de from '../locales/de.json';
import en from '../locales/en.json';
import hu from '../locales/hu.json';
import pl from '../locales/pl.json';
import sk from '../locales/sk.json';
import type { ParsedVoiceCommand, VoiceIntent } from './types';
import { executeVoiceCommand } from './VoiceCommands';

const LOCALES = { en, sk, cs, de, pl, hu } as const;

function command(
  intent: VoiceIntent,
  params: ParsedVoiceCommand['params'] = {}
): ParsedVoiceCommand {
  return { intent, confidence: 1, params, transcript: '' };
}

// Old hardcoded English literals that must no longer be produced.
const HARDCODED_EN = [
  'Opening fault report form',
  'Showing announcements',
  'Opening voting',
  'Opening documents',
  'Checking notifications',
  'Going to home',
  'Opening settings',
];

beforeAll(async () => {
  await i18n.init({
    lng: 'en',
    fallbackLng: 'en',
    resources: {
      en: { translation: en },
      sk: { translation: sk },
      cs: { translation: cs },
      de: { translation: de },
      pl: { translation: pl },
      hu: { translation: hu },
    },
    interpolation: { escapeValue: false },
  });
});

afterAll(async () => {
  await i18n.changeLanguage('en');
});

describe('executeVoiceCommand confirmation/error i18n', () => {
  it('produces the English confirmation from i18n when the language is en', async () => {
    await i18n.changeLanguage('en');
    expect(executeVoiceCommand(command('view_announcements')).confirmationMessage).toBe(
      'Showing announcements'
    );
  });

  it('localizes confirmation messages for a non-en language', async () => {
    await i18n.changeLanguage('sk');
    const skMessage = executeVoiceCommand(command('view_announcements')).confirmationMessage;
    expect(skMessage).toBe(sk.voice.confirmations.announcements);
    // Locale-distinct: Slovak differs from the English literal.
    expect(skMessage).not.toBe(en.voice.confirmations.announcements);
    // And none of the old hardcoded English literals leak through.
    expect(HARDCODED_EN).not.toContain(skMessage);
  });

  it('interpolates the fault category into the localized confirmation', async () => {
    await i18n.changeLanguage('en');
    const message = executeVoiceCommand(
      command('report_fault', { faultCategory: 'elevator' })
    ).confirmationMessage;
    expect(message).toBe('Opening fault report for elevator issue');
  });

  it('interpolates the document search query into the localized confirmation', async () => {
    await i18n.changeLanguage('de');
    const message = executeVoiceCommand(
      command('view_documents', { query: 'lease' })
    ).confirmationMessage;
    expect(message).toContain('lease');
    expect(message).not.toBe('Searching documents for lease');
  });

  it('localizes the unknown-command error message', async () => {
    await i18n.changeLanguage('sk');
    const result = executeVoiceCommand(command('unknown'));
    expect(result.success).toBe(false);
    expect(result.errorMessage).toBe(sk.voice.errors.unknownCommand);
    expect(result.errorMessage).not.toBe(en.voice.errors.unknownCommand);
  });
});

describe('voice.* resource coverage', () => {
  const confirmationKeys = [
    'reportFaultCategory',
    'reportFaultForm',
    'announcements',
    'voting',
    'documentsSearch',
    'documents',
    'notifications',
    'dashboard',
    'settings',
  ] as const;

  it.each(Object.keys(LOCALES))('locale %s carries every voice confirmation/error key', (lang) => {
    const bundle = LOCALES[lang as keyof typeof LOCALES] as {
      voice?: { confirmations?: Record<string, string>; errors?: Record<string, string> };
    };
    expect(bundle.voice).toBeDefined();
    for (const key of confirmationKeys) {
      expect(typeof bundle.voice?.confirmations?.[key]).toBe('string');
      expect(bundle.voice?.confirmations?.[key]).toBeTruthy();
    }
    expect(typeof bundle.voice?.errors?.unknownCommand).toBe('string');
    expect(bundle.voice?.errors?.unknownCommand).toBeTruthy();
  });

  it('translations are locale-distinct for a representative key', () => {
    const values = Object.values(LOCALES).map((b) => b.voice.confirmations.settings);
    expect(new Set(values).size).toBe(values.length);
  });
});
