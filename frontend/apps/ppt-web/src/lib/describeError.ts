/**
 * Translated error copy for toasts and inline errors (round-3 audit #34).
 *
 * The hand-written api clients throw `Error`s whose message is the server's
 * English text ("Only managers can create articles", "Failed to create
 * sensor") or a placeholder ("HTTP 422", "HTTP error 422", "Request failed").
 * Showing that text as the toast message leaked English into every locale.
 * `describeError` picks the translated copy from the HTTP status instead
 * (reusing the `errors.catalogue.*` bundle) and keeps the server text only as
 * a secondary detail.
 *
 * Deliberately free of `react-i18next` / `../i18n` imports so it can be used
 * from components that tests render with a mocked `react-i18next`.
 */
import i18next, { type TFunction } from 'i18next';

/** A translated, user-facing description of a failed request. */
export interface DescribedError {
  /** Translated message chosen from the HTTP status (or network failure). */
  message: string;
  /** The raw server/client text, kept only as secondary detail. */
  detail?: string;
  statusCode?: number;
}

type Translate = TFunction | ((key: string) => string);

const STATUS_ERROR_CODES: Record<number, string> = {
  400: 'INVALID_INPUT',
  401: 'AUTHENTICATION_ERROR',
  403: 'UNAUTHORIZED',
  404: 'NOT_FOUND',
  409: 'CONFLICT',
  422: 'VALIDATION_ERROR',
  429: 'RATE_LIMITED',
  503: 'SERVICE_UNAVAILABLE',
  504: 'TIMEOUT',
};

/** Placeholder messages the hand-written clients throw instead of real text. */
const PLACEHOLDER_MESSAGE =
  /^\s*(?:HTTP(?:\s+error)?\s+(\d{3})|Request failed(?: with status (?:code )?(\d{3}))?|Unknown error)\s*$/i;

function rawMessage(error: unknown): string {
  if (typeof error === 'string') return error;
  if (error && typeof error === 'object') {
    const message = (error as { message?: unknown }).message;
    if (typeof message === 'string') return message;
  }
  return '';
}

/**
 * Read an HTTP status off any error shape the clients throw: `status` /
 * `statusCode` fields (api-client `ApiError`, ppt-web `ApiError`), an axios
 * `response.status`, or a placeholder message such as "HTTP 422".
 */
export function errorStatusCode(error: unknown): number | undefined {
  if (error && typeof error === 'object') {
    const e = error as { status?: unknown; statusCode?: unknown; response?: { status?: unknown } };
    for (const candidate of [e.status, e.statusCode, e.response?.status]) {
      if (typeof candidate === 'number' && candidate >= 100 && candidate < 600) {
        return candidate;
      }
    }
  }
  const match = PLACEHOLDER_MESSAGE.exec(rawMessage(error));
  const code = match?.[1] ?? match?.[2];
  return code ? Number(code) : undefined;
}

function isNetworkFailure(error: unknown): boolean {
  if (error && typeof error === 'object' && (error as { code?: unknown }).code === 'ERR_NETWORK') {
    return true;
  }
  const message = rawMessage(error).toLowerCase();
  return (
    message.includes('failed to fetch') ||
    message.includes('networkerror') ||
    message.includes('network request failed') ||
    message.includes('network connection lost')
  );
}

/**
 * Turn any thrown value into translated copy: 403 → access denied, 404 → not
 * found, 422 → validation error, 5xx → server error, network failure →
 * network error, anything else → the generic error. The raw text is returned
 * as `detail` unless it is only a placeholder like "HTTP 422".
 */
export function describeError(
  error: unknown,
  t: Translate = i18next.t.bind(i18next) as Translate
): DescribedError {
  const raw = rawMessage(error).trim();
  const statusCode = errorStatusCode(error);
  let code = 'default';
  if (statusCode !== undefined) {
    code = STATUS_ERROR_CODES[statusCode] ?? (statusCode >= 500 ? 'INTERNAL_ERROR' : 'default');
  } else if (isNetworkFailure(error)) {
    code = 'NETWORK_ERROR';
  }
  const message = (t as (key: string) => string)(`errors.catalogue.${code}.message`);
  const isPlaceholder = raw === '' || PLACEHOLDER_MESSAGE.test(raw);
  const detail = isPlaceholder || raw === message ? undefined : raw;
  return { message, detail, statusCode };
}
