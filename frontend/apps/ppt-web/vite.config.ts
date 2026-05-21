import pptWorktreePlugin from '@ppt/vite-plugin-worktree';
import react from '@vitejs/plugin-react';
import { visualizer } from 'rollup-plugin-visualizer';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [
    react(),
    pptWorktreePlugin(),
    // Bundle analyzer - generates stats.html after build
    visualizer({
      filename: 'dist/stats.html',
      gzipSize: true,
      brotliSize: true,
      open: false, // Don't auto-open in browser
    }),
  ],
  server: {
    port: 3000,
    // Vite 5+ rejects requests whose Host header doesn't match a small
    // built-in allowlist (localhost variants only). All ppt-web environments
    // that ever reach the dev server through Caddy live under `ppt.rlt.sk`:
    //   - worktrees: `wt-<name>.dev.ppt.rlt.sk`
    //   - staging:   `staging.ppt.rlt.sk` (and any future `*.staging.ppt.rlt.sk`)
    //   - live:      `ppt.rlt.sk`         (and any future `*.ppt.rlt.sk`)
    // A single leading-dot entry covers all three: Vite's suffix-match form
    // `.ppt.rlt.sk` matches `ppt.rlt.sk` itself AND any subdomain at any depth
    // (so `wt-foo.dev.ppt.rlt.sk` ✓, `staging.ppt.rlt.sk` ✓, `ppt.rlt.sk` ✓).
    // DNS-rebinding protection stays intact — anything not under ppt.rlt.sk
    // is still rejected. Only ever applied in `vite dev`; production builds
    // don't run a server.
    allowedHosts: ['.ppt.rlt.sk'],
    // Proxy /api/* to the api-server in local dev so relative-path fetches
    // (OpenAPI.BASE = '' and VITE_API_BASE_URL = '') resolve correctly without
    // setting VITE_API_URL. In production VITE_API_URL must be set explicitly.
    proxy: {
      '/api': {
        target: 'http://localhost:8080',
        changeOrigin: true,
      },
    },
  },
  build: {
    outDir: 'dist',
    // Target modern browsers for smaller bundles
    target: 'es2020',
    // Enable source maps for debugging
    sourcemap: true,
    rollupOptions: {
      output: {
        // Manual chunks for better code splitting.
        // Vite 8 bundles with Rolldown, which only supports the *function*
        // form of `manualChunks` — the object form (chunk-name -> module list)
        // that classic Rollup accepted is rejected with "manualChunks is not
        // a function". This function reproduces the same grouping by matching
        // each module's path against the vendor packages.
        manualChunks(id) {
          if (!id.includes('node_modules')) return undefined;
          // Vendor chunks - libraries that rarely change
          if (
            /[\\/]node_modules[\\/](react|react-dom|react-router-dom|react-router)[\\/]/.test(id)
          ) {
            return 'vendor-react';
          }
          if (/[\\/]node_modules[\\/]@tanstack[\\/]/.test(id)) {
            return 'vendor-tanstack';
          }
          if (/[\\/]node_modules[\\/](react-i18next|i18next)[\\/]/.test(id)) {
            return 'vendor-i18n';
          }
          return undefined;
        },
      },
    },
  },
});
