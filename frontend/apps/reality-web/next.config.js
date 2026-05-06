const createNextIntlPlugin = require('next-intl/plugin');
const { detectWorktree } = require('@ppt/vite-plugin-worktree/detect');

const withNextIntl = createNextIntlPlugin('./src/i18n/request.ts');

const worktree = detectWorktree();

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
    NEXT_PUBLIC_PPT_WORKTREE_NAME: worktree.name,
    NEXT_PUBLIC_PPT_WORKTREE_BRANCH: worktree.branch,
    NEXT_PUBLIC_PPT_IS_WORKTREE: String(worktree.isWorktree),
  },
};

module.exports = withNextIntl(nextConfig);
