import type { Locator, Page } from '@playwright/test';
import type { Component } from './types';

/**
 * Language switcher region (reality-web is i18n-first; ppt/admin may hide it).
 *
 * Resolves via `data-testid="language-switcher"`, then several ARIA fallbacks
 * so it works across the apps' differing markup until the testid sweep lands:
 *  - reality-web renders a `<button aria-label="Language: …">` popover trigger,
 *  - ppt-web renders a native `<select aria-label="Select language">`
 *    (a `combobox`), which is neither a button nor carries the testid.
 */
export class LanguageSwitcher implements Component {
  readonly root: Locator;

  constructor(page: Page) {
    const label = /language|select language|jazyk|sprache/i;
    this.root = page
      .getByTestId('language-switcher')
      .or(page.getByRole('button', { name: label }))
      .or(page.getByRole('combobox', { name: label }))
      .first();
  }

  /** Choose a locale by its visible option label. */
  option(locale: string | RegExp): Locator {
    return this.root
      .getByRole('option', { name: locale })
      .or(this.root.getByRole('menuitem', { name: locale }));
  }
}
