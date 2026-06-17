import type { Locator, Page } from '@playwright/test';
import { Selectors } from '../selectors';
import type { Component } from './types';

/**
 * Primary navigation region. Resolves to the page's `banner`/`navigation`
 * landmark and exposes link lookups by accessible name.
 */
export class Nav implements Component {
  readonly root: Locator;
  private readonly sel: Selectors;

  constructor(page: Page) {
    // Prefer the testid'd nav, fall back to the ARIA navigation landmark.
    const testIdNav = page.getByTestId('primary-nav');
    this.root = testIdNav.or(page.getByRole('navigation')).first();
    this.sel = new Selectors(this.root);
  }

  /** A nav link by its accessible name. */
  link(name: string | RegExp): Locator {
    return this.sel.link(name);
  }

  /** All links inside the nav region. */
  links(): Locator {
    return this.sel.role('link');
  }
}
