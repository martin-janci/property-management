import '@ppt/ui-kit/tokens.css'; // Design system tokens (colors, spacing, type, dark mode)
import { OpenAPI } from '@ppt/api-client';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import { ErrorBoundary } from './components/ErrorBoundary';
import './i18n'; // Initialize i18n

// Override the generated client's hardcoded BASE with the configured API URL.
// VITE_API_URL is set in .env.* files; falls back to empty string so that
// Vite's dev-server proxy handles /api/* requests without a host prefix.
OpenAPI.BASE = import.meta.env.VITE_API_URL || '';

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 1000 * 60 * 5, // 5 minutes
      retry: 1,
    },
  },
});

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <ErrorBoundary>
      <QueryClientProvider client={queryClient}>
        <App />
      </QueryClientProvider>
    </ErrorBoundary>
  </React.StrictMode>
);
