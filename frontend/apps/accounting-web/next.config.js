const createNextIntlPlugin = require('next-intl/plugin');

const withNextIntl = createNextIntlPlugin('./src/i18n/request.ts');

const isDev = process.env.NODE_ENV !== 'production';

// Public site origin — placeholder until the board finalises the product
// name + domain (PAP-303 open decision #1). Worktree/staging deploys override
// via NEXT_PUBLIC_SITE_URL; the deploy-wiring task (#10 / PAP deploy issue)
// owns the real subdomain + DNS.
const apiOrigin = process.env.NEXT_PUBLIC_API_URL;

// Conservative CSP for a public marketing/signup surface. Mirrors the
// reality-web posture but trimmed: this app has no maps/3rd-party embeds yet.
// `connect-src` allows the configured api-server origin so the (future)
// signup POST can reach it once the accounting-api-client (#2) is wired.
const connectSrc = ["'self'", apiOrigin].filter(Boolean).join(' ');

const csp = [
  "default-src 'self'",
  // next/font self-hosts; styled by CSS Modules + ui-kit tokens (no inline <style> from us,
  // but Next injects a small inline bootstrap so 'unsafe-inline' for style is required).
  "style-src 'self' 'unsafe-inline'",
  // Next.js runtime + RSC payload hydration needs 'unsafe-inline'; in dev it also needs eval.
  isDev ? "script-src 'self' 'unsafe-inline' 'unsafe-eval'" : "script-src 'self' 'unsafe-inline'",
  "img-src 'self' data: blob:",
  "font-src 'self' data:",
  `connect-src ${connectSrc}`,
  "frame-ancestors 'none'",
  "base-uri 'self'",
  "form-action 'self'",
].join('; ');

/** @type {import('next').NextConfig} */
const nextConfig = {
  // Standalone output for the reality-web Docker pattern (deploy wiring = #10).
  output: 'standalone',
  reactStrictMode: true,
  transpilePackages: ['@ppt/ui-kit', '@ppt/shared', '@ppt/dev-panel'],
  async headers() {
    return [
      {
        source: '/:path*',
        headers: [
          { key: 'Content-Security-Policy', value: csp },
          { key: 'X-Content-Type-Options', value: 'nosniff' },
          { key: 'X-Frame-Options', value: 'DENY' },
          { key: 'Referrer-Policy', value: 'strict-origin-when-cross-origin' },
        ],
      },
    ];
  },
};

module.exports = withNextIntl(nextConfig);
