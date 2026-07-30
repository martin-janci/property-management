import react from '@vitejs/plugin-react';
import { configDefaults, defineConfig } from 'vitest/config';

export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./vitest.setup.ts'],
    // Only collect unit/component tests under src/. The Playwright e2e specs
    // under e2e/ use `@playwright/test` (test.describe/registerNavigationSpecs)
    // and must NOT be picked up by Vitest's default `**/*.spec.ts` glob —
    // otherwise they fail collection and structurally red the verify gate.
    include: ['src/**/*.{test,spec}.{ts,tsx}'],
    exclude: [...configDefaults.exclude, 'e2e/**'],
  },
});
