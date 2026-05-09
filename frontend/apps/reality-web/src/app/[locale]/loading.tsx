/**
 * Route loading state (Next.js convention) for the [locale] segment.
 * Renders the Header so the navigation bar doesn't flash out during page transitions.
 */

import { LoadingSkeleton } from '@/components/states';
import { Header } from '@/components/ui';

export default function Loading() {
  return (
    <div className="loading-page" aria-label="Loading page">
      <Header />
      <main className="loading-main">
        <div className="loading-card">
          <LoadingSkeleton width="60%" height="2rem" />
          <div style={{ height: 16 }} />
          <LoadingSkeleton lines={3} />
          <div style={{ height: 24 }} />
          <LoadingSkeleton cards={3} />
        </div>
      </main>
      <style>{`
        .loading-page {
          min-height: 100vh;
          display: flex;
          flex-direction: column;
          background: var(--ppt-bg-app);
        }
        .loading-main {
          flex: 1;
          padding: 32px 16px;
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
    </div>
  );
}
