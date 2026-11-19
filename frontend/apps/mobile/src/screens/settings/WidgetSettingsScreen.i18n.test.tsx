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
 * hardcoded English. This suite locks the contract: that copy is now sourced
 * from `widgets.*` / `common.*` i18n keys via `t()`, with matching entries in
 * every locale JSON.
 *
 * Mirrors PersonMonthsScreen.i18n.test.tsx: the render layer asserts the UI is
 * built from i18n KEYS (global setup mocks `t(key) => key`); the resource layer
 * asserts the locale JSON carries those keys with real translations.
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

  it('raises the "widgets updated" Alert via i18n keys, not hardcoded literals', async () => {
    const alertSpy = jest.spyOn(Alert, 'alert').mockImplementation(() => {});
    await render(<WidgetSettingsScreen onNavigate={jest.fn()} />);

    const refresh = await screen.findByText('widgets.refreshAll');
    fireEvent.press(refresh);

    await waitFor(() => {
      expect(alertSpy).toHaveBeenCalledWith('widgets.updatedTitle', 'widgets.updatedMessage');
    });
    alertSpy.mockRestore();
  });
});

describe('WidgetSettingsScreen static copy i18n', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('renders the screen chrome from i18n keys, not hardcoded English', async () => {
    await render(<WidgetSettingsScreen onNavigate={jest.fn()} />);

    // Header + Back control (Back reuses the shared common.back key).
    expect(await screen.findByText('widgets.settingsTitle')).toBeTruthy();
    expect(screen.getByText('← common.back')).toBeTruthy();

    // Section titles / descriptions.
    expect(screen.getByText('widgets.availableTitle')).toBeTruthy();
    expect(screen.getByText('widgets.availableDescription')).toBeTruthy();
    expect(screen.getByText('widgets.activeTitle')).toBeTruthy();

    // Empty-state, refresh action and instructions block.
    expect(screen.getByText('widgets.emptyText')).toBeTruthy();
    expect(screen.getByText('widgets.refreshAll')).toBeTruthy();
    expect(screen.getByText('widgets.instructionsTitle')).toBeTruthy();

    // No raw English leaked through.
    expect(screen.queryByText('Widget Settings')).toBeNull();
    expect(screen.queryByText('Available Widgets')).toBeNull();
    expect(screen.queryByText('🔄 Refresh All Widgets')).toBeNull();
    expect(screen.queryByText('How to Add Widgets')).toBeNull();
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
