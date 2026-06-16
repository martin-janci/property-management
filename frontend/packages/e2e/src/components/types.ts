import type { Locator } from '@playwright/test';

/**
 * A reusable UI region. Each component scopes itself to a single root
 * {@link Locator} so the same class composes into any page object.
 */
export interface Component {
  /** The component's root locator (for visibility/scoping assertions). */
  readonly root: Locator;
}
