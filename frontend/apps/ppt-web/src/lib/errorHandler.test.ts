/// <reference types="vitest/globals" />
/**
 * Unit coverage for the ppt-web API error handler (Story 79.3 — Error Handling
 * and Toast Notifications).
 *
 * `lib/errorHandler.ts` owns the translation from raw transport/backend errors
 * into the `ParsedApiError` shape the UI (toasts + inline form feedback) relies
 * on. It was shipped and exported via the `lib` barrel but had no direct unit
 * coverage. These tests exercise every public export and each branch of the
 * parsing logic:
 *
 *   - `parseApiError` — network errors, axios errors with/without a structured
 *     body, rate-limit (429 + Retry-After parsing), bare HTTP status codes,
 *     plain `ErrorResponse` objects, `Error` instances, and unknown fallbacks.
 *   - `formatValidationErrors` — Map -> multi-line human string.
 *   - `getFieldError` — field lookup incl. the undefined-map guard.
 *
 * Additive-only: no production code is touched.
 */

import i18n from 'i18next';
import sk from '../../messages/sk.json';
import {
  type ErrorResponse,
  formatValidationErrors,
  getFieldError,
  parseApiError,
} from './errorHandler';

describe('parseApiError — network errors', () => {
  it('detects axios ERR_NETWORK code', () => {
    const result = parseApiError({ code: 'ERR_NETWORK', message: 'boom' });

    expect(result.code).toBe('NETWORK_ERROR');
    expect(result.isNetworkError).toBe(true);
    expect(result.isRateLimitError).toBe(false);
    expect(result.title).toBe('Network Error');
  });

  it('detects network keyword in the message', () => {
    const result = parseApiError({ message: 'Network request failed' });
    expect(result.code).toBe('NETWORK_ERROR');
    expect(result.isNetworkError).toBe(true);
  });

  it('detects "failed to fetch" wording', () => {
    const result = parseApiError({ message: 'Failed to fetch resource' });
    expect(result.code).toBe('NETWORK_ERROR');
    expect(result.isNetworkError).toBe(true);
  });

  it('treats a fetch TypeError as a network error', () => {
    const result = parseApiError(new TypeError('Failed to fetch'));
    expect(result.code).toBe('NETWORK_ERROR');
    expect(result.isNetworkError).toBe(true);
  });
});

describe('parseApiError — axios errors with structured body', () => {
  it('maps a known backend error code to its friendly title', () => {
    const error = {
      response: {
        status: 401,
        data: {
          requestId: 'req-123',
          error: 'SESSION_EXPIRED',
          message: 'Session gone',
        } satisfies ErrorResponse,
      },
    };

    const result = parseApiError(error);

    expect(result.code).toBe('SESSION_EXPIRED');
    expect(result.title).toBe('Session Expired');
    // Backend message takes precedence over the canned message.
    expect(result.message).toBe('Session gone');
    expect(result.requestId).toBe('req-123');
    expect(result.statusCode).toBe(401);
    expect(result.isNetworkError).toBe(false);
  });

  it('falls back to the canned message when body message is empty', () => {
    const error = {
      response: {
        status: 404,
        data: { requestId: 'r', error: 'RESOURCE_NOT_FOUND', message: '' },
      },
    };

    const result = parseApiError(error);
    expect(result.code).toBe('RESOURCE_NOT_FOUND');
    expect(result.message).toBe('The item you are looking for does not exist or has been deleted.');
  });

  it('uses the default title/message for an unknown backend code', () => {
    const error = {
      response: {
        status: 418,
        data: { requestId: 'r', error: 'TEAPOT', message: 'I am a teapot' },
      },
    };

    const result = parseApiError(error);
    expect(result.code).toBe('TEAPOT');
    expect(result.title).toBe('Error');
    expect(result.message).toBe('I am a teapot');
  });

  it('extracts validation errors into a field-keyed Map', () => {
    const error = {
      response: {
        status: 400,
        data: {
          requestId: 'r',
          error: 'VALIDATION_ERROR',
          message: 'Bad form',
          details: [
            { field: 'user.email', message: 'Invalid email' },
            { field: 'address.street_name', message: 'Required' },
          ],
        } satisfies ErrorResponse,
      },
    };

    const result = parseApiError(error);
    expect(result.validationErrors).toBeInstanceOf(Map);
    expect(result.validationErrors?.get('user.email')).toBe('Invalid email');
    expect(result.validationErrors?.get('address.street_name')).toBe('Required');
  });

  it('leaves validationErrors undefined when details are empty', () => {
    const error = {
      response: {
        status: 400,
        data: { requestId: 'r', error: 'VALIDATION_ERROR', message: 'x', details: [] },
      },
    };
    const result = parseApiError(error);
    expect(result.validationErrors).toBeUndefined();
  });
});

describe('parseApiError — rate limiting', () => {
  it('parses numeric Retry-After seconds', () => {
    const error = {
      response: {
        status: 429,
        data: { requestId: 'r', error: 'RATE_LIMITED', message: 'slow down' },
        headers: { 'retry-after': '30' },
      },
    };

    const result = parseApiError(error);
    expect(result.code).toBe('RATE_LIMITED');
    expect(result.isRateLimitError).toBe(true);
    expect(result.retryAfterSeconds).toBe(30);
    expect(result.message).toContain('30 seconds');
  });

  it('parses Retry-After given as an HTTP date', () => {
    const future = new Date(Date.now() + 60_000).toUTCString();
    const error = {
      response: {
        status: 429,
        data: { requestId: 'r', error: 'RATE_LIMITED', message: 'slow' },
        headers: { 'retry-after': future },
      },
    };

    const result = parseApiError(error);
    expect(result.isRateLimitError).toBe(true);
    // ~60 seconds in the future (allow a little clock slack).
    expect(result.retryAfterSeconds).toBeGreaterThanOrEqual(55);
    expect(result.retryAfterSeconds).toBeLessThanOrEqual(61);
  });

  it('falls back to a generic rate-limit message without Retry-After', () => {
    const error = {
      response: {
        status: 429,
        data: { requestId: 'r', error: 'RATE_LIMITED', message: '' },
      },
    };

    const result = parseApiError(error);
    expect(result.isRateLimitError).toBe(true);
    expect(result.retryAfterSeconds).toBeUndefined();
    expect(result.message).toBe(
      'You have made too many requests. Please wait before trying again.'
    );
  });
});

describe('parseApiError — bare HTTP status (no structured body)', () => {
  const cases: Array<[number, string]> = [
    [400, 'INVALID_INPUT'],
    [401, 'AUTHENTICATION_ERROR'],
    [403, 'UNAUTHORIZED'],
    [404, 'NOT_FOUND'],
    [409, 'CONFLICT'],
    [500, 'INTERNAL_ERROR'],
    [502, 'INTERNAL_ERROR'],
    [503, 'INTERNAL_ERROR'],
    [504, 'TIMEOUT'],
  ];

  it.each(cases)('maps status %i to code %s', (status, code) => {
    const result = parseApiError({ response: { status, data: undefined } });
    expect(result.code).toBe(code);
    expect(result.statusCode).toBe(status);
  });

  it('maps 403 as an authorization error, not rate limited', () => {
    const result = parseApiError({ response: { status: 403, data: undefined } });
    expect(result.isRateLimitError).toBe(false);
  });

  it('flags a bare 429 as rate limited', () => {
    const result = parseApiError({ response: { status: 429, data: undefined } });
    expect(result.code).toBe('RATE_LIMITED');
    expect(result.isRateLimitError).toBe(true);
  });

  it('falls back to UNKNOWN_ERROR for an unmapped status', () => {
    const result = parseApiError({ response: { status: 418, data: undefined } });
    expect(result.code).toBe('UNKNOWN_ERROR');
    expect(result.statusCode).toBe(418);
  });
});

describe('parseApiError — plain objects, Error, and fallback', () => {
  it('parses a plain ErrorResponse object (no axios envelope)', () => {
    const result = parseApiError({
      requestId: 'req-9',
      error: 'DUPLICATE_ENTRY',
      message: 'Already exists',
    });

    expect(result.code).toBe('DUPLICATE_ENTRY');
    expect(result.title).toBe('Duplicate Entry');
    expect(result.message).toBe('Already exists');
    expect(result.requestId).toBe('req-9');
  });

  it('uses the Error message for a raw Error instance', () => {
    const result = parseApiError(new Error('something specific broke'));
    expect(result.code).toBe('UNKNOWN_ERROR');
    expect(result.message).toBe('something specific broke');
    expect(result.isNetworkError).toBe(false);
  });

  it('falls back to the default message for an Error with no message', () => {
    const result = parseApiError(new Error(''));
    expect(result.code).toBe('UNKNOWN_ERROR');
    expect(result.message).toBe('An unexpected error occurred.');
  });

  it('falls back cleanly for a non-object primitive', () => {
    const result = parseApiError('just a string');
    expect(result.code).toBe('UNKNOWN_ERROR');
    expect(result.title).toBe('Error');
    expect(result.isNetworkError).toBe(false);
    expect(result.isRateLimitError).toBe(false);
  });

  it('falls back cleanly for null', () => {
    const result = parseApiError(null);
    expect(result.code).toBe('UNKNOWN_ERROR');
  });
});

describe('formatValidationErrors', () => {
  it('renders each field on its own line with a Title-Cased label', () => {
    const errors = new Map<string, string>([
      ['user.email', 'Invalid email'],
      ['address.street_name', 'Required'],
    ]);

    const formatted = formatValidationErrors(errors);
    expect(formatted).toBe('Email: Invalid email\nStreet Name: Required');
  });

  it('returns an empty string for an empty map', () => {
    expect(formatValidationErrors(new Map())).toBe('');
  });

  it('uses only the last path segment for the label', () => {
    const errors = new Map<string, string>([['deeply.nested.field_name', 'bad']]);
    expect(formatValidationErrors(errors)).toBe('Field Name: bad');
  });
});

describe('getFieldError', () => {
  const errors = new Map<string, string>([['email', 'Invalid email']]);

  it('returns the message for a known field', () => {
    expect(getFieldError(errors, 'email')).toBe('Invalid email');
  });

  it('returns undefined for an unknown field', () => {
    expect(getFieldError(errors, 'password')).toBeUndefined();
  });

  it('returns undefined when the map itself is undefined', () => {
    expect(getFieldError(undefined, 'email')).toBeUndefined();
  });
});

describe('parseApiError — i18n localisation', () => {
  // Regression for the hardcoded-English error catalogue: user-facing titles
  // and messages must follow the active react-i18next language rather than
  // always emitting English. This fails on `dev` (where the catalogue was a
  // literal English `Record`) and passes once parsing routes through
  // `errors.catalogue.*`.
  beforeAll(() => {
    // Ensure the Slovak bundle is available on the shared i18next instance the
    // error handler reads from, regardless of test import order.
    i18n.addResourceBundle('sk', 'translation', sk, true, true);
  });

  afterEach(() => {
    // Restore English so the other suites (and other test files sharing the
    // singleton) are unaffected.
    i18n.changeLanguage('en');
  });

  it('translates a known error title/message into the active locale', () => {
    i18n.changeLanguage('sk');

    const result = parseApiError({
      requestId: 'req-sk',
      error: 'SESSION_EXPIRED',
      message: '',
    });

    expect(result.code).toBe('SESSION_EXPIRED');
    expect(result.title).toBe('Relácia vypršala');
    expect(result.message).toBe('Vaša relácia vypršala. Prihláste sa prosím znova.');
  });

  it('translates the default fallback copy for unknown codes', () => {
    i18n.changeLanguage('sk');

    const result = parseApiError({ response: { status: 418, data: undefined } });

    expect(result.code).toBe('UNKNOWN_ERROR');
    expect(result.title).toBe('Chyba');
  });

  it('localises the interpolated rate-limit retry message', () => {
    i18n.changeLanguage('sk');

    const result = parseApiError({
      response: {
        status: 429,
        data: { requestId: 'r', error: 'RATE_LIMITED', message: '' },
        headers: { 'retry-after': '30' },
      },
    });

    expect(result.isRateLimitError).toBe(true);
    expect(result.message).toBe('Pred ďalším pokusom prosím počkajte 30 sekúnd.');
  });

  it('still emits English copy when the active locale is English', () => {
    i18n.changeLanguage('en');

    const result = parseApiError({ requestId: 'r', error: 'SESSION_EXPIRED', message: '' });

    expect(result.title).toBe('Session Expired');
  });
});
