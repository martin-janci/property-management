/**
 * Pagination component for navigating through pages.
 */

import type React from 'react';
import { forwardRef, useMemo } from 'react';
import './Pagination.css';

export interface PaginationProps extends React.HTMLAttributes<HTMLElement> {
  /** Current page (1-indexed) */
  currentPage: number;
  /** Total number of pages */
  totalPages: number;
  /** Callback when page changes */
  onPageChange: (page: number) => void;
  /** Number of page buttons to show around current page */
  siblingCount?: number;
  /** Whether to show first/last page buttons */
  showFirstLast?: boolean;
  /** Label for previous button */
  previousLabel?: string;
  /** Label for next button */
  nextLabel?: string;
  /** Label for first button */
  firstLabel?: string;
  /** Label for last button */
  lastLabel?: string;
  /** Whether the pagination is disabled */
  disabled?: boolean;
}

/**
 * Pagination component for page navigation.
 *
 * @example
 * ```tsx
 * <Pagination
 *   currentPage={page}
 *   totalPages={10}
 *   onPageChange={setPage}
 * />
 * ```
 */
export const Pagination = forwardRef<HTMLElement, PaginationProps>(
  (
    {
      currentPage,
      totalPages,
      onPageChange,
      siblingCount = 1,
      showFirstLast = true,
      previousLabel = 'Previous',
      nextLabel = 'Next',
      firstLabel = 'First',
      lastLabel = 'Last',
      disabled = false,
      className = '',
      ...props
    },
    ref
  ) => {
    const pages = useMemo(() => {
      return generatePagination(currentPage, totalPages, siblingCount);
    }, [currentPage, totalPages, siblingCount]);

    const classes = ['ppt-pagination', disabled && 'ppt-pagination--disabled', className]
      .filter(Boolean)
      .join(' ');

    const handlePageClick = (page: number) => {
      if (!disabled && page >= 1 && page <= totalPages && page !== currentPage) {
        onPageChange(page);
      }
    };

    if (totalPages <= 1) {
      return null;
    }

    return (
      <nav
        ref={ref}
        className={classes}
        aria-label="Pagination"
        {...props}
      >
        <ul className="ppt-pagination__list">
          {/* First */}
          {showFirstLast && (
            <li className="ppt-pagination__item">
              <button
                type="button"
                className="ppt-pagination__button ppt-pagination__button--nav"
                onClick={() => handlePageClick(1)}
                disabled={disabled || currentPage === 1}
                aria-label={firstLabel}
                title={firstLabel}
              >
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <polyline points="11 17 6 12 11 7" />
                  <polyline points="18 17 13 12 18 7" />
                </svg>
              </button>
            </li>
          )}

          {/* Previous */}
          <li className="ppt-pagination__item">
            <button
              type="button"
              className="ppt-pagination__button ppt-pagination__button--nav"
              onClick={() => handlePageClick(currentPage - 1)}
              disabled={disabled || currentPage === 1}
              aria-label={previousLabel}
              title={previousLabel}
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <polyline points="15 18 9 12 15 6" />
              </svg>
            </button>
          </li>

          {/* Page numbers */}
          {pages.map((page, index) => (
            <li key={index} className="ppt-pagination__item">
              {page === 'ellipsis' ? (
                <span className="ppt-pagination__ellipsis" aria-hidden="true">
                  ...
                </span>
              ) : (
                <button
                  type="button"
                  className={`ppt-pagination__button ${page === currentPage ? 'ppt-pagination__button--active' : ''}`}
                  onClick={() => handlePageClick(page)}
                  disabled={disabled}
                  aria-current={page === currentPage ? 'page' : undefined}
                  aria-label={`Page ${page}`}
                >
                  {page}
                </button>
              )}
            </li>
          ))}

          {/* Next */}
          <li className="ppt-pagination__item">
            <button
              type="button"
              className="ppt-pagination__button ppt-pagination__button--nav"
              onClick={() => handlePageClick(currentPage + 1)}
              disabled={disabled || currentPage === totalPages}
              aria-label={nextLabel}
              title={nextLabel}
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <polyline points="9 18 15 12 9 6" />
              </svg>
            </button>
          </li>

          {/* Last */}
          {showFirstLast && (
            <li className="ppt-pagination__item">
              <button
                type="button"
                className="ppt-pagination__button ppt-pagination__button--nav"
                onClick={() => handlePageClick(totalPages)}
                disabled={disabled || currentPage === totalPages}
                aria-label={lastLabel}
                title={lastLabel}
              >
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <polyline points="13 17 18 12 13 7" />
                  <polyline points="6 17 11 12 6 7" />
                </svg>
              </button>
            </li>
          )}
        </ul>
      </nav>
    );
  }
);

Pagination.displayName = 'Pagination';

type PageItem = number | 'ellipsis';

function generatePagination(
  currentPage: number,
  totalPages: number,
  siblingCount: number
): PageItem[] {
  // Always show first page, last page, and pages around current
  const pages: PageItem[] = [];

  // Calculate the range of pages to show
  const leftSibling = Math.max(currentPage - siblingCount, 1);
  const rightSibling = Math.min(currentPage + siblingCount, totalPages);

  // Should we show left ellipsis?
  const showLeftEllipsis = leftSibling > 2;
  // Should we show right ellipsis?
  const showRightEllipsis = rightSibling < totalPages - 1;

  // First page
  pages.push(1);

  // Left ellipsis or second page
  if (showLeftEllipsis) {
    pages.push('ellipsis');
  } else if (leftSibling > 1) {
    // Fill in pages between 1 and leftSibling
    for (let i = 2; i < leftSibling; i++) {
      pages.push(i);
    }
  }

  // Pages around current
  for (let i = leftSibling; i <= rightSibling; i++) {
    if (i !== 1 && i !== totalPages) {
      pages.push(i);
    }
  }

  // Right ellipsis or second-to-last page
  if (showRightEllipsis) {
    pages.push('ellipsis');
  } else if (rightSibling < totalPages) {
    // Fill in pages between rightSibling and totalPages
    for (let i = rightSibling + 1; i < totalPages; i++) {
      pages.push(i);
    }
  }

  // Last page (only if different from first)
  if (totalPages > 1) {
    pages.push(totalPages);
  }

  return pages;
}
