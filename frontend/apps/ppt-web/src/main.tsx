import { DevPanel, type ApiMode, getMode } from '@ppt/dev-panel';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import { ErrorBoundary } from './components/ErrorBoundary';
import './i18n'; // Initialize i18n
import { worker } from './mocks/browser';

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 1000 * 60 * 5, // 5 minutes
      retry: 1,
    },
  },
});

const defaultMode: ApiMode = (import.meta.env.VITE_API_DEFAULT as ApiMode) || 'local';
const initialMode = getMode(defaultMode);

async function bootstrap() {
  if (import.meta.env.DEV && initialMode === 'mock') {
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
        <DevPanel
          defaultMode={defaultMode}
          onModeChange={() => window.location.reload()}
        />
      )}
    </React.StrictMode>
  );
}

void bootstrap();
