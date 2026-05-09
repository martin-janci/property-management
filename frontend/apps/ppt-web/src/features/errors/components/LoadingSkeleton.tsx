/**
 * LoadingSkeleton — accessible shimmer placeholder.
 *
 * Use the `lines` variant for paragraph-style placeholders, the `card`
 * variant for card lists, or pass explicit `width` / `height` for
 * single-block placeholders.
 */

import type { CSSProperties } from 'react';
import '../styles/errorPages.css';

export interface LoadingSkeletonProps {
  /** Render N stacked line placeholders. */
  lines?: number;
  /** Render N card placeholders. */
  cards?: number;
  /** Explicit width (CSS value). */
  width?: string;
  /** Explicit height (CSS value). Defaults to 1rem. */
  height?: string;
  /** Optional accessible label announced to screen readers. */
  label?: string;
}

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
            className="skeleton"
            style={{
              width: '100%',
              height: '100px',
              marginBottom: '12px',
              borderRadius: '12px',
            }}
          />
        ))}
        <span className="sr-only" style={visuallyHidden}>
          {label}
        </span>
      </div>
    );
  }

  if (lines != null) {
    return (
      <div role="status" aria-live="polite" aria-label={label} style={{ width: '100%' }}>
        {Array.from({ length: lines }).map((_, i) => (
          <div
            key={`skeleton-line-${i}`}
            className="skeleton"
            style={{
              width: i === lines - 1 ? '70%' : '100%',
              height,
              marginBottom: '0.5rem',
            }}
          />
        ))}
        <span style={visuallyHidden}>{label}</span>
      </div>
    );
  }

  return (
    <div role="status" aria-live="polite" aria-label={label} style={{ width }}>
      <div className="skeleton" style={{ width, height }} />
      <span style={visuallyHidden}>{label}</span>
    </div>
  );
}

const visuallyHidden: CSSProperties = {
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

LoadingSkeleton.displayName = 'LoadingSkeleton';
