import type { Locator, Page } from '@playwright/test';
import type { Component } from './types';

/**
 * Language switcher region (reality-web is i18n-first; ppt/admin may hide it).
 * Resolves via `data-testid="language-switcher"` with an ARIA fallback.
 */
export class LanguageSwitcher implements Component {
  readonly root: Locator;

  constructor(page: Page) {
    this.root = page
      .getByTestId('language-switcher')
      .or(page.getByRole('button', { name: /language|jazyk|sprache/i }))
      .first();
  }

  /** Choose a locale by its visible option label. */
  option(locale: string | RegExp): Locator {
    return this.root
      .getByRole('option', { name: locale })
      .or(this.root.getByRole('menuitem', { name: locale }));
  }
}
