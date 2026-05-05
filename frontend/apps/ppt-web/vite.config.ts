import react from '@vitejs/plugin-react';
import { visualizer } from 'rollup-plugin-visualizer';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [
    react(),
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
