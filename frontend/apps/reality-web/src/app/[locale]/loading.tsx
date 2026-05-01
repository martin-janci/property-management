/**
 * Route loading state (Next.js convention) for the [locale] segment.
 */

import { LoadingSkeleton } from '@/components/states';

export default function Loading() {
  return (
    <main className="loading-page" aria-label="Loading page">
      <div className="loading-card">
        <LoadingSkeleton width="60%" height="2rem" />
        <div style={{ height: 16 }} />
        <LoadingSkeleton lines={3} />
        <div style={{ height: 24 }} />
        <LoadingSkeleton cards={3} />
      </div>
      <style>{`
        .loading-page {
          min-height: 100vh;
          padding: 32px 16px;
          background: var(--ppt-bg-app);
          display: flex;
          justify-content: center;
        }
        .loading-card {
          width: 100%;
          max-width: 960px;
          background: var(--ppt-bg-surface);
          padding: 32px;
          border-radius: 12px;
          box-shadow: 0 1px 3px rgba(0,0,0,.06);
        }
      `}</style>
    </main>
  );
}
