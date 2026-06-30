import path from 'node:path';
import react from '@vitejs/plugin-react';
/// <reference types="vitest" />
import { defineConfig } from 'vitest/config';

export default defineConfig({
  plugins: [react()],
  resolve: {
    // Mirror tsconfig's `@/*` -> `src/*` mapping so test files and the modules
    // they import (e.g. `@/i18n/config`, `@/styles/...`) resolve under vitest.
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: ['./src/test/setup.tsx'],
    include: ['src/**/*.{test,spec}.{ts,tsx}'],
  },
});
