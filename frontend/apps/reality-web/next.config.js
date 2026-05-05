const createNextIntlPlugin = require('next-intl/plugin');

const withNextIntl = createNextIntlPlugin('./src/i18n/request.ts');

const isDev = process.env.NODE_ENV !== 'production';

// Allow the configured API origin (plus production defaults and — in dev —
// localhost / emulator host) in connect-src so the browser can actually reach
// the reality-server.
const apiOrigin = process.env.NEXT_PUBLIC_API_URL;
const connectSrcOrigins = new Set([
  "'self'",
  'https://api.reality-portal.sk',
  'https://api.reality-portal.cz',
  'https://api.reality-portal.eu',
]);
if (apiOrigin) {
  try {
    connectSrcOrigins.add(new URL(apiOrigin).origin);
  } catch {
    // Ignore malformed values; default production domains already in the set.
  }
}
if (isDev) {
  connectSrcOrigins.add('http://localhost:8081');
  connectSrcOrigins.add('http://127.0.0.1:8081');
  connectSrcOrigins.add('http://10.0.2.2:8081');
  connectSrcOrigins.add('ws://localhost:*');
  connectSrcOrigins.add('ws://127.0.0.1:*');
}

// Security headers applied to every route.
//
// `script-src 'unsafe-inline'`: Next.js 14 injects a small inline
// bootstrapper (NEXT_DATA, React hydration stubs) that is not nonced by
// default in the App Router. Removing `'unsafe-inline'` breaks hydration.
// Follow-up is to introduce a middleware-generated per-request nonce and
// attach it via `<Script nonce={...}>` for every first-party script, then
// drop `'unsafe-inline'`. Tracked as a deferred item on PR #176.
//
// `style-src 'unsafe-inline'`: Next.js CSS-in-JS + next-intl inject inline
// `<style>` blocks; same nonce migration will cover them.
//
// `'unsafe-eval'` is intentionally NOT allowed — dev hot-reload may need
// it for some React Native Fast Refresh flows, but next-dev does not.
const securityHeaders = [
  {
    key: 'Content-Security-Policy',
    value: [
      "default-src 'self'",
      "script-src 'self' 'unsafe-inline'",
      "style-src 'self' 'unsafe-inline'",
      "img-src 'self' data: blob: https:",
      "font-src 'self' data:",
      `connect-src ${Array.from(connectSrcOrigins).join(' ')}`,
      "frame-ancestors 'none'",
      "base-uri 'self'",
      "form-action 'self'",
      "object-src 'none'",
    ].join('; '),
  },
  { key: 'Referrer-Policy', value: 'strict-origin-when-cross-origin' },
  { key: 'X-Content-Type-Options', value: 'nosniff' },
  { key: 'X-Frame-Options', value: 'DENY' },
  {
    key: 'Permissions-Policy',
    value: 'camera=(), microphone=(), geolocation=(self), interest-cohort=()',
  },
  { key: 'Strict-Transport-Security', value: 'max-age=63072000; includeSubDomains; preload' },
];

/** @type {import('next').NextConfig} */
const nextConfig = {
  // Standalone output for Docker deployment
  output: 'standalone',

  // Enable React strict mode
  reactStrictMode: true,

  // Image optimization
  images: {
    domains: ['api.reality-portal.sk', 'api.reality-portal.cz', 'api.reality-portal.eu'],
  },

  // Environment variables
  env: {
    REGION: process.env.REGION || 'local',
  },

  async headers() {
    return [
      {
        source: '/:path*',
        headers: securityHeaders,
      },
    ];
  },
};

module.exports = withNextIntl(nextConfig);
