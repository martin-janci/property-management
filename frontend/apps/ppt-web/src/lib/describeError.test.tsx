/// <reference types="vitest/globals" />
/**
 * Translated error copy for toasts (round-3 audit #34). Raw English server
 * text ("Only managers can create articles", "HTTP 422", "Request failed")
 * used to be the toast message; it is now chosen by HTTP status and the
 * server text survives only as a detail line.
 */
import { render, screen } from '@testing-library/react';
import i18n from 'i18next';
import { act } from 'react';
import sk from '../../messages/sk.json';
import { ToastProvider, useToast } from '../components/Toast';
import { getErrorMessage } from './api';
import { describeError, errorStatusCode } from './describeError';

class StatusError extends Error {
  constructor(
    readonly status: number,
    message: string
  ) {
    super(message);
  }
}

beforeAll(() => {
  i18n.addResourceBundle('sk', 'translation', sk, true, true);
});

beforeEach(async () => {
  await i18n.changeLanguage('sk');
});

afterAll(async () => {
  await i18n.changeLanguage('en');
});

describe('errorStatusCode', () => {
  it('reads status fields and placeholder messages', () => {
    expect(errorStatusCode(new StatusError(403, 'x'))).toBe(403);
    expect(errorStatusCode({ response: { status: 404 } })).toBe(404);
    expect(errorStatusCode(new Error('HTTP 422'))).toBe(422);
    expect(errorStatusCode(new Error('HTTP error 422'))).toBe(422);
    expect(errorStatusCode(new Error('Request failed with status code 500'))).toBe(500);
    expect(errorStatusCode(new Error('Only managers can create articles'))).toBeUndefined();
  });
});

describe('describeError', () => {
  it('maps 403 to the translated access-denied copy and keeps server text as detail', () => {
    const d = describeError(new StatusError(403, 'Only managers can create articles'));
    expect(d.message).toBe(sk.errors.catalogue.UNAUTHORIZED.message);
    expect(d.detail).toBe('Only managers can create articles');
    expect(d.statusCode).toBe(403);
  });

  it('maps placeholder "HTTP 422" text and drops it as detail', () => {
    const d = describeError(new Error('HTTP error 422'));
    expect(d.message).toBe(sk.errors.catalogue.VALIDATION_ERROR.message);
    expect(d.detail).toBeUndefined();
  });

  it('maps 5xx to the server error copy', () => {
    const d = describeError(new StatusError(500, 'Failed to create sensor'));
    expect(d.message).toBe(sk.errors.catalogue.INTERNAL_ERROR.message);
    expect(d.detail).toBe('Failed to create sensor');
  });

  it('maps network failures', () => {
    expect(describeError(new TypeError('Failed to fetch')).message).toBe(
      sk.errors.catalogue.NETWORK_ERROR.message
    );
  });

  it('uses the generic copy for an unknown error without a status', () => {
    const d = describeError(new Error('Article not found'));
    expect(d.message).toBe(sk.errors.catalogue.default.message);
    expect(d.detail).toBe('Article not found');
  });

  it('getErrorMessage returns only the translated message', () => {
    expect(getErrorMessage(new Error('Request failed'))).toBe(sk.errors.catalogue.default.message);
  });
});

describe('error toasts', () => {
  function Trigger({ error }: { error: unknown }) {
    const { showToast } = useToast();
    return (
      <button
        type="button"
        onClick={() => showToast({ type: 'error', title: 'Uloženie zlyhalo', error })}
      >
        go
      </button>
    );
  }

  it('show the translated message with the server text as detail', () => {
    render(
      <ToastProvider>
        <Trigger error={new StatusError(403, 'Manager role required for this action')} />
      </ToastProvider>
    );
    act(() => {
      screen.getByRole('button', { name: 'go' }).click();
    });
    expect(screen.getByText(sk.errors.catalogue.UNAUTHORIZED.message)).toBeTruthy();
    const detail = screen.getByText('Manager role required for this action');
    expect(detail.className).toBe('toast-detail');
  });
});

describe('describeError — production build (no raw server text leak)', () => {
  // Security regression (code-review: ppt-web core error handler raw backend
  // message passthrough). The raw backend `error.message` is a DEV-only
  // diagnostic: it ships English regardless of locale and can carry internal
  // server/DB text, so in a production build it must NOT survive as the toast
  // `detail` line. This mirrors `devDetail` in ./errorHandler, which already
  // gates the raw text behind `import.meta.env.DEV`. The user still sees the
  // localized, status-derived `message`.
  //
  // These fail on the pre-fix `describeError` (which returned the raw text as
  // `detail` in every build) and pass once the DEV gate is added.
  function Trigger({ error }: { error: unknown }) {
    const { showToast } = useToast();
    return (
      <button
        type="button"
        onClick={() => showToast({ type: 'error', title: 'Uloženie zlyhalo', error })}
      >
        go
      </button>
    );
  }

  beforeEach(() => {
    vi.stubEnv('DEV', false);
  });

  afterEach(() => {
    vi.unstubAllEnvs();
  });

  it('drops the raw server text as detail in production', () => {
    const d = describeError(new StatusError(403, 'Only managers can create articles'));
    expect(d.message).toBe(sk.errors.catalogue.UNAUTHORIZED.message);
    expect(d.detail).toBeUndefined();
  });

  it('drops internal 5xx server/DB text as detail in production', () => {
    const d = describeError(new StatusError(500, 'row 42 violates unique constraint foo_key'));
    expect(d.message).toBe(sk.errors.catalogue.INTERNAL_ERROR.message);
    expect(d.detail).toBeUndefined();
  });

  it('does not render the raw server text in the toast in production', () => {
    render(
      <ToastProvider>
        <Trigger error={new StatusError(403, 'Manager role required for this action')} />
      </ToastProvider>
    );
    act(() => {
      screen.getByRole('button', { name: 'go' }).click();
    });
    // The localized, status-derived copy is still shown to the user...
    expect(screen.getByText(sk.errors.catalogue.UNAUTHORIZED.message)).toBeTruthy();
    // ...but the raw backend message never reaches the DOM.
    expect(screen.queryByText('Manager role required for this action')).toBeNull();
    expect(document.querySelector('.toast-detail')).toBeNull();
  });
});
