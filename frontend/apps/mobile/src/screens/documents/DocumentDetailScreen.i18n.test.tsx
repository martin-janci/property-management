/**
 * DocumentDetailScreen — open-failure Alert i18n regression test
 * (code-review-mobile-rn-alert-hardcoded-en).
 *
 * The open action's catch block raised a hardcoded English Alert —
 * `Alert.alert('Could not open document', 'The file viewer is unavailable on
 * this device.')` — instead of i18n keys. This suite locks the contract: the
 * Alert is now sourced from `documents.openErrorTitle/openErrorMessage` via
 * `t()`, and those keys exist in every locale JSON.
 *
 * Render layer asserts the Alert fires with the i18n KEYS (global setup mocks
 * `t(key) => key`) via an `Alert.alert` spy; resource layer asserts the locale
 * JSON carries the keys.
 */
import { fireEvent, render, screen, waitFor } from '@testing-library/react-native';
import { Alert } from 'react-native';
import cs from '../../locales/cs.json';
import de from '../../locales/de.json';
import en from '../../locales/en.json';
import hu from '../../locales/hu.json';
import pl from '../../locales/pl.json';
import sk from '../../locales/sk.json';
import { DocumentDetailScreen } from './DocumentDetailScreen';

const mockApiRequest = jest.fn();

jest.mock('../../hooks/useApi', () => ({
  // Return a document so the "Open document" button renders.
  useApiQuery: () => ({
    data: {
      document: {
        id: 'doc-1',
        title: 'Lease.pdf',
        category: 'lease',
        file_name: 'lease.pdf',
        mime_type: 'application/pdf',
        size_bytes: 1024,
        created_at: '2026-01-01T00:00:00Z',
        updated_at: '2026-01-02T00:00:00Z',
      },
    },
    isLoading: false,
    error: null,
    refetch: jest.fn(),
  }),
  apiRequest: (...args: unknown[]) => mockApiRequest(...args),
}));

// Keep the share sheet out of the render tree — it has its own deps.
jest.mock('./DocumentShareSheet', () => ({ DocumentShareSheet: () => null }));

describe('DocumentDetailScreen open-failure Alert i18n', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    // Hard failure (not a previewable-type fallback) → the catch raises the Alert.
    mockApiRequest.mockRejectedValue(new Error('Network request failed'));
  });

  it('raises the open-failure Alert via i18n keys, not hardcoded literals', async () => {
    const alertSpy = jest.spyOn(Alert, 'alert').mockImplementation(() => {});
    await render(<DocumentDetailScreen documentId="doc-1" onBack={jest.fn()} />);

    fireEvent.press(await screen.findByText('Open document'));

    await waitFor(() => {
      expect(alertSpy).toHaveBeenCalledWith(
        'documents.openErrorTitle',
        'documents.openErrorMessage'
      );
    });
    alertSpy.mockRestore();
  });
});

describe('documents open-error Alert keys resolve to real, locale-defined translations', () => {
  it('en carries the expected English copy', () => {
    expect(en.documents.openErrorTitle).toBe('Could not open document');
    expect(en.documents.openErrorMessage).toBe('The file viewer is unavailable on this device.');
  });

  it('every locale defines the open-error keys and translates them away from English', () => {
    for (const locale of [sk, cs, de, pl, hu]) {
      for (const key of ['openErrorTitle', 'openErrorMessage'] as const) {
        const value = (locale.documents as unknown as Record<string, string>)[key];
        expect(typeof value).toBe('string');
        expect(value.length).toBeGreaterThan(0);
      }
      expect(locale.documents.openErrorTitle).not.toBe(en.documents.openErrorTitle);
      expect(locale.documents.openErrorMessage).not.toBe(en.documents.openErrorMessage);
    }
  });
});
