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
