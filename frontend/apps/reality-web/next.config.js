const createNextIntlPlugin = require('next-intl/plugin');

const withNextIntl = createNextIntlPlugin('./src/i18n/request.ts');

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
};

module.exports = withNextIntl(nextConfig);
