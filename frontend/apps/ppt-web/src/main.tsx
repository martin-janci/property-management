import '@ppt/ui-kit/tokens.css'; // Design system tokens (colors, spacing, type, dark mode)
import './index.css'; // Tailwind base + components + utilities + minimal app shell styles
import { client } from '@ppt/api-client';
import { type ApiMode, DevPanel, getMode, parseMode } from '@ppt/dev-panel';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import { ErrorBoundary } from './components/ErrorBoundary';
import './i18n'; // Initialize i18n

// MSW is intentionally NOT imported at the top — it's loaded via dynamic import only
// when bootstrap detects mode === 'mock' AND DEV. Keeps the production bundle MSW-free.

// Override the generated client's hardcoded BASE with the configured API URL.
// VITE_API_URL is set in .env.* files; falls back to empty string so that
// Vite's dev-server proxy handles /api/* requests without a host prefix.
client.setConfig({
  baseUrl: import.meta.env.VITE_API_URL || '',
});

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 1000 * 60 * 5, // 5 minutes
      retry: 1,
    },
  },
});

// Validate the build-time env override against the allowed ApiMode values so
// a typo in VITE_API_DEFAULT can't put the app in an unknown mode.
const defaultMode: ApiMode = parseMode(import.meta.env.VITE_API_DEFAULT);
const initialMode = getMode(defaultMode);

async function bootstrap() {
  if (import.meta.env.DEV && initialMode === 'mock') {
    const { worker } = await import('./mocks/browser');
    await worker.start({ onUnhandledRequest: 'bypass' });
  }

  ReactDOM.createRoot(document.getElementById('root')!).render(
    <React.StrictMode>
      <ErrorBoundary>
        <QueryClientProvider client={queryClient}>
          <App />
        </QueryClientProvider>
      </ErrorBoundary>
      {import.meta.env.DEV && (
        <DevPanel defaultMode={defaultMode} onModeChange={() => window.location.reload()} />
      )}
    </React.StrictMode>
  );
}

void bootstrap();
