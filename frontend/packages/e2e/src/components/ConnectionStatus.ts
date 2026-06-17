import type { Locator, Page } from '@playwright/test';
import type { Component } from './types';

/**
 * Backend/connection status indicator. Used by route-health and offline
 * specs to assert whether the app rendered an "offline"/"degraded" banner.
 */
export class ConnectionStatus implements Component {
  readonly root: Locator;

  constructor(page: Page) {
    this.root = page.getByTestId('connection-status').or(page.getByRole('status')).first();
  }

  /** True if a connection-status indicator is currently visible. */
  isVisible(): Promise<boolean> {
    return this.root.isVisible();
  }
}
