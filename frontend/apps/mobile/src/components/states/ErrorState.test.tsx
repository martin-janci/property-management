/**
 * ErrorState — unit + i18n regression tests
 * (code-review-mobile-rn-states-untested-hardcoded).
 *
 * `ErrorState` is the shared full-screen error placeholder. It previously
 * shipped its default title/description/retry copy as hardcoded English
 * literals ("Something went wrong", "Try again", …) even though react-i18next
 * was already wired. These tests lock the new contract: the intrinsic copy now
 * flows through `t()` (with an English `defaultValue` fallback), caller props
 * still override it, and the retry button is wired to `onRetry`.
 *
 * The global setup (src/test/setup.ts) mocks react-i18next so
 * `t(key, { defaultValue })` resolves to the `defaultValue` — mirroring real
 * i18next behaviour for untranslated keys. The resource-layer block below then
 * proves the keys exist in every locale and are translated away from English.
 */

import { fireEvent, render, screen } from '@testing-library/react-native';
import cs from '../../locales/cs.json';
import de from '../../locales/de.json';
import en from '../../locales/en.json';
import sk from '../../locales/sk.json';
import { ErrorState } from './ErrorState';

describe('ErrorState', () => {
  it('renders the i18n-sourced default title and description', async () => {
    await render(<ErrorState />);
    expect(screen.getByText('Something went wrong')).toBeTruthy();
    expect(
      screen.getByText('Please try again. If the issue persists, contact support.')
    ).toBeTruthy();
  });

  it('prefers caller-provided (already localized) title and description over the defaults', async () => {
    // Routed through `{expr}` props so the no-hardcoded-strings biome plugin
    // stays quiet on intentional test data (it rightly flags literal JSX copy).
    const title = 'Could not load faults';
    const description = 'Pull to refresh.';
    await render(<ErrorState title={title} description={description} />);
    expect(screen.getByText(title)).toBeTruthy();
    expect(screen.getByText(description)).toBeTruthy();
    // The default copy must not leak through when an override is given.
    expect(screen.queryByText('Something went wrong')).toBeNull();
  });

  it('renders no retry button when onRetry is omitted', async () => {
    await render(<ErrorState />);
    expect(screen.queryByText('Try again')).toBeNull();
  });

  it('renders a retry button with the i18n-sourced default label and calls onRetry', async () => {
    const onRetry = jest.fn();
    await render(<ErrorState onRetry={onRetry} />);

    const button = screen.getByText('Try again');
    expect(button).toBeTruthy();

    await fireEvent.press(button);
    expect(onRetry).toHaveBeenCalledTimes(1);
  });

  it('honours a caller-provided retry label', async () => {
    await render(<ErrorState onRetry={jest.fn()} retryLabel="Reload" />);
    expect(screen.getByText('Reload')).toBeTruthy();
    expect(screen.queryByText('Try again')).toBeNull();
  });
});

describe('ErrorState default copy resolves to real, locale-distinct translations', () => {
  it('en carries the expected English copy', () => {
    expect(en.common.errorTitle).toBe('Something went wrong');
    expect(en.common.errorDescription).toBe(
      'Please try again. If the issue persists, contact support.'
    );
    expect(en.common.tryAgain).toBe('Try again');
  });

  it('every locale defines the keys and translates them away from English', () => {
    for (const locale of [sk, cs, de]) {
      for (const key of ['errorTitle', 'errorDescription', 'tryAgain'] as const) {
        expect(typeof locale.common[key]).toBe('string');
        expect(locale.common[key].length).toBeGreaterThan(0);
        expect(locale.common[key]).not.toBe(en.common[key]);
      }
    }
  });
});
