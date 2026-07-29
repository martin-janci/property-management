/**
 * next-config-headers.js
 *
 * Extracted header-building logic from next.config.js so it can be unit-tested
 * without importing Next.js plugins or workspace packages.
 *
 * Called by next.config.js; exported for tests.
 */

/**
 * @typedef {{ key: string; value: string }} HeaderEntry
 */

/**
 * @typedef {{ source: string; has?: Array<{ type: string; key: string; value: string }>; headers: HeaderEntry[] }} RouteHeaderEntry
 */

/**
 * @typedef {{ isDev: boolean; connectSrcOrigins: Set<string>; layoutPreviewOrigins: string }} BuildHeaderEntriesOptions
 */

/**
 * Build the Next.js `headers()` return value.
 *
 * - Blanket entry (first): all routes, `frame-ancestors 'none'` + `X-Frame-Options: DENY`.
 * - Carve-out entry (after blanket, only when `layoutPreviewOrigins` is non-empty):
 *   matches `?layoutPreview=1` requests on listing-detail routes ONLY
 *   (`/:locale/listings/:slug` — the single screen the layout preview frames)
 *   and overwrites CSP frame-ancestors with the configured origins. X-Frame-Options is omitted (CSP supersedes it in modern browsers;
 *   XFO cannot express allowlists). Next.js applies all matching entries cumulatively with
 *   last-wins for duplicate header keys, so placing the carve-out entry AFTER the blanket
 *   causes its CSP value to win.
 *
 * @param {BuildHeaderEntriesOptions} options
 * @returns {RouteHeaderEntry[]}
 */
function buildHeaderEntries({ isDev, connectSrcOrigins, layoutPreviewOrigins }) {
  const scriptSrc = ["'self'", "'unsafe-inline'"];
  if (isDev) {
    scriptSrc.push("'unsafe-eval'");
  }

  const connectSrcValue = Array.from(connectSrcOrigins).join(' ');

  const blanketCsp = [
    "default-src 'self'",
    `script-src ${scriptSrc.join(' ')}`,
    "style-src 'self' 'unsafe-inline'",
    "img-src 'self' data: blob: https:",
    "font-src 'self' data:",
    `connect-src ${connectSrcValue}`,
    "frame-ancestors 'none'",
    "base-uri 'self'",
    "form-action 'self'",
    "object-src 'none'",
  ].join('; ');

  /** @type {HeaderEntry[]} */
  const blanketHeaders = [
    { key: 'Content-Security-Policy', value: blanketCsp },
    { key: 'Referrer-Policy', value: 'strict-origin-when-cross-origin' },
    { key: 'X-Content-Type-Options', value: 'nosniff' },
    { key: 'X-Frame-Options', value: 'DENY' },
    {
      key: 'Permissions-Policy',
      value: 'camera=(), microphone=(), geolocation=(self), interest-cohort=()',
    },
    { key: 'Strict-Transport-Security', value: 'max-age=63072000; includeSubDomains; preload' },
  ];

  /** @type {RouteHeaderEntry[]} */
  const entries = [
    {
      source: '/:path*',
      headers: blanketHeaders,
    },
  ];

  if (layoutPreviewOrigins) {
    const previewCsp = [
      "default-src 'self'",
      `script-src ${scriptSrc.join(' ')}`,
      "style-src 'self' 'unsafe-inline'",
      "img-src 'self' data: blob: https:",
      "font-src 'self' data:",
      `connect-src ${connectSrcValue}`,
      `frame-ancestors ${layoutPreviewOrigins}`,
      "base-uri 'self'",
      "form-action 'self'",
      "object-src 'none'",
    ].join('; ');

    entries.push({
      // Narrowed to listing-detail routes only — allowed origins must not be
      // able to frame arbitrary pages, just the screen the preview targets.
      source: '/:locale/listings/:slug',
      has: [{ type: 'query', key: 'layoutPreview', value: '1' }],
      headers: [
        { key: 'Content-Security-Policy', value: previewCsp },
        { key: 'Referrer-Policy', value: 'strict-origin-when-cross-origin' },
        { key: 'X-Content-Type-Options', value: 'nosniff' },
        // X-Frame-Options intentionally omitted — cannot express allowlists;
        // CSP frame-ancestors supersedes it in all modern browsers.
        {
          key: 'Permissions-Policy',
          value: 'camera=(), microphone=(), geolocation=(self), interest-cohort=()',
        },
        {
          key: 'Strict-Transport-Security',
          value: 'max-age=63072000; includeSubDomains; preload',
        },
      ],
    });
  }

  return entries;
}

module.exports = { buildHeaderEntries };
