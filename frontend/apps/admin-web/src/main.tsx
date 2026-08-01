import '@ppt/ui-kit/tokens.css'; // Legacy tokens — admin Toast/CSS uses --ppt-* vars.
import '@ppt/ui-kit/tokens/tokens.css'; // Design-system tokens (--accent/--bg-surface/…) used by @ppt/ui-kit primitives (Stepper, FileUpload, …).
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import React from 'react';
import ReactDOM from 'react-dom/client';
import { BrowserRouter } from 'react-router-dom';

import './i18n';
import { App } from './App';

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: 2,
      staleTime: 30_000,
    },
  },
});

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <App />
      </BrowserRouter>
    </QueryClientProvider>
  </React.StrictMode>
);
