import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [react()],
  server: {
    port: 3100,
    // Vite host-header allowlist. admin lives under .rlt.sk:
    //   prod:    admin.rlt.sk
    //   staging: admin.staging.rlt.sk
    // The leading-dot form matches the apex and any subdomain.
    allowedHosts: ['.rlt.sk', 'localhost'],
    proxy: {
      '/api': {
        target: 'http://localhost:8080',
        changeOrigin: true,
      },
    },
  },
  build: {
    sourcemap: true,
    outDir: 'dist',
  },
});
