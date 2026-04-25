'use client';

/**
 * Runtime error boundary (Next.js convention) for the [locale] segment.
 *
 * Receives the thrown error and a reset function from Next.js. We log the
 * error to the console for diagnostics in development; production
 * deployments should wire it to their telemetry pipeline.
 */

import { StateView } from '@/components/states';
import { useEffect } from 'react';

interface ErrorPageProps {
  error: Error & { digest?: string };
  reset: () => void;
}

export default function ErrorPage({ error, reset }: ErrorPageProps) {
  useEffect(() => {
    console.error('reality-web error boundary:', error);
  }, [error]);

  return (
    <StateView
      icon="🛠️"
      code="500"
      title="Something went wrong"
      description="An unexpected error occurred. Please try again in a moment."
    >
      <button type="button" className="state-action" onClick={reset}>
        Try again
      </button>
      <style jsx global>{`
        .state-action {
          padding: 10px 20px; border-radius: 8px; border: none; cursor: pointer;
          background: #2563eb; color: #fff; font-weight: 500;
          text-align: center; font-size: 15px;
        }
        .state-action:hover { background: #1d4ed8; }
      `}</style>
    </StateView>
  );
}
