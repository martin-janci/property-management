/**
 * Global Error Boundary Component.
 *
 * Catches JavaScript errors anywhere in the child component tree,
 * logs those errors, and displays a fallback UI instead of crashing.
 *
 * Features:
 * - Catches render errors, lifecycle errors, and errors in constructors
 * - Displays user-friendly error message with retry option
 * - Logs error details for debugging
 * - Optional error reporting callback for external services
 * - Supports custom fallback UI
 * - Internationalized error messages via i18n
 *
 * @see https://react.dev/reference/react/Component#catching-rendering-errors-with-an-error-boundary
 */

import { Component, type ErrorInfo, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import './ErrorBoundary.css';

/** Props for the ErrorBoundary component */
export interface ErrorBoundaryProps {
  /** Child components to render */
  children: ReactNode;
  /** Optional custom fallback UI to display on error */
  fallback?: ReactNode;
  /** Optional callback for error reporting (e.g., to Sentry) */
  onError?: (error: Error, errorInfo: ErrorInfo) => void;
  /** Optional title for the error message */
  title?: string;
}

/** Internal props including translations */
interface ErrorBoundaryInternalProps extends ErrorBoundaryProps {
  translations: {
    title: string;
    message: string;
    tryAgain: string;
    reloadPage: string;
    copyErrorDetails: string;
    copyErrorDetailsTitle: string;
  };
}

/** State for the ErrorBoundary component */
interface ErrorBoundaryState {
  /** Whether an error has been caught */
  hasError: boolean;
  /** The caught error */
  error: Error | null;
  /** Additional error info from React */
  errorInfo: ErrorInfo | null;
}

/**
 * Internal Error Boundary class component.
 * Wrapped by the functional ErrorBoundary for i18n support.
 */
class ErrorBoundaryInternal extends Component<ErrorBoundaryInternalProps, ErrorBoundaryState> {
  static displayName = 'ErrorBoundaryInternal';

  constructor(props: ErrorBoundaryInternalProps) {
    super(props);
    this.state = {
      hasError: false,
      error: null,
      errorInfo: null,
    };
  }

  /**
   * Update state to render fallback UI on next render.
   */
  static getDerivedStateFromError(error: Error): Partial<ErrorBoundaryState> {
    return { hasError: true, error };
  }

  /**
   * Log error information and call optional error handler.
   */
  componentDidCatch(error: Error, errorInfo: ErrorInfo): void {
    // Log to console for development
    console.error('ErrorBoundary caught an error:', error);
    console.error('Component stack:', errorInfo.componentStack);

    // Update state with error info
    this.setState({ errorInfo });

    // Call optional error handler (e.g., for Sentry reporting)
    if (this.props.onError) {
      this.props.onError(error, errorInfo);
    }
  }

  /**
   * Reset error state to allow retry.
   */
  handleRetry = (): void => {
    this.setState({
      hasError: false,
      error: null,
      errorInfo: null,
    });
  };

  /**
   * Copy error details to clipboard for support.
   */
  handleCopyError = async (): Promise<void> => {
    const { error, errorInfo } = this.state;
    const errorDetails = [
      `Error: ${error?.message || 'Unknown error'}`,
      `Stack: ${error?.stack || 'No stack trace'}`,
      `Component Stack: ${errorInfo?.componentStack || 'No component stack'}`,
      `Timestamp: ${new Date().toISOString()}`,
      `URL: ${window.location.href}`,
      `User Agent: ${navigator.userAgent}`,
    ].join('\n\n');

    try {
      await navigator.clipboard.writeText(errorDetails);
    } catch (copyError) {
      // Clipboard API failed (e.g., permissions denied, insecure context)
      // Modern browsers (Chrome 63+, Firefox 53+, Safari 13.1+) all support Clipboard API
      // We log the error for debugging rather than using deprecated fallbacks
      console.error('Failed to copy error details to clipboard:', copyError);
    }
  };

  render(): ReactNode {
    const { hasError, error } = this.state;
    const { children, fallback, title, translations } = this.props;

    if (hasError) {
      // Use custom fallback if provided
      if (fallback) {
        return fallback;
      }

      // Use provided title or fall back to translation
      const displayTitle = title || translations.title;

      // Default error UI
      return (
        <div className="error-boundary" role="alert">
          <div className="error-boundary-content">
            <div className="error-boundary-icon">
              <svg
                width="48"
                height="48"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <circle cx="12" cy="12" r="10" />
                <line x1="12" y1="8" x2="12" y2="12" />
                <line x1="12" y1="16" x2="12.01" y2="16" />
              </svg>
            </div>
            <h1 className="error-boundary-title">{displayTitle}</h1>
            <p className="error-boundary-message">{translations.message}</p>
            {/*
             * The raw `error.message` can carry internal server/DB/stack detail
             * and ships English regardless of locale, so it is NEVER surfaced to
             * the user in a production build — only the generic i18n copy above
             * is shown. The raw detail is kept for debugging in development only.
             * Mirrors `devDetail` in `lib/describeError` and `errorHandler`.
             */}
            {import.meta.env.DEV && error && (
              <p className="error-boundary-details">
                <code>{error.message}</code>
              </p>
            )}
            <div className="error-boundary-actions">
              <button
                type="button"
                className="error-boundary-button error-boundary-button--primary"
                onClick={this.handleRetry}
              >
                {translations.tryAgain}
              </button>
              <button
                type="button"
                className="error-boundary-button error-boundary-button--secondary"
                onClick={() => window.location.reload()}
              >
                {translations.reloadPage}
              </button>
              <button
                type="button"
                className="error-boundary-button error-boundary-button--tertiary"
                onClick={this.handleCopyError}
                title={translations.copyErrorDetailsTitle}
              >
                {translations.copyErrorDetails}
              </button>
            </div>
          </div>
        </div>
      );
    }

    return children;
  }
}

/**
 * Error Boundary component for catching and handling React errors.
 * Wraps the class component to provide i18n support.
 *
 * @example
 * ```tsx
 * // Basic usage
 * <ErrorBoundary>
 *   <App />
 * </ErrorBoundary>
 *
 * // With custom fallback
 * <ErrorBoundary fallback={<div>Custom error message</div>}>
 *   <App />
 * </ErrorBoundary>
 *
 * // With error reporting
 * <ErrorBoundary onError={(error) => reportToSentry(error)}>
 *   <App />
 * </ErrorBoundary>
 * ```
 */
export function ErrorBoundary(props: ErrorBoundaryProps): ReactNode {
  const { t } = useTranslation();

  const translations = {
    title: t('errors.somethingWentWrong'),
    message: t('errors.unexpectedError'),
    tryAgain: t('common.tryAgain'),
    reloadPage: t('common.reloadPage'),
    copyErrorDetails: t('common.copyErrorDetails'),
    copyErrorDetailsTitle: t('errors.copyErrorDetailsSupport'),
  };

  return <ErrorBoundaryInternal {...props} translations={translations} />;
}

ErrorBoundary.displayName = 'ErrorBoundary';

export default ErrorBoundary;
