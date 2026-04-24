'use client';

/**
 * LoadingSkeleton — accessible shimmer placeholder.
 */

export interface LoadingSkeletonProps {
  lines?: number;
  cards?: number;
  width?: string;
  height?: string;
  label?: string;
}

const visuallyHidden: React.CSSProperties = {
  position: 'absolute',
  width: '1px',
  height: '1px',
  padding: 0,
  margin: '-1px',
  overflow: 'hidden',
  clip: 'rect(0, 0, 0, 0)',
  whiteSpace: 'nowrap',
  border: 0,
};

export function LoadingSkeleton({
  lines,
  cards,
  width = '100%',
  height = '1rem',
  label = 'Loading…',
}: LoadingSkeletonProps) {
  if (cards != null) {
    return (
      <div role="status" aria-live="polite" aria-label={label} style={{ width: '100%' }}>
        {Array.from({ length: cards }).map((_, i) => (
          <div
            key={`skeleton-card-${i}`}
            className="skeleton card"
          />
        ))}
        <span style={visuallyHidden}>{label}</span>
        <style jsx>{`
          .skeleton {
            background: linear-gradient(90deg, #f3f4f6 0%, #e5e7eb 50%, #f3f4f6 100%);
            background-size: 200% 100%;
            border-radius: 12px;
            animation: shimmer 1.5s ease-in-out infinite;
          }
          .card { width: 100%; height: 100px; margin-bottom: 12px; }
          @keyframes shimmer {
            0% { background-position: 200% 0; }
            100% { background-position: -200% 0; }
          }
          @media (prefers-reduced-motion: reduce) {
            .skeleton { animation: none; }
          }
        `}</style>
      </div>
    );
  }

  if (lines != null) {
    return (
      <div role="status" aria-live="polite" aria-label={label} style={{ width: '100%' }}>
        {Array.from({ length: lines }).map((_, i) => (
          <div
            key={`skeleton-line-${i}`}
            className="skeleton line"
            style={{
              width: i === lines - 1 ? '70%' : '100%',
              height,
            }}
          />
        ))}
        <span style={visuallyHidden}>{label}</span>
        <style jsx>{`
          .skeleton {
            background: linear-gradient(90deg, #f3f4f6 0%, #e5e7eb 50%, #f3f4f6 100%);
            background-size: 200% 100%;
            border-radius: 6px;
            animation: shimmer 1.5s ease-in-out infinite;
          }
          .line { margin-bottom: 0.5rem; }
          @keyframes shimmer {
            0% { background-position: 200% 0; }
            100% { background-position: -200% 0; }
          }
          @media (prefers-reduced-motion: reduce) {
            .skeleton { animation: none; }
          }
        `}</style>
      </div>
    );
  }

  return (
    <div role="status" aria-live="polite" aria-label={label} style={{ width }}>
      <div className="skeleton" style={{ width, height }} />
      <span style={visuallyHidden}>{label}</span>
      <style jsx>{`
        .skeleton {
          background: linear-gradient(90deg, #f3f4f6 0%, #e5e7eb 50%, #f3f4f6 100%);
          background-size: 200% 100%;
          border-radius: 6px;
          animation: shimmer 1.5s ease-in-out infinite;
        }
        @keyframes shimmer {
          0% { background-position: 200% 0; }
          100% { background-position: -200% 0; }
        }
        @media (prefers-reduced-motion: reduce) {
          .skeleton { animation: none; }
        }
      `}</style>
    </div>
  );
}
