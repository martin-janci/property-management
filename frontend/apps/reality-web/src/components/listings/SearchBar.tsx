/**
 * SearchBar Component
 *
 * Search bar with autocomplete for listings (Epic 44, Story 44.2).
 */

'use client';

import { useSearchSuggestions } from '@ppt/reality-api-client';
import { useRouter } from 'next/navigation';
import { useTranslations } from 'next-intl';
import { useEffect, useRef, useState } from 'react';

interface SearchBarProps {
  initialQuery?: string;
  onSearch: (query: string) => void;
}

export function SearchBar({ initialQuery = '', onSearch }: SearchBarProps) {
  const router = useRouter();
  const t = useTranslations('search');
  const [query, setQuery] = useState(initialQuery);
  const [showSuggestions, setShowSuggestions] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  const { data: suggestions } = useSearchSuggestions(query);

  // Close suggestions when clicking outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(event.target as Node)) {
        setShowSuggestions(false);
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSearch(query);
    setShowSuggestions(false);
  };

  const handleSuggestionClick = (suggestion: { type: string; value: string }) => {
    if (suggestion.type === 'listing') {
      router.push(`/listings/${suggestion.value}`);
    } else {
      onSearch(suggestion.value);
      setQuery(suggestion.value);
    }
    setShowSuggestions(false);
  };

  const getSuggestionIcon = (type: string) => {
    switch (type) {
      case 'city':
        return (
          <svg
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            aria-hidden="true"
          >
            <path d="M21 10c0 7-9 13-9 13s-9-6-9-13a9 9 0 0 1 18 0z" />
            <circle cx="12" cy="10" r="3" />
          </svg>
        );
      case 'district':
        return (
          <svg
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            aria-hidden="true"
          >
            <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
            <path d="M3 9h18M9 21V9" />
          </svg>
        );
      case 'listing':
        return (
          <svg
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            aria-hidden="true"
          >
            <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
          </svg>
        );
      default:
        return (
          <svg
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            aria-hidden="true"
          >
            <circle cx="11" cy="11" r="8" />
            <path d="m21 21-4.35-4.35" />
          </svg>
        );
    }
  };

  return (
    <div className="search-container" ref={containerRef}>
      <form className="search-form" onSubmit={handleSubmit}>
        <svg
          className="search-icon"
          width="20"
          height="20"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          aria-hidden="true"
        >
          <circle cx="11" cy="11" r="8" />
          <path d="m21 21-4.35-4.35" />
        </svg>
        <input
          ref={inputRef}
          type="text"
          className="search-input"
          placeholder={t('placeholder')}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onFocus={() => setShowSuggestions(true)}
        />
        {query && (
          <button
            type="button"
            className="clear-button"
            onClick={() => {
              setQuery('');
              onSearch('');
              inputRef.current?.focus();
            }}
            aria-label="Clear search"
          >
            <svg
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              aria-hidden="true"
            >
              <path d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        )}
        <button type="submit" className="search-button">
          {t('button')}
        </button>
      </form>

      {/* Suggestions dropdown */}
      {showSuggestions && suggestions && suggestions.length > 0 && (
        <div className="suggestions">
          {suggestions.map((suggestion, index) => (
            <button
              key={`suggestion-${suggestion.type}-${suggestion.value}-${index}`}
              type="button"
              className="suggestion-item"
              onClick={() => handleSuggestionClick(suggestion)}
            >
              <span className="suggestion-icon">{getSuggestionIcon(suggestion.type)}</span>
              <span className="suggestion-text">
                <span className="suggestion-label">{suggestion.label}</span>
                {suggestion.count !== undefined && (
                  <span className="suggestion-count">
                    {t('listingsCount', { count: suggestion.count })}
                  </span>
                )}
              </span>
            </button>
          ))}
        </div>
      )}

      <style jsx>{`
        .search-container {
          position: relative;
        }

        .search-form {
          display: flex;
          align-items: center;
          gap: 12px;
          background: var(--ppt-bg-surface);
          border: 1px solid var(--ppt-border-default);
          border-radius: 12px;
          padding: 4px 4px 4px 16px;
        }

        .search-icon {
          color: var(--ppt-fg-subtle);
          flex-shrink: 0;
        }

        .search-input {
          flex: 1;
          border: none;
          background: transparent;
          font-size: 16px;
          padding: 12px 0;
          outline: none;
          min-width: 0;
        }

        .search-input::placeholder {
          color: var(--ppt-fg-subtle);
        }

        .clear-button {
          padding: 4px;
          background: transparent;
          border: none;
          cursor: pointer;
          color: var(--ppt-fg-subtle);
          display: flex;
          align-items: center;
          justify-content: center;
        }

        .clear-button:hover {
          color: var(--ppt-fg-muted);
        }

        .search-button {
          padding: 12px 20px;
          background: var(--ppt-color-primary);
          color: var(--ppt-fg-on-accent);
          border: none;
          border-radius: 8px;
          font-size: 14px;
          font-weight: 600;
          cursor: pointer;
          transition: background 0.2s;
        }

        .search-button:hover {
          background: var(--ppt-color-primary-hover);
        }

        .suggestions {
          position: absolute;
          top: calc(100% + 8px);
          left: 0;
          right: 0;
          background: var(--ppt-bg-surface);
          border: 1px solid var(--ppt-border-default);
          border-radius: 12px;
          box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
          z-index: 50;
          overflow: hidden;
        }

        .suggestion-item {
          display: flex;
          align-items: center;
          gap: 12px;
          width: 100%;
          padding: 12px 16px;
          border: none;
          background: transparent;
          cursor: pointer;
          text-align: left;
        }

        .suggestion-item:hover {
          background: var(--ppt-bg-app);
        }

        .suggestion-icon {
          color: var(--ppt-fg-muted);
          flex-shrink: 0;
        }

        .suggestion-text {
          display: flex;
          flex-direction: column;
          min-width: 0;
        }

        .suggestion-label {
          font-size: 14px;
          color: var(--ppt-fg-primary);
        }

        .suggestion-count {
          font-size: 12px;
          color: var(--ppt-fg-muted);
        }
      `}</style>
    </div>
  );
}
