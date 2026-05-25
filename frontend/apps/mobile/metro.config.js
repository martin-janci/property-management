/**
 * Metro Bundler Configuration
 *
 * Epic 85 - Story 85.1: Environment Variable Setup
 *
 * Extends @expo/metro-config with:
 *  - EXPO_PUBLIC_* env var support (built-in via Expo SDK 49+)
 *  - Support for workspace packages (@ppt/api-client, @ppt/shared) resolved
 *    from the monorepo root
 *
 * Environment variable loading:
 *   Expo SDK 49+ automatically loads EXPO_PUBLIC_* vars from .env files.
 *   Variable resolution order (highest priority first):
 *     1. EXPO_PUBLIC_* variables in process environment (CI / shell)
 *     2. EXPO_PUBLIC_* variables in .env.local
 *     3. EXPO_PUBLIC_* variables in .env.<APP_ENV>   (e.g. .env.development)
 *     4. EXPO_PUBLIC_* variables in .env
 *
 * See also: app.config.ts for injecting env vars into `extra` and Info.plist.
 */

// eslint-disable-next-line @typescript-eslint/no-require-imports
const { getDefaultConfig } = require('@expo/metro-config');
// eslint-disable-next-line @typescript-eslint/no-require-imports
const path = require('path');

const projectRoot = __dirname;
// Monorepo root is two levels up from frontend/apps/mobile
const monorepoRoot = path.resolve(projectRoot, '../../..');

/** @type {import('metro-config').MetroConfig} */
const config = getDefaultConfig(projectRoot);

// ---------------------------------------------------------------------------
// Monorepo workspace support
// Metro needs to know about workspace packages located outside the app root.
// ---------------------------------------------------------------------------
config.watchFolders = [monorepoRoot];

config.resolver = {
  ...config.resolver,
  // Allow Metro to resolve modules from both the app root and the monorepo root.
  nodeModulesPaths: [
    path.resolve(projectRoot, 'node_modules'),
    path.resolve(monorepoRoot, 'frontend/node_modules'),
    path.resolve(monorepoRoot, 'node_modules'),
  ],
};

module.exports = config;
