/**
 * The maintenance catalog must be exposed to next-intl under
 * `layout.maintenance` for every configured locale, so the SSR kill-switch
 * document resolves localized copy (not a raw key or hardcoded English).
 *
 * This covers the real merge used by `i18n/request.ts`; the component-level
 * test mocks next-intl, so this is what proves the keys actually resolve.
 */

import { describe, expect, it } from 'vitest';
import { locales } from './config';
import {
  maintenanceMessages,
  resolveMaintenanceLocale,
  withMaintenanceMessages,
} from './maintenance-messages';

describe('maintenance-messages', () => {
  it('ships title + body for every configured locale', () => {
    for (const locale of locales) {
      const copy = maintenanceMessages[locale];
      expect(copy?.title, `missing maintenance title for ${locale}`).toBeTruthy();
      expect(copy?.body, `missing maintenance body for ${locale}`).toContain('{name}');
    }
  });

  it('merges the catalog under layout.maintenance without dropping existing keys', () => {
    const base = { layout: { placeholderTitle: 'X' }, other: { a: 1 } };
    const merged = withMaintenanceMessages(base, 'sk') as {
      layout: { placeholderTitle: string; maintenance: { title: string; body: string } };
      other: { a: number };
    };

    expect(merged.layout.placeholderTitle).toBe('X');
    expect(merged.other.a).toBe(1);
    expect(merged.layout.maintenance).toEqual(maintenanceMessages.sk);
  });

  it('falls back to en for an unknown locale and normalizes region subtags', () => {
    expect(resolveMaintenanceLocale('cs')).toBe('cs');
    expect(resolveMaintenanceLocale('de-AT')).toBe('de');
    expect(resolveMaintenanceLocale('fr')).toBe('en');
    expect(resolveMaintenanceLocale(null)).toBe('en');

    const merged = withMaintenanceMessages({}, 'xx-YY') as {
      layout: { maintenance: { title: string } };
    };
    expect(merged.layout.maintenance).toEqual(maintenanceMessages.en);
  });
});
