/**
 * Abstract base for all page objects.
 *
 * Subclasses provide a {@link path} (concrete URL path, locale/params already
 * applied) and reuse the shared region components via composition. No subclass
 * should ever call `page.goto` with a raw string or hand-roll selectors —
 * everything goes through here and {@link Selectors}.
 */

import type { Locator, Page } from '@playwright/test';
import { ConnectionStatus, LanguageSwitcher, Nav } from '../components';
import { Selectors } from '../selectors';

/** Options for {@link BasePage.goto}. */
export interface GotoOptions {
  /** Override Playwright's wait-until strategy. Default: 'domcontentloaded'. */
  readonly waitUntil?: 'load' | 'domcontentloaded' | 'commit';
  /** Skip the {@link BasePage.waitReady} call after navigation. */
  readonly skipReady?: boolean;
}

export abstract class BasePage {
  protected readonly page: Page;
  protected readonly sel: Selectors;

  /** Shared UI regions — composed, not inherited. */
  protected readonly navRegion: Nav;
  protected readonly languageSwitcher: LanguageSwitcher;
  protected readonly connectionStatus: ConnectionStatus;

  constructor(page: Page) {
    this.page = page;
    this.sel = new Selectors(page);
    this.navRegion = new Nav(page);
    this.languageSwitcher = new LanguageSwitcher(page);
    this.connectionStatus = new ConnectionStatus(page);
  }

  /**
   * The concrete URL path this page lives at (e.g. `/login`, `/sk/listings`).
   * Subclasses resolve params/locale before returning.
   */
  abstract get path(): string;

  /** Navigate to {@link path} and (by default) wait until ready. */
  async goto(options: GotoOptions = {}): Promise<void> {
    await this.page.goto(this.path, {
      waitUntil: options.waitUntil ?? 'domcontentloaded',
    });
    if (!options.skipReady) {
      await this.waitReady();
    }
  }

  /**
   * Wait until the page has settled: the `main` landmark is attached and the
   * document body is visible. Uses explicit element waits — never a fixed
   * timeout. "networkidle-lite": we wait for `main` rather than full network
   * idle, which is flaky under long-poll/websocket apps.
   */
  async waitReady(): Promise<void> {
    await this.page.waitForLoadState('domcontentloaded');
    await this.main().waitFor({ state: 'attached' });
  }

  /** The `<main>` landmark (or its testid'd equivalent). */
  main(): Locator {
    return this.page.getByRole('main').or(this.page.getByTestId('main')).first();
  }

  /** Header/banner landmark. */
  header(): Locator {
    return this.page.getByRole('banner').or(this.page.getByTestId('app-header')).first();
  }

  /** Primary navigation component. */
  nav(): Nav {
    return this.navRegion;
  }

  /** Language switcher component. */
  language(): LanguageSwitcher {
    return this.languageSwitcher;
  }

  /** Connection-status component. */
  connection(): ConnectionStatus {
    return this.connectionStatus;
  }

  /** Resilient selectors scoped to the whole page. */
  locators(): Selectors {
    return this.sel;
  }

  /** Scope resilient selectors to a sub-region of this page. */
  region(root: Locator): Selectors {
    return this.sel.within(root);
  }

  /** The page heading (first `<h1>`/heading landmark). */
  heading(name?: string | RegExp): Locator {
    return this.sel.heading(name);
  }
}
