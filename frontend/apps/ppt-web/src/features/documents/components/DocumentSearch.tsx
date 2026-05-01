/**
 * Document Search Interface (Story 39.1).
 *
 * Full-text search with highlighted snippets and filters.
 */

import {
  DOCUMENT_CATEGORIES,
  type DocumentSearchRequest,
  type DocumentSearchResult,
  type OcrStatus,
  useDocumentSearch,
} from '@ppt/api-client';
import { useCallback, useMemo, useState } from 'react';
import { DocumentSearchResult as SearchResultCard } from './DocumentSearchResult';

interface DocumentSearchProps {
  organizationId: string;
  buildingId?: string;
  onSelectDocument?: (documentId: string) => void;
}

export function DocumentSearch({
  organizationId,
  buildingId,
  onSelectDocument,
}: DocumentSearchProps) {
  const [query, setQuery] = useState('');
  const [debouncedQuery, setDebouncedQuery] = useState('');
  const [selectedCategories, setSelectedCategories] = useState<string[]>([]);
  const [ocrStatusFilter, setOcrStatusFilter] = useState<OcrStatus[]>([]);
  const [hasSummaryFilter, setHasSummaryFilter] = useState<boolean | undefined>();
  const [dateFrom, setDateFrom] = useState<string>('');
  const [dateTo, setDateTo] = useState<string>('');

  // Debounce search query
  const debounceTimeout = useMemo(() => {
    let timeout: ReturnType<typeof setTimeout>;
    return (value: string) => {
      clearTimeout(timeout);
      timeout = setTimeout(() => setDebouncedQuery(value), 300);
    };
  }, []);

  const handleQueryChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const value = e.target.value;
      setQuery(value);
      debounceTimeout(value);
    },
    [debounceTimeout]
  );

  // Build search request
  const searchRequest: DocumentSearchRequest = useMemo(
    () => ({
      query: debouncedQuery,
      organization_id: organizationId,
      building_id: buildingId,
      categories: selectedCategories.length > 0 ? selectedCategories : undefined,
      ocr_status: ocrStatusFilter.length > 0 ? ocrStatusFilter : undefined,
      has_summary: hasSummaryFilter,
      date_from: dateFrom || undefined,
      date_to: dateTo || undefined,
      limit: 20,
    }),
    [
      debouncedQuery,
      organizationId,
      buildingId,
      selectedCategories,
      ocrStatusFilter,
      hasSummaryFilter,
      dateFrom,
      dateTo,
    ]
  );

  const { data, isLoading, error } = useDocumentSearch(searchRequest);

  const toggleCategory = (category: string) => {
    setSelectedCategories((prev) =>
      prev.includes(category) ? prev.filter((c) => c !== category) : [...prev, category]
    );
  };

  const toggleOcrStatus = (status: OcrStatus) => {
    setOcrStatusFilter((prev) =>
      prev.includes(status) ? prev.filter((s) => s !== status) : [...prev, status]
    );
  };

  return (
    <div className="document-search">
      {/* Search Input */}
      <div className="search-header">
        <div className="search-input-container">
          <svg
            className="search-icon"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
            width="20"
            height="20"
            aria-hidden="true"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
            />
          </svg>
          <input
            type="text"
            value={query}
            onChange={handleQueryChange}
            placeholder="Search document contents, titles, and descriptions..."
            className="search-input"
          />
          {isLoading && <span className="loading-spinner" />}
        </div>
      </div>

      {/* Filters */}
      <div className="search-filters">
        {/* Category Filter */}
        <fieldset className="filter-group">
          <legend className="filter-label">Categories</legend>
          <div className="filter-chips">
            {DOCUMENT_CATEGORIES.map((category) => (
              <button
                key={category}
                type="button"
                onClick={() => toggleCategory(category)}
                className={`filter-chip ${selectedCategories.includes(category) ? 'active' : ''}`}
                aria-pressed={selectedCategories.includes(category)}
              >
                {category}
              </button>
            ))}
          </div>
        </fieldset>

        {/* OCR Status Filter */}
        <fieldset className="filter-group">
          <legend className="filter-label">OCR Status</legend>
          <div className="filter-chips">
            {(['completed', 'pending', 'processing', 'failed'] as OcrStatus[]).map((status) => (
              <button
                key={status}
                type="button"
                onClick={() => toggleOcrStatus(status)}
                className={`filter-chip ${ocrStatusFilter.includes(status) ? 'active' : ''}`}
                aria-pressed={ocrStatusFilter.includes(status)}
              >
                {status}
              </button>
            ))}
          </div>
        </fieldset>

        {/* Date Range */}
        <div className="filter-group">
          <span className="filter-label">Date Range</span>
          <div className="date-inputs">
            <input
              type="date"
              value={dateFrom}
              onChange={(e) => setDateFrom(e.target.value)}
              className="date-input"
              aria-label="From date"
            />
            <span>to</span>
            <input
              type="date"
              value={dateTo}
              onChange={(e) => setDateTo(e.target.value)}
              className="date-input"
              aria-label="To date"
            />
          </div>
        </div>

        {/* Summary Filter */}
        <div className="filter-group">
          <label className="filter-checkbox">
            <input
              type="checkbox"
              checked={hasSummaryFilter === true}
              onChange={(e) => setHasSummaryFilter(e.target.checked ? true : undefined)}
            />
            Has AI summary
          </label>
        </div>
      </div>

      {/* Results */}
      <div className="search-results">
        {error && <div className="error-message">Error searching documents: {error.message}</div>}

        {data && debouncedQuery.length >= 2 && (
          <div className="results-header">
            <span>
              Found {data.total} document{data.total !== 1 ? 's' : ''} in {data.took_ms}ms
            </span>
          </div>
        )}

        {data?.results.map((result: DocumentSearchResult) => (
          <SearchResultCard
            key={result.document.id}
            result={result}
            onClick={() => onSelectDocument?.(result.document.id)}
          />
        ))}

        {data && data.results.length === 0 && debouncedQuery.length >= 2 && (
          <div className="no-results">No documents found matching "{debouncedQuery}"</div>
        )}

        {debouncedQuery.length > 0 && debouncedQuery.length < 2 && (
          <div className="search-hint">Type at least 2 characters to search</div>
        )}
      </div>

      <style>{`
        .document-search {
          display: flex;
          flex-direction: column;
          gap: 1rem;
        }

        .search-header {
          position: sticky;
          top: 0;
          background: white;
          z-index: 10;
          padding: 0.5rem 0;
        }

        .search-input-container {
          display: flex;
          align-items: center;
          gap: 0.5rem;
          padding: 0.75rem 1rem;
          border: 1px solid #e2e8f0;
          border-radius: 0.5rem;
          background: #f8fafc;
        }

        .search-input-container:focus-within {
          border-color: var(--ppt-brand-500);
          box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
        }

        .search-icon {
          color: #94a3b8;
          flex-shrink: 0;
        }

        .search-input {
          flex: 1;
          border: none;
          background: transparent;
          font-size: 1rem;
          outline: none;
        }

        .loading-spinner {
          width: 20px;
          height: 20px;
          border: 2px solid #e2e8f0;
          border-top-color: #3b82f6;
          border-radius: 50%;
          animation: spin 0.6s linear infinite;
        }

        @keyframes spin {
          to { transform: rotate(360deg); }
        }

        .search-filters {
          display: flex;
          flex-wrap: wrap;
          gap: 1rem;
          padding: 1rem;
          background: #f8fafc;
          border-radius: 0.5rem;
        }

        .filter-group {
          display: flex;
          flex-direction: column;
          gap: 0.5rem;
          border: none;
          padding: 0;
          margin: 0;
        }

        .filter-label {
          font-size: 0.875rem;
          font-weight: 500;
          color: #475569;
          padding: 0;
        }

        .filter-chips {
          display: flex;
          flex-wrap: wrap;
          gap: 0.25rem;
        }

        .filter-chip {
          padding: 0.25rem 0.75rem;
          font-size: 0.75rem;
          border: 1px solid #e2e8f0;
          border-radius: 9999px;
          background: white;
          color: #64748b;
          cursor: pointer;
          transition: all 0.15s;
        }

        .filter-chip:hover {
          border-color: #3b82f6;
          color: #3b82f6;
        }

        .filter-chip.active {
          background: #3b82f6;
          border-color: #3b82f6;
          color: white;
        }

        .date-inputs {
          display: flex;
          align-items: center;
          gap: 0.5rem;
        }

        .date-input {
          padding: 0.25rem 0.5rem;
          border: 1px solid #e2e8f0;
          border-radius: 0.25rem;
          font-size: 0.875rem;
        }

        .filter-checkbox {
          display: flex;
          align-items: center;
          gap: 0.5rem;
          font-size: 0.875rem;
          color: #475569;
          cursor: pointer;
        }

        .search-results {
          display: flex;
          flex-direction: column;
          gap: 0.75rem;
        }

        .results-header {
          font-size: 0.875rem;
          color: #64748b;
          padding: 0.5rem 0;
        }

        .error-message {
          padding: 1rem;
          background: #fef2f2;
          border: 1px solid #fecaca;
          border-radius: 0.5rem;
          color: #dc2626;
        }

        .no-results,
        .search-hint {
          padding: 2rem;
          text-align: center;
          color: #64748b;
        }
      `}</style>
    </div>
  );
}
