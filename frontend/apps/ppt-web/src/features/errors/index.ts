/**
 * Error / empty / loading state surfaces for ppt-web.
 *
 * Provides full-page error screens (404, 403, 500, session-expired) and
 * three reusable inline state primitives (EmptyState, ErrorState,
 * LoadingSkeleton) used across feature pages.
 */

export type { EmptyStateProps } from './components/EmptyState';
export { EmptyState } from './components/EmptyState';
export type { ErrorStateProps } from './components/ErrorState';
export { ErrorState } from './components/ErrorState';
export type { LoadingSkeletonProps } from './components/LoadingSkeleton';
export { LoadingSkeleton } from './components/LoadingSkeleton';
export type { StateViewProps } from './components/StateView';
export { StateView } from './components/StateView';
export { ForbiddenPage } from './pages/ForbiddenPage';
export { NotFoundPage } from './pages/NotFoundPage';
export type { ServerErrorPageProps } from './pages/ServerErrorPage';
export { ServerErrorPage } from './pages/ServerErrorPage';
export { SessionExpiredPage } from './pages/SessionExpiredPage';
