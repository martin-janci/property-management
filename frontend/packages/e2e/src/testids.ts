/**
 * data-testid strategy for the PPT E2E framework.
 *
 * Three layers, smallest surface first:
 *
 *  1. {@link tid} — a *builder* that enforces the kebab
 *     `<feature>-<subject>[-<qualifier>]-<elementType>` convention so Page
 *     Objects compose ids by rule rather than hardcoding 1800+ literals.
 *  2. {@link testIds} — a small *curated registry* of the verified
 *     cross-cutting ids (nav.*, auth.*, plus a handful of app-shell markers).
 *     Per-feature ids are NOT enumerated here — Page Objects build them with
 *     {@link tid}.
 *  3. {@link byTestId} / {@link locate} — *lookup* helpers. `byTestId` is the
 *     direct `data-testid` resolve; `locate` is the resilient testid-first →
 *     role/text → css fallback used everywhere the underlying id may not yet
 *     exist in the checkout (the component sweep is still uncommitted).
 *
 * No XPath, ever. The element-type suffix is a closed set so a typo in a
 * composed id fails fast at authoring time.
 */

import type { Locator, Page } from '@playwright/test';

// ---------------------------------------------------------------------------
// Convention: element-type suffixes
// ---------------------------------------------------------------------------

/** The closed set of element-type suffixes the convention permits. */
export const ELEMENT_TYPES = [
  'input',
  'textarea',
  'select',
  'checkbox',
  'button',
  'link',
  'form',
  'overlay',
  'backdrop',
  'canvas',
] as const;

/** A valid trailing element-type for a composed testid. */
export type ElementType = (typeof ELEMENT_TYPES)[number];

function isElementType(value: string): value is ElementType {
  return (ELEMENT_TYPES as readonly string[]).includes(value);
}

/** Lower-kebab a single id fragment (defensive against stray casing/spaces). */
function kebab(part: string | number): string {
  return String(part)
    .trim()
    .replace(/([a-z0-9])([A-Z])/g, '$1-$2')
    .replace(/[\s_]+/g, '-')
    .toLowerCase();
}

/**
 * Build a convention-compliant `data-testid` from a feature, any number of
 * subject/qualifier parts, and a trailing element type:
 *
 *   tid('login', 'email', 'input')         // 'login-email-input'
 *   tid('documents', 'pagination', 'next', 'button')
 *                                           // 'documents-pagination-next-button'
 *   tid('announcements', 'card', 'edit', someId, 'button')
 *                                           // 'announcements-card-edit-<id>-button'
 *
 * The final argument MUST be one of {@link ELEMENT_TYPES}; otherwise this
 * throws, catching convention drift at authoring time rather than producing a
 * silently-wrong selector. Container/state-marker ids that have no element type
 * (e.g. `dashboard`) are NOT built with `tid` — reference them via
 * {@link testIds} or {@link byTestId} directly.
 */
export function tid(feature: string, ...rest: Array<string | number>): string {
  if (rest.length === 0) {
    throw new Error(
      `tid('${feature}'): need at least an element type — e.g. tid('${feature}', 'submit', 'button')`
    );
  }
  const last = String(rest[rest.length - 1]);
  if (!isElementType(last)) {
    throw new Error(
      `tid('${feature}', …, '${last}'): '${last}' is not a valid element type. ` +
        `Expected one of: ${ELEMENT_TYPES.join(', ')}.`
    );
  }
  return [feature, ...rest].map(kebab).join('-');
}

/** Pagination helper: `tidPagination('documents','next')` → `documents-pagination-next-button`. */
export function tidPagination(
  feature: string,
  direction: 'next' | 'prev',
  variant?: 'mobile'
): string {
  return variant
    ? tid(feature, 'pagination', direction, variant, 'button')
    : tid(feature, 'pagination', direction, 'button');
}

/** Filter helper: `tidFilter('listings','city','select')` → `listings-filter-city-select`. */
export function tidFilter(
  feature: string,
  name: string,
  elementType: Extract<ElementType, 'select' | 'checkbox' | 'input'>
): string {
  return tid(feature, 'filter', name, elementType);
}

// ---------------------------------------------------------------------------
// Curated registry of verified cross-cutting ids
// ---------------------------------------------------------------------------

/**
 * Cross-cutting `data-testid`s that appear across many screens and are verified
 * present in the sweep. Per-feature ids are intentionally absent — Page Objects
 * compose those with {@link tid}.
 */
export const testIds = Object.freeze({
  nav: Object.freeze({
    home: 'nav-home-link',
    documents: 'nav-documents-link',
    news: 'nav-news-link',
    emergency: 'nav-emergency-link',
    disputes: 'nav-disputes-link',
    outages: 'nav-outages-link',
    accessibility: 'nav-accessibility-link',
    privacy: 'nav-privacy-link',
    logout: 'nav-logout-button',
    signin: 'nav-signin-link',
  }),
  auth: Object.freeze({
    loginForm: 'login-form',
    homeSigninButton: 'home-signin-button',
    sessionExpiredSigninButton: 'errors-session-expired-signin-button',
    logoutButton: 'logout-btn',
    confirmLogout: 'confirm-logout',
    forgotPasswordForm: 'forgot-password-form',
    forgotPasswordEmailInput: 'forgot-password-email-input',
    forgotPasswordSubmitButton: 'forgot-password-submit-button',
    forgotPasswordLoginLink: 'forgot-password-login-link',
    registerEmailInput: 'register-email-input',
    registerPasswordInput: 'register-password-input',
    registerConfirmPasswordInput: 'register-confirm-password-input',
    registerLoginLink: 'register-login-link',
    resetPasswordForm: 'reset-password-form',
    resetPasswordPasswordInput: 'reset-password-password-input',
    resetPasswordConfirmPasswordInput: 'reset-password-confirm-password-input',
    resetPasswordSubmitButton: 'reset-password-submit-button',
    resetPasswordSignInButton: 'reset-password-sign-in-button',
    changePasswordForm: 'change-password-form',
    changePasswordCurrentInput: 'change-password-current-input',
    changePasswordNewInput: 'change-password-new-input',
    changePasswordConfirmInput: 'change-password-confirm-input',
    changePasswordSubmitButton: 'change-password-submit-button',
  }),
  common: Object.freeze({
    dashboard: 'dashboard',
    toastSuccess: 'toast-success',
    commandPaletteOverlay: 'command-palette-overlay',
    connectionStatus: 'connection-status-indicator',
  }),
} as const);

/** Type of the curated registry (for typed access in Page Objects). */
export type TestIdRegistry = typeof testIds;

// ---------------------------------------------------------------------------
// Lookups
// ---------------------------------------------------------------------------

/** Direct `data-testid` resolve, scoped to a page or locator. */
export function byTestId(scope: Page | Locator, id: string): Locator {
  return scope.getByTestId(id);
}

/** ARIA role accepted by {@link locate} (mirrors Playwright's role union). */
type AriaRole = Parameters<Page['getByRole']>[0];

/** A resilient locator descriptor — at least one strategy must be supplied. */
export interface LocateOptions {
  /** Most resilient: a `data-testid`. Tried first. */
  readonly testId?: string;
  /** ARIA role; pair with {@link name} for an accessible-name match. */
  readonly role?: AriaRole;
  /** Accessible name / text for the role lookup. */
  readonly name?: string | RegExp;
  /** Visible-text fallback (when no role is appropriate). */
  readonly text?: string | RegExp;
  /** Last-resort CSS (no XPath). Use for `[name="email"]`-style flow selectors. */
  readonly css?: string;
}

/**
 * Resolve a locator using the framework's resilient priority order:
 * `data-testid` → ARIA role+name → visible text → CSS. Each provided strategy
 * is `.or()`-chained so the FIRST present wins at match time — letting specs
 * author against real testids that resolve once the component sweep merges,
 * while degrading gracefully (role/text/css) where the id is not yet in the
 * checkout. At least one strategy must be supplied.
 *
 * `.first()` keeps the result a single deterministic locator even when more
 * than one strategy matches.
 */
export function locate(scope: Page | Locator, options: LocateOptions): Locator {
  const candidates: Locator[] = [];
  if (options.testId) {
    candidates.push(scope.getByTestId(options.testId));
  }
  if (options.role) {
    candidates.push(
      options.name === undefined
        ? scope.getByRole(options.role)
        : scope.getByRole(options.role, { name: options.name })
    );
  }
  if (options.text !== undefined) {
    candidates.push(scope.getByText(options.text));
  }
  if (options.css) {
    candidates.push(scope.locator(options.css));
  }
  const first = candidates[0];
  if (!first) {
    throw new Error('locate(): supply at least one of testId / role / text / css.');
  }
  return candidates.reduce((acc, next) => acc.or(next)).first();
}
