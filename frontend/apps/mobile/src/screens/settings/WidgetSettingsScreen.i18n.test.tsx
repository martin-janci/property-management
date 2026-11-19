/**
 * WidgetSettingsScreen — Alert i18n regression test
 * (code-review-mobile-rn-alert-hardcoded-en).
 *
 * The screen raised its two confirmation Alerts with hardcoded English
 * literals — `Alert.alert('Widget Added', '<name> widget has been
 * configured.')` and `Alert.alert('Widgets Updated', 'All widgets have been
 * refreshed.')` — even though `widgets.addedTitle/addedMessage/updatedTitle/
 * updatedMessage` already existed in every locale JSON. This suite locks the
 * contract: both Alerts are now sourced from i18n keys via `t()`.
 *
 * Mirrors PersonMonthsScreen.i18n.test.tsx: render layer asserts the Alert
 * fires with the i18n KEYS (global setup mocks `t(key) => key`) via an
 * `Alert.alert` spy; resource layer asserts the locale JSON carries the keys.
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

    const refresh = await screen.findByText('🔄 Refresh All Widgets');
    fireEvent.press(refresh);

    await waitFor(() => {
      expect(alertSpy).toHaveBeenCalledWith('widgets.updatedTitle', 'widgets.updatedMessage');
    });
    alertSpy.mockRestore();
  });
});

describe('widgets Alert keys resolve to real, locale-defined translations', () => {
  const keys = ['addedTitle', 'addedMessage', 'updatedTitle', 'updatedMessage'] as const;

  it('en carries the expected English copy and addedMessage interpolates the name', () => {
    expect(en.widgets.addedTitle).toBe('Widget Added');
    expect(en.widgets.addedMessage).toContain('{{name}}');
    expect(en.widgets.updatedTitle).toBe('Widgets Updated');
    expect(en.widgets.updatedMessage).toBe('All widgets have been refreshed.');
  });

  it('every locale defines the widget Alert keys and keeps the {{name}} placeholder', () => {
    for (const locale of [en, sk, cs, de, pl, hu]) {
      for (const key of keys) {
        const value = (locale.widgets as Record<string, string>)[key];
        expect(typeof value).toBe('string');
        expect(value.length).toBeGreaterThan(0);
      }
      expect(locale.widgets.addedMessage).toContain('{{name}}');
    }
  });
});
