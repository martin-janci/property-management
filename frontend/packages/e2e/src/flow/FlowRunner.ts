/**
 * FlowRunner — the engine that interprets a {@link UserFlow} from
 * `@ppt/sitemap`'s `allFlows` as an executable E2E. Specs-as-data: the flow
 * catalog is the spec, this class is the only interpreter.
 *
 * Step semantics (action.type):
 *  - `navigate` → `page.goto(target)`         (target is a path)
 *  - `fill`     → resolve `{{user.*}}` from {@link testUsers}, then fill the
 *                 target field. Object-valued fills (multi-field forms) are
 *                 best-effort: each `<key, value>` is filled into a field
 *                 located by testid/name/label `key`, skipping any not present.
 *  - `click`    → click the target
 *  - `submit`   → submit the target form (click its submit control, falling
 *                 back to pressing Enter in the form)
 *  - `wait`     → `waitMs` is honored as a *bounded* fallback for flow wait
 *                 steps only; everywhere else we wait on explicit conditions.
 *
 * Assertion semantics (assertion.type):
 *  - `url`          → the URL path matches `expected`
 *  - `visible`      → the target locator is visible/hidden per `expected`
 *  - `api_response` → asserts the status captured around the apiCall step; when
 *                     no backend is wired (`E2E_WITH_BACKEND` unset) the
 *                     assertion is annotated-and-skipped rather than failing.
 *
 * Selectors inside flows are already real CSS/testid strings
 * (`[data-testid="…"]`, `[name="…"]`), so they are used as-is via
 * {@link locate}'s `css` strategy — no re-derivation.
 *
 * Backend gating: a flow whose steps carry an `apiCall`, or whose
 * `requiredRole` is not anonymous, needs a live backend. {@link flowNeedsBackend}
 * reports this so the spec factory can `test.skip` under `@requires-backend`.
 */

import { expect, type Locator, type Page, type TestInfo } from '@playwright/test';
import type { FlowAction, FlowAssertion, FlowStep, UserFlow } from '@ppt/sitemap';
import { type TestRole, type TestUserCreds, testUsers } from '../fixtures';
import { locate } from '../testids';

/** Whether the runner is allowed to perform backend-dependent assertions. */
export function backendEnabled(): boolean {
  return process.env.E2E_WITH_BACKEND === '1';
}

/** Roles considered authenticated (anything but anonymous needs a session). */
function isAnonymous(role: string | undefined): boolean {
  return role === undefined || role === 'anonymous';
}

/**
 * Does this flow need a live backend to run honestly? True when any step makes
 * an API call, when any assertion checks an api_response, or when the flow's
 * required role is authenticated.
 */
export function flowNeedsBackend(flow: UserFlow): boolean {
  if (!isAnonymous(flow.requiredRole)) {
    return true;
  }
  return flow.steps.some(
    (step) =>
      Boolean(step.apiCall) || (step.assertions ?? []).some((a) => a.type === 'api_response')
  );
}

/** Map a flow's `requiredRole` onto a known {@link TestRole} for login. */
function roleForFlow(flow: UserFlow): TestRole {
  switch (flow.requiredRole) {
    case 'super_admin':
    case 'organization_admin':
      return 'admin';
    case 'manager':
    case 'technical_manager':
    case 'property_manager':
      return 'manager';
    default:
      return 'resident';
  }
}

/** Substitute `{{user.email}}` / `{{user.password}}` and other tokens. */
function resolveValue(raw: string, creds: TestUserCreds): string {
  return raw.replace(/\{\{\s*([\w.]+)\s*\}\}/g, (_full, token: string) => {
    switch (token) {
      case 'user.email':
        return creds.email;
      case 'user.password':
        return creds.password;
      default:
        // Non-user tokens (document.*, search.*, …) have no fixture source;
        // leave a deterministic, obviously-test placeholder rather than crash.
        return `e2e-${token.replace(/\./g, '-')}`;
    }
  });
}

/** Strip a trailing `:first-child` / `:nth-child(n)` and apply it as `.nth()`. */
function locatorFromSelector(page: Page, selector: string): Locator {
  const firstChild = selector.match(/^(.*):first-child$/);
  if (firstChild?.[1]) {
    return locate(page, { css: firstChild[1] }).first();
  }
  const nth = selector.match(/^(.*):nth-child\((\d+)\)$/);
  if (nth?.[1] && nth[2]) {
    return locate(page, { css: nth[1] }).nth(Number(nth[2]) - 1);
  }
  return locate(page, { css: selector });
}

/**
 * Interprets one {@link UserFlow}. Construct per test with the live `page`,
 * the resolved role credentials, and the optional {@link TestInfo} used for
 * annotations.
 */
export class FlowRunner {
  private readonly creds: TestUserCreds;

  constructor(
    private readonly page: Page,
    private readonly flow: UserFlow,
    private readonly testInfo?: TestInfo
  ) {
    this.creds = testUsers[roleForFlow(flow)];
  }

  /** Execute every step in `order`. */
  async run(): Promise<void> {
    const steps = [...this.flow.steps].sort((a, b) => a.order - b.order);
    for (const step of steps) {
      await this.runStep(step);
    }
  }

  private async runStep(step: FlowStep): Promise<void> {
    // Capture the API response around any step that declares an apiCall, so an
    // `api_response` assertion in the same step can read its status.
    const responsePromise =
      backendEnabled() && step.apiCall
        ? this.page.waitForResponse(() => true, { timeout: 15_000 }).catch(() => null)
        : null;

    if (step.action) {
      await this.runAction(step.action);
    }

    const status = responsePromise ? (await responsePromise)?.status() : undefined;

    for (const assertion of step.assertions ?? []) {
      await this.runAssertion(assertion, status);
    }
  }

  private async runAction(action: FlowAction): Promise<void> {
    switch (action.type) {
      case 'navigate': {
        if (action.target) {
          await this.page.goto(action.target, { waitUntil: 'domcontentloaded' });
        }
        return;
      }
      case 'fill': {
        await this.fill(action);
        return;
      }
      case 'click': {
        if (action.target) {
          await locatorFromSelector(this.page, action.target).click();
        }
        return;
      }
      case 'submit': {
        await this.submit(action.target);
        return;
      }
      case 'wait': {
        // Explicit-condition preferred; the bounded timeout is the documented
        // fallback for flow `wait` steps only.
        if (action.waitMs) {
          await this.page.waitForTimeout(Math.min(action.waitMs, 2_000));
        }
        return;
      }
      default:
        // `assert` / `api_call` action variants are expressed via assertions /
        // apiCall in this catalog; nothing imperative to do here.
        return;
    }
  }

  /** Fill a single field or a record of fields (best-effort per field). */
  private async fill(action: FlowAction): Promise<void> {
    const { target, value } = action;

    // Single-field fill: explicit target + string value.
    if (typeof value === 'string') {
      if (!target) {
        return;
      }
      await locatorFromSelector(this.page, target).fill(resolveValue(value, this.creds));
      return;
    }

    // Multi-field fill: object value, each key located by testid/name/label.
    if (value && typeof value === 'object') {
      for (const [key, raw] of Object.entries(value)) {
        const field = locate(this.page, {
          testId: key,
          css: `[name="${key}"]`,
          role: 'textbox',
          name: new RegExp(key, 'i'),
        });
        if ((await field.count()) === 0) {
          continue;
        }
        const text = typeof raw === 'string' ? resolveValue(raw, this.creds) : String(raw);
        await field
          .first()
          .fill(text)
          .catch(() => {
            /* field not fillable (select/checkbox) — skip in this generic pass */
          });
      }
    }
  }

  /** Submit a form: click its submit control, else press Enter inside it. */
  private async submit(target?: string): Promise<void> {
    const form = target ? locatorFromSelector(this.page, target) : this.page.locator('form');
    const submitButton = form
      .getByRole('button', { name: /submit|sign in|log ?in|save|upload|file|send|search/i })
      .or(form.locator('button[type="submit"]'))
      .first();
    if ((await submitButton.count()) > 0) {
      await submitButton.click();
      return;
    }
    // Fallback: press Enter in the first text field of the form.
    await form.press('Enter').catch(() => {
      /* form not directly pressable; nothing else to try generically */
    });
  }

  private async runAssertion(assertion: FlowAssertion, capturedStatus?: number): Promise<void> {
    switch (assertion.type) {
      case 'url': {
        const expectedPath = String(assertion.expected);
        await expect(this.page).toHaveURL(new RegExp(`${escapeRegExp(expectedPath)}/?$`));
        return;
      }
      case 'visible': {
        if (!assertion.target) {
          return;
        }
        const locator = locatorFromSelector(this.page, assertion.target);
        if (assertion.expected === false) {
          await expect(locator).toBeHidden();
        } else {
          await expect(locator).toBeVisible();
        }
        return;
      }
      case 'api_response': {
        if (!backendEnabled()) {
          this.annotate(
            'skip-assertion',
            `api_response (${String(assertion.expected)}) — backend not enabled (E2E_WITH_BACKEND unset)`
          );
          return;
        }
        expect(capturedStatus, 'no API response was captured for this step').toBeDefined();
        expect(capturedStatus).toBe(Number(assertion.expected));
        return;
      }
      default:
        // `hidden` / `text` / `state` are not emitted by the current catalog;
        // no-op keeps the runner total over the assertion union.
        return;
    }
  }

  private annotate(type: string, description: string): void {
    this.testInfo?.annotations.push({ type, description });
  }
}

function escapeRegExp(input: string): string {
  return input.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}
