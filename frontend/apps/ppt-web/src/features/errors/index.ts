/**
 * Error / empty / loading state surfaces for ppt-web.
 *
 * Provides full-page error screens (404, 403, 500, session-expired) and
 * three reusable inline state primitives (EmptyState, ErrorState,
 * LoadingSkeleton) used across feature pages.
 */

export { StateView } from './components/StateView';
export type { StateViewProps } from './components/StateView';

export { EmptyState } from './components/EmptyState';
export type { EmptyStateProps } from './components/EmptyState';

export { ErrorState } from './components/ErrorState';
export type { ErrorStateProps } from './components/ErrorState';

export { LoadingSkeleton } from './components/LoadingSkeleton';
export type { LoadingSkeletonProps } from './components/LoadingSkeleton';

export { NotFoundPage } from './pages/NotFoundPage';
export { ForbiddenPage } from './pages/ForbiddenPage';
export { ServerErrorPage } from './pages/ServerErrorPage';
export type { ServerErrorPageProps } from './pages/ServerErrorPage';
export { SessionExpiredPage } from './pages/SessionExpiredPage';
