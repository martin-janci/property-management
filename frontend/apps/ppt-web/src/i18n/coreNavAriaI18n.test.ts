/// <reference types="vitest/globals" />
/**
 * Core-navigation aria-label i18n regression
 * (code-review-ppt-web-core-nav-aria-label-i18n).
 *
 * The AppShell core navigation (`src/components/AppShell/AppShell.tsx`) labelled
 * its four landmark/controls aria-labels with the `shell.*` namespace *and* an
 * inline English `defaultValue` fallback — e.g.
 * `aria-label={t('shell.mainNavigation', { defaultValue: 'Main navigation' })}`.
 *
 * That inline English is a hardcoded aria string: `fallbackLng` masks any
 * locale gap, so a missing sk/cs/de/pl/hu key announces the English default to
 * assistive tech instead of failing typecheck/lint. Every other aria-label in
 * ppt-web (see `ariaI18n.test.ts`, `aria.closeMenu` already consumed by
 * `MessageBubble.tsx`) routes through the shared `aria.*` namespace with a bare
 * `t('aria.key')` call and the English copy living only in `en.json`.
 *
 * This test locks the core nav onto that same convention:
 *   1. the four nav aria keys exist — non-empty — in all six locale bundles
 *      under `aria.*` (parity, so non-English users hear their own language);
 *   2. AppShell no longer carries an inline `defaultValue` aria-label nor a
 *      `shell.*`-namespaced nav aria-label (no hardcoded-English fallback).
 *
 * It fails on the pre-fix tree (three keys missing from `aria.*`; AppShell still
 * on `shell.* + defaultValue`) and passes once the labels are migrated.
 */

import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import cs from '../../messages/cs.json';
import de from '../../messages/de.json';
import en from '../../messages/en.json';
import hu from '../../messages/hu.json';
import pl from '../../messages/pl.json';
import sk from '../../messages/sk.json';

type Json = Record<string, unknown>;

const BUNDLES: Record<string, Json> = { en, sk, cs, de, pl, hu };

/** The four landmark / control aria-labels rendered by the core nav shell. */
const NAV_ARIA_KEYS = ['mainNavigation', 'publicNavigation', 'openMenu', 'closeMenu'] as const;

const APP_SHELL_SRC = readFileSync(
  resolve(process.cwd(), 'src/components/AppShell/AppShell.tsx'),
  'utf-8'
);

describe('core-nav aria-label i18n', () => {
  it('covers exactly the six shipped locale bundles', () => {
    expect(Object.keys(BUNDLES).sort()).toEqual(['cs', 'de', 'en', 'hu', 'pl', 'sk']);
  });

  for (const [locale, bundle] of Object.entries(BUNDLES)) {
    for (const key of NAV_ARIA_KEYS) {
      it(`${locale}.json defines a non-empty aria.${key}`, () => {
        const aria = bundle.aria as Json | undefined;
        const value = aria?.[key];
        expect(value, `aria.${key} missing in ${locale}.json`).toBeTypeOf('string');
        expect((value as string).trim().length).toBeGreaterThan(0);
      });
    }
  }

  it('AppShell routes every nav aria-label through the aria.* namespace', () => {
    for (const key of NAV_ARIA_KEYS) {
      expect(
        APP_SHELL_SRC.includes(`t('aria.${key}')`),
        `AppShell.tsx should label the nav via t('aria.${key}')`
      ).toBe(true);
    }
  });

  it('AppShell carries no hardcoded-English aria-label fallback', () => {
    // No inline defaultValue on an aria-label, and no shell.*-namespaced nav
    // aria-label — both re-introduce English copy into the component.
    expect(/aria-label=\{t\('shell\./.test(APP_SHELL_SRC)).toBe(false);
    expect(/aria-label=\{t\([^)]*defaultValue/.test(APP_SHELL_SRC)).toBe(false);
  });
});
