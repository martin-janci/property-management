/**
 * WidgetSettingsScreen — i18n regression tests.
 *
 * Part 1 (code-review-mobile-rn-alert-hardcoded-en): the screen raised its two
 * confirmation Alerts with hardcoded English literals — `Alert.alert('Widget
 * Added', '<name> widget has been configured.')` and `Alert.alert('Widgets
 * Updated', 'All widgets have been refreshed.')`. That leak is now sourced from
 * `widgets.addedTitle/addedMessage/updatedTitle/updatedMessage` via `t()`.
 *
 * Part 2 (code-review-mobile-rn-widget-settings-i18n-leak): the rest of the
 * screen's chrome — the header title + Back control, the "Widgets Not
 * Available" fallback, the Available/Active section titles & descriptions, the
 * active-widget size/refresh meta line, the Remove button, the "Refresh All
 * Widgets" button and the "How to Add Widgets" instructions — was still
 * hardcoded English. That copy is now sourced from `widgets.*` / `common.*`
 * i18n keys via `t()`, with matching entries in every locale JSON.
 *
 * Part 3 (code-review-mobile-rn-widget-settings-alerts-i18n): every `t()` call
 * (UI copy + the native Alert titles/messages) is now the object form
 * `t(key, { defaultValue })` — matching the app-wide convention
 * (VoteDetailScreen, ArticleDetailScreen) so an untranslated key degrades to a
 * readable English fallback instead of leaking the raw key. This suite locks
 * that contract with a *spyable* `t` (mirroring VoteDetailScreen.test): the
 * render layer asserts the resolved fallback text AND that each string is
 * routed through `t(key, { defaultValue })`; the resource layer asserts the
 * locale JSON carries those keys with real translations.
 */
import { fireEvent, render, screen, waitFor } from '@testing-library/react-native';
import { Alert } from 'react-native';
import cs from '../../locales/cs.json';
import de from '../../locales/de.json';
import en from '../../locales/en.json';
import hu from '../../locales/hu.json';
import pl from '../../locales/pl.json';
import sk from '../../locales/sk.json';
import { WidgetSettingsScreen } from './WidgetSettingsScreen';

// Override the global `react-i18next` mock (src/test/setup.ts) with a *spyable*
// `t` so these suites can assert that each string is routed THROUGH
// `t(key, { defaultValue })`. The behavior is identical to the global mock
// (`defaultValue ?? key`), so the resolved fallback text matches en.json.
// `mock`-prefixed names are allowed inside a hoisted jest.mock factory.
const mockT = jest.fn((key: string, opts?: { defaultValue?: string }) => opts?.defaultValue ?? key);
jest.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: mockT,
    i18n: { language: 'en', changeLanguage: jest.fn() },
  }),
  initReactI18next: { type: '3rdParty', init: jest.fn() },
}));

// No available widget types so no RN <Switch> renders — jest-expo 56's preset
// can't transform the Switch native component (same codegen-spec limitation the
// DebuggingOverlay/assets-registry stubs work around), so rendering one crashes
// the suite. WidgetSettingsScreen.auth.test.tsx avoids it the same way. The
// refresh button (always rendered) exercises the "widgets updated" Alert; the
// "widget added" Alert copy is pinned at the resource layer below.
jest.mock('../../widgets', () => {
  class MockWidgetBridge {
    setAuthToken = jest.fn();
    getAvailableWidgetTypes = jest.fn(() => []);
    isWidgetSupported = jest.fn(async () => true);
    getAllConfigs = jest.fn(async () => []);
    configureWidget = jest.fn(async () => {});
    removeWidget = jest.fn(async () => {});
    updateAllWidgets = jest.fn(async () => {});
  }
  return { WidgetBridge: MockWidgetBridge };
});

jest.mock('../../contexts', () => ({
  useAuth: () => ({ accessToken: 'token' }),
}));

describe('WidgetSettingsScreen Alert i18n', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('raises the "widgets updated" Alert via object-form t(key, { defaultValue })', async () => {
    const alertSpy = jest.spyOn(Alert, 'alert').mockImplementation(() => {});
    await render(<WidgetSettingsScreen onNavigate={jest.fn()} />);

    const refresh = await screen.findByText('🔄 Refresh All Widgets');
    fireEvent.press(refresh);

    await waitFor(() => {
      // Resolved fallback text (the real Alert the user sees)…
      expect(alertSpy).toHaveBeenCalledWith('Widgets Updated', 'All widgets have been refreshed.');
    });
    // …and proof the titles/messages are routed through i18n keys, not literals.
    expect(mockT).toHaveBeenCalledWith(
      'widgets.updatedTitle',
      expect.objectContaining({ defaultValue: 'Widgets Updated' })
    );
    expect(mockT).toHaveBeenCalledWith(
      'widgets.updatedMessage',
      expect.objectContaining({ defaultValue: 'All widgets have been refreshed.' })
    );
    alertSpy.mockRestore();
  });
});

describe('WidgetSettingsScreen static copy i18n', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('renders the screen chrome via object-form t(key, { defaultValue })', async () => {
    await render(<WidgetSettingsScreen onNavigate={jest.fn()} />);

    // Resolved fallback text renders (via each key's defaultValue).
    expect(await screen.findByText('Widget Settings')).toBeTruthy();
    expect(screen.getByText('← Back')).toBeTruthy();
    expect(screen.getByText('Available Widgets')).toBeTruthy();
    expect(screen.getByText('Configure which widgets appear on your home screen.')).toBeTruthy();
    expect(screen.getByText('Active Widgets')).toBeTruthy();
    expect(screen.getByText('No widgets configured yet.')).toBeTruthy();
    expect(screen.getByText('🔄 Refresh All Widgets')).toBeTruthy();
    expect(screen.getByText('How to Add Widgets')).toBeTruthy();

    // Each string is routed through an i18n key with an explicit defaultValue —
    // not a hardcoded literal.
    expect(mockT).toHaveBeenCalledWith(
      'widgets.settingsTitle',
      expect.objectContaining({ defaultValue: 'Widget Settings' })
    );
    expect(mockT).toHaveBeenCalledWith(
      'common.back',
      expect.objectContaining({ defaultValue: 'Back' })
    );
    expect(mockT).toHaveBeenCalledWith(
      'widgets.availableTitle',
      expect.objectContaining({ defaultValue: 'Available Widgets' })
    );
    expect(mockT).toHaveBeenCalledWith(
      'widgets.refreshAll',
      expect.objectContaining({ defaultValue: '🔄 Refresh All Widgets' })
    );
    expect(mockT).toHaveBeenCalledWith(
      'widgets.instructionsTitle',
      expect.objectContaining({ defaultValue: 'How to Add Widgets' })
    );
  });
});

describe('widgets i18n keys resolve to real, locale-defined translations', () => {
  const alertKeys = ['addedTitle', 'addedMessage', 'updatedTitle', 'updatedMessage'] as const;
  const copyKeys = [
    'emptyText',
    'settingsTitle',
    'notAvailableTitle',
    'notAvailableIos',
    'notAvailableAndroid',
    'availableTitle',
    'availableDescription',
    'activeTitle',
    'metaInfo',
    'remove',
    'refreshAll',
    'instructionsTitle',
    'instructionsIos',
    'instructionsAndroid',
  ] as const;

  it('en carries the expected English copy with placeholders intact', () => {
    expect(en.widgets.addedTitle).toBe('Widget Added');
    expect(en.widgets.addedMessage).toContain('{{name}}');
    expect(en.widgets.updatedTitle).toBe('Widgets Updated');
    expect(en.widgets.updatedMessage).toBe('All widgets have been refreshed.');
    expect(en.widgets.settingsTitle).toBe('Widget Settings');
    expect(en.widgets.refreshAll).toBe('🔄 Refresh All Widgets');
    expect(en.widgets.metaInfo).toContain('{{size}}');
    expect(en.widgets.metaInfo).toContain('{{interval}}');
  });

  it('every locale defines all widget keys, with {{name}} and the meta placeholders preserved', () => {
    for (const locale of [en, sk, cs, de, pl, hu]) {
      for (const key of [...alertKeys, ...copyKeys]) {
        const value = (locale.widgets as Record<string, string>)[key];
        expect(typeof value).toBe('string');
        expect(value.length).toBeGreaterThan(0);
      }
      expect(locale.widgets.addedMessage).toContain('{{name}}');
      expect(locale.widgets.metaInfo).toContain('{{size}}');
      expect(locale.widgets.metaInfo).toContain('{{interval}}');
    }
  });
});
