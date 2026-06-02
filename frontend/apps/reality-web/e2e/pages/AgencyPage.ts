/**
 * AgencyPage — agency management dashboard (`reality-agency`, role-gated).
 *
 * Restricted to `real_estate_agent` / `organization_admin`. The header renders
 * the agency name as the `<h1>` with an "Agency Dashboard" subtitle and a
 * performance-overview region. Selectors are testid-first with role/text
 * fallback via {@link locate}; the in-page "Sign in required" guard is exposed
 * so unauthenticated specs can assert it without a session.
 */

import type { Locator, Page } from '@playwright/test';
import { BasePage, type GotoOptions, locate, sitemapPage } from '@ppt/e2e';
import { APP, type RealityLocale, withLocale } from './support';

export class AgencyPage extends BasePage {
  private readonly resolvedPath: string;

  constructor(page: Page, locale?: RealityLocale) {
    super(page);
    this.resolvedPath = withLocale(sitemapPage(page, APP, 'reality-agency').path, locale);
  }

  get path(): string {
    return this.resolvedPath;
  }

  override async goto(options?: GotoOptions): Promise<void> {
    await super.goto(options);
    await this.ready().waitFor({ state: 'visible' });
  }

  /** Settled state: the dashboard subtitle (authed) or the sign-in guard. */
  ready(): Locator {
    return this.subtitle().or(this.signInRequired());
  }

  /** The agency name heading (`<h1>`). */
  agencyName(): Locator {
    return this.heading().first();
  }

  /** "Agency Dashboard" subtitle text. */
  subtitle(): Locator {
    return locate(this.main(), {
      testId: 'agency-dashboard-subtitle',
      text: /agency dashboard|prehľad/i,
    });
  }

  /** "Performance Overview" section heading. */
  performanceOverview(): Locator {
    return locate(this.main(), {
      testId: 'agency-performance-overview',
      role: 'heading',
      text: /performance overview|výkon/i,
    });
  }

  /** "Manage Realtors" quick action. */
  manageRealtorsAction(): Locator {
    return locate(this.main(), {
      testId: 'agency-manage-realtors-link',
      role: 'link',
      name: /manage realtors|makléri/i,
    });
  }

  /** The "Sign in required" guard shown to unauthenticated visitors. */
  signInRequired(): Locator {
    return locate(this.main(), {
      testId: 'sign-in-required',
      role: 'heading',
      text: /sign in required|prihlás/i,
    });
  }
}
