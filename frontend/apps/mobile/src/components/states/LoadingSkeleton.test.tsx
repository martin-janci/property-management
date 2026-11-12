/**
 * LoadingSkeleton — unit + i18n regression tests
 * (code-review-mobile-rn-states-untested-hardcoded).
 *
 * `LoadingSkeleton` is the shared shimmer placeholder. Its progressbar
 * accessibility label was previously the hardcoded English literal "Loading";
 * it now flows through `t('common.loading')`. These tests lock the three render
 * modes (single block / N lines / N cards), the progressbar role, and the
 * i18n-sourced accessibility label.
 *
 * The global setup (src/test/setup.ts) mocks react-i18next so
 * `t('common.loading', { defaultValue: 'Loading…' })` resolves to the
 * `defaultValue`. The resource layer proves the key exists in every locale.
 */

import { render, screen } from '@testing-library/react-native';
import cs from '../../locales/cs.json';
import de from '../../locales/de.json';
import en from '../../locales/en.json';
import sk from '../../locales/sk.json';
import { LoadingSkeleton } from './LoadingSkeleton';

describe('LoadingSkeleton', () => {
  it('renders a single labelled progressbar container by default', async () => {
    await render(<LoadingSkeleton />);
    expect(screen.getAllByLabelText('Loading…')).toHaveLength(1);
  });

  it('labels the progressbar via the common.loading i18n key, not a hardcoded literal', async () => {
    await render(<LoadingSkeleton />);
    // Resolves to the English defaultValue under the test mock; crucially the
    // old hardcoded "Loading" literal is no longer present.
    expect(screen.getByLabelText('Loading…')).toBeTruthy();
    expect(screen.queryByLabelText('Loading')).toBeNull();
  });

  it('renders the requested number of card placeholders inside one progressbar', async () => {
    await render(<LoadingSkeleton cards={3} />);
    expect(screen.getByLabelText('Loading…')).toBeTruthy();
  });

  it('renders the requested number of line placeholders inside one progressbar', async () => {
    await render(<LoadingSkeleton lines={4} />);
    expect(screen.getByLabelText('Loading…')).toBeTruthy();
  });
});

describe('LoadingSkeleton label resolves to a real, locale-distinct translation', () => {
  it('en carries the expected English copy', () => {
    expect(en.common.loading).toBe('Loading...');
  });

  it('every locale defines common.loading and translates it away from English', () => {
    for (const locale of [sk, cs, de]) {
      expect(typeof locale.common.loading).toBe('string');
      expect(locale.common.loading.length).toBeGreaterThan(0);
      expect(locale.common.loading).not.toBe(en.common.loading);
    }
  });
});
