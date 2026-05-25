/**
 * Metro Bundler Configuration
 *
 * Epic 85 - Story 85.1: Environment Variable Setup
 *
 * Extends @expo/metro-config with monorepo workspace support so Metro
 * can resolve @ppt/api-client and @ppt/shared from the frontend/packages/
 * directory outside the app root.
 *
 * Environment variable loading:
 *   Expo SDK 49+ automatically loads EXPO_PUBLIC_* vars from .env files.
 *   Variable resolution order (highest priority first):
 *     1. EXPO_PUBLIC_* in shell / CI environment
 *     2. EXPO_PUBLIC_* in .env.local
 *     3. EXPO_PUBLIC_* in .env.<APP_ENV>  (e.g. .env.development)
 *     4. EXPO_PUBLIC_* in .env
 *
 * See also: app.config.ts for injecting env vars into `extra` and
 * ios.infoPlist (API_BASE_URL, ENVIRONMENT).
 */

// eslint-disable-next-line @typescript-eslint/no-require-imports
const { getDefaultConfig } = require('@expo/metro-config');
// eslint-disable-next-line @typescript-eslint/no-require-imports
const path = require('node:path');

const projectRoot = __dirname;
// Monorepo root is three levels up: mobile/ -> apps/ -> frontend/ -> property-management/
const monorepoRoot = path.resolve(projectRoot, '../../..');

/** @type {import('metro-config').MetroConfig} */
const config = getDefaultConfig(projectRoot);

// ---------------------------------------------------------------------------
// Monorepo workspace support
// ---------------------------------------------------------------------------
config.watchFolders = [monorepoRoot];

config.resolver = {
  ...config.resolver,
  nodeModulesPaths: [
    path.resolve(projectRoot, 'node_modules'),
    path.resolve(monorepoRoot, 'frontend/node_modules'),
    path.resolve(monorepoRoot, 'node_modules'),
  ],
};

module.exports = config;
