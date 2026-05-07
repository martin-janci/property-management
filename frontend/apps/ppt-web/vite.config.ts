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
    // built-in allowlist (localhost variants only). Worktree dev runs reach
    // the container via the deploy-server's Caddy reverse proxy at hosts like
    // `wt-<name>.dev.ppt.rlt.sk` — without this they 403 with
    // `Blocked request. This host (...) is not allowed`.
    //
    // A leading-dot entry is Vite's documented suffix-match form: `.dev.ppt.rlt.sk`
    // matches `dev.ppt.rlt.sk` itself AND any subdomain of it (`wt-foo.dev.ppt.rlt.sk`).
    // Explicit list keeps Vite's DNS-rebinding protection intact — random hostnames
    // pointed at the dev server are still rejected, only our worktree subdomains
    // pass. Only ever applied in `vite dev`; production builds don't run a server.
    allowedHosts: ['.dev.ppt.rlt.sk'],
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
        // Manual chunks for better code splitting
        manualChunks: {
          // Vendor chunks - libraries that rarely change
          'vendor-react': ['react', 'react-dom', 'react-router-dom'],
          'vendor-tanstack': ['@tanstack/react-query'],
          'vendor-i18n': ['react-i18next', 'i18next'],
        },
      },
    },
  },
});
