/**
 * Mobile configuration page object (`/platform/mobile`).
 *
 * Models `apps/admin-web/src/pages/MobileConfigPage.tsx`: an `<h1>` "Mobile
 * configuration" plus two `<SettingsForm>` sections — "Force-update floor"
 * (min iOS / Android version inputs) and "React Native feature flags". Each
 * `SettingsForm` (from `@ppt/admin-ui`) renders a `<form>` ending in a Save /
 * Read-only submit button; the button label flips to "Read-only" when the
 * principal lacks `mobile_config_write`.
 *
 * Selectors are testid-first with a role/text/css fallback via `locate`.
 */

import type { Locator } from '@playwright/test';
import { locate } from '@ppt/e2e';
import { AdminShellPage } from './AdminShellPage';
import { ADMIN_ROUTES } from './routes';

export class MobileConfigPage extends AdminShellPage {
  get path(): string {
    return ADMIN_ROUTES.mobileConfig;
  }

  /** Page heading ("Mobile configuration"). */
  title(): Locator {
    return locate(this.page, {
      testId: 'admin-mobile-title',
      role: 'heading',
      name: /mobile configuration/i,
    });
  }

  /** "Force-update floor" section heading (`<h2>`). */
  forceUpdateSection(): Locator {
    return locate(this.page, {
      testId: 'admin-mobile-force-update-heading',
      role: 'heading',
      name: /force-update floor/i,
    });
  }

  /** "React Native feature flags" section heading (`<h2>`). */
  featureFlagsSection(): Locator {
    return locate(this.page, {
      testId: 'admin-mobile-flags-heading',
      role: 'heading',
      name: /react native feature flags/i,
    });
  }

  /** Minimum iOS version field. */
  minIosVersionInput(): Locator {
    return locate(this.page, {
      testId: 'admin-mobile-min-ios-version-input',
      role: 'textbox',
      name: /minimum ios version/i,
    });
  }

  /** Minimum Android version field. */
  minAndroidVersionInput(): Locator {
    return locate(this.page, {
      testId: 'admin-mobile-min-android-version-input',
      role: 'textbox',
      name: /minimum android version/i,
    });
  }

  /**
   * The Save / Read-only submit control of a settings form. Two forms render on
   * this page; pass `index` to target the second (`1`). Label is "Save" when
   * writable, "Read-only" when gated — so we match either. We scope the button
   * to its form first so `index` selects the form, not a flattened button list.
   */
  saveButton(index = 0): Locator {
    return locate(this.settingsForm(index), {
      role: 'button',
      name: /save|read-only/i,
    });
  }

  /**
   * A settings `<form>` on the page (force-update = 0, flags = 1).
   *
   * Built without `locate`'s baked-in `.first()` so `.nth(index)` can target
   * the second form; the testid OR css candidates are `.or()`-chained directly.
   */
  settingsForm(index = 0): Locator {
    return this.page
      .getByTestId('admin-mobile-settings-form')
      .or(this.page.locator('form.ppt-admin-settings-form'))
      .nth(index);
  }
}
