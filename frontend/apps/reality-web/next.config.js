const createNextIntlPlugin = require('next-intl/plugin');
const { detectWorktree } = require('@ppt/vite-plugin-worktree/detect');

const withNextIntl = createNextIntlPlugin('./src/i18n/request.ts');

const worktree = detectWorktree();

const isDev = process.env.NODE_ENV !== 'production';

// Allow the configured API origin (plus production defaults and — in dev —
// localhost / emulator host) in connect-src so the browser can actually reach
// the reality-server.
const apiOrigin = process.env.NEXT_PUBLIC_API_URL;
// Hardcoded reality-server origins. Includes the real prod/staging hosts
// (api.rlt.sk family) and the legacy multi-region hosts (api.reality-portal.*).
// Worktree dev URLs at *.dev.rlt.sk fall back to api.rlt.sk via host inference
// in lib/env.ts and app/env.js/route.ts when NEXT_PUBLIC_API_URL isn't set;
// this list ensures CSP `connect-src` permits those connections even when the
// dynamic apiOrigin block below adds nothing (process.env unset).
const connectSrcOrigins = new Set([
  "'self'",
  'https://api.rlt.sk',
  'https://api.staging.rlt.sk',
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
// `'unsafe-eval'` (dev only): Next.js 14 Fast Refresh runtime
// (`@next/react-refresh-utils/runtime`) evaluates hot-updated module code via
// `eval()`. Without it, the dev bundle throws EvalError on first paint, the
// client never hydrates, and styled-jsx never injects component CSS — the
// page is left with only globals.css and looks completely unstyled. Production
// builds don't ship react-refresh, so the prod CSP is unaffected.
const scriptSrc = ["'self'", "'unsafe-inline'"];
if (isDev) {
  scriptSrc.push("'unsafe-eval'");
}
const securityHeaders = [
  {
    key: 'Content-Security-Policy',
    value: [
      "default-src 'self'",
      `script-src ${scriptSrc.join(' ')}`,
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

  // Don't advertise the framework via `X-Powered-By: Next.js` (#957). No
  // version is leaked, but the Rust API hosts omit any banner — match them.
  poweredByHeader: false,

  // Enable React strict mode
  reactStrictMode: true,

  // Workspace packages whose `main` points at TypeScript source rather than
  // built JS. Next won't compile TS from `node_modules` by default; listing
  // them here lets the Next compiler pick them up. Add new TS-only workspace
  // deps here (or build them to `dist/*` and switch their `main`/`exports`).
  transpilePackages: ['@ppt/dev-panel'],

  // Image optimization.
  // `domains` is deprecated in Next 14+ in favour of `remotePatterns`, which
  // also accepts wildcards. Added unsplash for mock journal article photos
  // and `*.rlt.sk` to cover both prod and worktree-deploy hosts; the API
  // family is enumerated for prod CDN-served listing photos.
  images: {
    remotePatterns: [
      { protocol: 'https', hostname: 'api.rlt.sk' },
      { protocol: 'https', hostname: 'api.staging.rlt.sk' },
      { protocol: 'https', hostname: 'api.reality-portal.sk' },
      { protocol: 'https', hostname: 'api.reality-portal.cz' },
      { protocol: 'https', hostname: 'api.reality-portal.eu' },
      { protocol: 'https', hostname: 'images.unsplash.com' },
      { protocol: 'https', hostname: '**.rlt.sk' },
    ],
  },

  // Environment variables
  env: {
    REGION: process.env.REGION || 'local',
    NEXT_PUBLIC_PPT_WORKTREE_NAME: worktree.name,
    NEXT_PUBLIC_PPT_WORKTREE_BRANCH: worktree.branch,
    NEXT_PUBLIC_PPT_IS_WORKTREE: String(worktree.isWorktree),
  },

  async headers() {
    return [
      {
        source: '/:path*',
        headers: securityHeaders,
      },
    ];
  },

  // Rewrite /api/* to the prod reality-server when running on a *.dev.rlt.sk
  // worktree host. Shared-backend mode means the worktree's reality-web is
  // backed by api.rlt.sk, but the prod CORS allowlist doesn't include the
  // worktree subdomain, so direct cross-origin fetch fails. The dev server
  // proxies the call, turning it into a same-origin request from the
  // browser's perspective — CORS is bypassed.
  //
  // Gated on the request `host` header so localhost dev keeps talking to
  // localhost:8081 directly, prod (rlt.sk) goes straight to api.rlt.sk
  // (which is in its CORS allowlist), and only worktree subdomains take
  // the rewrite path.
  //
  // Caveat: this proxies authenticated calls but does NOT rewrite the
  // Set-Cookie domain. Login on shared-backend worktrees still requires a
  // session cookie on the worktree's domain — open with backend: dedicated
  // for full SSO functionality.
  async rewrites() {
    return [
      {
        source: '/api/:path*',
        destination: 'https://api.rlt.sk/api/:path*',
        has: [
          {
            type: 'host',
            value: 'wt-.+\\.dev\\.rlt\\.sk',
          },
        ],
      },
      {
        source: '/api/:path*',
        destination: 'https://api.staging.rlt.sk/api/:path*',
        has: [
          {
            type: 'host',
            value: 'wt-.+\\.staging\\.rlt\\.sk',
          },
        ],
      },
    ];
  },
};

module.exports = withNextIntl(nextConfig);
