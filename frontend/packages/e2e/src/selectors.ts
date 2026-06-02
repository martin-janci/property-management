/**
 * Centralized, resilient locator strategy.
 *
 * The whole framework funnels selector creation through here so individual
 * specs never hand-roll fragile selectors. Priority order (most → least
 * resilient): `data-testid` → ARIA role + accessible name → label/placeholder
 * → visible text. No XPath. No CSS that couples to styling.
 */

import type { Locator, Page } from '@playwright/test';

/** Roles we expose first-class helpers for (subset of ARIA roles). */
type Role = Parameters<Page['getByRole']>[0];

/** Options accepted by role-based lookups. */
export interface ByRoleOptions {
  readonly name?: string | RegExp;
  readonly exact?: boolean;
}

/**
 * `Selectors` wraps a {@link Page} (or any {@link Locator} scope) and yields
 * resilient locators. Construct one per scope; never store raw strings.
 */
export class Selectors {
  constructor(private readonly scope: Page | Locator) {}

  /** Re-scope to a region (e.g. nav, a dialog) and keep the resilient API. */
  within(region: Locator): Selectors {
    return new Selectors(region);
  }

  /** Most resilient: `data-testid`. Prefer this for app-owned elements. */
  testId(id: string): Locator {
    return this.scope.getByTestId(id);
  }

  /** ARIA role + accessible name. The default for buttons/links/headings. */
  role(role: Role, options?: ByRoleOptions): Locator {
    return this.scope.getByRole(role, options);
  }

  /** A button by its accessible name. */
  button(name: string | RegExp): Locator {
    return this.scope.getByRole('button', { name });
  }

  /** A link by its accessible name. */
  link(name: string | RegExp): Locator {
    return this.scope.getByRole('link', { name });
  }

  /** A heading by its accessible name (omit to match any heading). */
  heading(name?: string | RegExp): Locator {
    return name === undefined
      ? this.scope.getByRole('heading')
      : this.scope.getByRole('heading', { name });
  }

  /** A form field by its associated label. */
  label(text: string | RegExp, exact = false): Locator {
    return this.scope.getByLabel(text, { exact });
  }

  /** A field by placeholder (use only when no label exists). */
  placeholder(text: string | RegExp): Locator {
    return this.scope.getByPlaceholder(text);
  }

  /** Visible text — last resort, prefer role/testId. */
  text(text: string | RegExp): Locator {
    return this.scope.getByText(text);
  }
}

/** Convenience: build a {@link Selectors} bound to the page root. */
export function selectors(page: Page): Selectors {
  return new Selectors(page);
}

/** Shorthand for the single most common case. */
export function testId(scope: Page | Locator, id: string): Locator {
  return scope.getByTestId(id);
}
