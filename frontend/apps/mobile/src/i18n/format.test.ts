import { formatDate, formatDateTime, formatTime, resolveLocale } from './format';

describe('resolveLocale', () => {
  it.each([
    ['en', 'en'],
    ['sk', 'sk'],
    ['cs', 'cs'],
    ['de', 'de'],
    ['pl', 'pl'],
    ['hu', 'hu'],
  ])('passes through the supported language %s', (input, expected) => {
    expect(resolveLocale(input)).toBe(expected);
  });

  it('strips the region subtag (de-DE -> de)', () => {
    expect(resolveLocale('de-DE')).toBe('de');
    expect(resolveLocale('en-US')).toBe('en');
  });

  it('falls back to the default locale for unsupported languages', () => {
    expect(resolveLocale('fr')).toBe('en');
    expect(resolveLocale('')).toBe('en');
  });

  it('falls back to the default locale when nothing is active', () => {
    // No explicit language and (in the unit-test env) no initialised i18next
    // language → must not throw and must not emit a foreign locale.
    expect(resolveLocale()).toBe('en');
  });
});

describe('formatDate', () => {
  const iso = '2026-07-01T00:00:00Z';

  // Regression for #2282: the hardcoded 'en-US' meant dates never localised.
  // A non-en locale must format the same instant differently from en.
  it('formats a non-en locale differently from en', () => {
    const en = formatDate(iso, { month: 'long', year: 'numeric' }, 'en');
    const de = formatDate(iso, { month: 'long', year: 'numeric' }, 'de');
    const sk = formatDate(iso, { month: 'long', year: 'numeric' }, 'sk');

    expect(en).toBeTruthy();
    // German "Juli" / Slovak "júl" differ from English "July".
    expect(de).not.toBe(en);
    expect(sk).not.toBe(en);
  });

  it('honours the locale it is handed for each supported language', () => {
    for (const lang of ['en', 'sk', 'cs', 'de', 'pl', 'hu']) {
      expect(formatDate(iso, { year: 'numeric' }, lang)).toMatch(/2026/);
    }
  });
});

describe('formatTime / formatDateTime', () => {
  const iso = '2026-07-01T13:37:00Z';

  it('produce non-empty localized strings', () => {
    expect(formatTime(iso, { hour: '2-digit', minute: '2-digit' }, 'de')).toBeTruthy();
    expect(formatDateTime(iso, { dateStyle: 'short', timeStyle: 'short' }, 'sk')).toBeTruthy();
  });
});
