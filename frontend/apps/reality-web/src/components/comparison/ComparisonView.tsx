/**
 * Property comparison view component.
 *
 * Epic 51 - Story 51.2: Comparison View
 */

'use client';

import type { ListingSummary } from '@ppt/reality-api-client';
import Link from 'next/link';
import { useLocale, useTranslations } from 'next-intl';
import { useMemo, useState } from 'react';

import { useComparison } from '../../lib/comparison-context';

interface ComparisonRow {
  /** Stable, translation-independent identifier (used for keys + highlight logic). */
  id: string;
  label: string;
  getValue: (listing: ListingSummary) => string | number | undefined;
  format?: (value: string | number | undefined, listing?: ListingSummary) => string;
  highlight?: 'lowest' | 'highest' | 'none';
}

export function ComparisonView() {
  const { listings, removeFromComparison, clearComparison, generateShareUrl, shareUrl } =
    useComparison();
  const t = useTranslations('comparison');
  const locale = useLocale();
  const [toast, setToast] = useState<{ message: string; type: 'success' | 'info' } | null>(null);

  // Format price with the listing's actual currency, in the active locale.
  const formatPrice = useMemo(
    () => (value: string | number | undefined, listing?: ListingSummary) => {
      if (value === undefined) return '-';
      const currency = listing?.currency ?? 'EUR';
      return new Intl.NumberFormat(locale, {
        style: 'currency',
        currency,
        maximumFractionDigits: 0,
      }).format(Number(value));
    },
    [locale]
  );

  const comparisonRows: ComparisonRow[] = useMemo(
    () => [
      {
        id: 'price',
        label: t('rows.price'),
        getValue: (l) => l.price,
        format: formatPrice,
        highlight: 'lowest',
      },
      {
        id: 'type',
        label: t('rows.type'),
        getValue: (l) => l.transactionType,
        format: (v) => {
          if (!v) return '-';
          if (v === 'sale') return t('transactionType.sale');
          if (v === 'rent') return t('transactionType.rent');
          return String(v);
        },
      },
      {
        id: 'propertyType',
        label: t('rows.propertyType'),
        getValue: (l) => l.propertyType,
        format: (v) => (v ? String(v).charAt(0).toUpperCase() + String(v).slice(1) : '-'),
      },
      {
        id: 'area',
        label: t('rows.area'),
        getValue: (l) => l.area,
        format: (v) => (v !== undefined ? `${v} m²` : '-'),
        highlight: 'highest',
      },
      {
        id: 'rooms',
        label: t('rows.rooms'),
        getValue: (l) => l.rooms,
        format: (v) => (v !== undefined ? String(v) : '-'),
        highlight: 'highest',
      },
      {
        id: 'floor',
        label: t('rows.floor'),
        getValue: (l) => l.floor,
        format: (v) => (v !== undefined ? String(v) : '-'),
      },
      {
        id: 'city',
        label: t('rows.city'),
        getValue: (l) => l.address?.city,
        format: (v) => (v ? String(v) : '-'),
      },
      {
        id: 'district',
        label: t('rows.district'),
        getValue: (l) => l.address?.district,
        format: (v) => (v ? String(v) : '-'),
      },
      {
        id: 'pricePerSqm',
        label: t('rows.pricePerSqm'),
        getValue: (l) => (l.area > 0 ? Math.round(l.price / l.area) : undefined),
        format: formatPrice,
        highlight: 'lowest',
      },
    ],
    [t, formatPrice]
  );

  const showToast = (message: string, type: 'success' | 'info' = 'success') => {
    setToast({ message, type });
    setTimeout(() => setToast(null), 3000);
  };

  if (listings.length === 0) {
    return (
      <div className="empty-state">
        <div className="empty-icon">📊</div>
        <h2>{t('empty.title')}</h2>
        <p>{t('empty.description')}</p>
        <Link href="/listings" className="browse-btn">
          {t('empty.browse')}
        </Link>
        <style jsx>{`
          .empty-state {
            text-align: center;
            padding: 80px 24px;
          }
          .empty-icon {
            font-size: 64px;
            margin-bottom: 24px;
          }
          h2 {
            font-size: 24px;
            color: var(--ppt-fg-primary);
            margin: 0 0 8px;
          }
          p {
            color: var(--ppt-fg-muted);
            margin: 0 0 24px;
          }
          .browse-btn {
            display: inline-block;
            background: var(--ppt-color-primary);
            color: var(--ppt-fg-on-accent);
            padding: 12px 24px;
            border-radius: 8px;
            text-decoration: none;
            font-weight: 500;
          }
        `}</style>
      </div>
    );
  }

  const handleShare = async () => {
    const url = generateShareUrl();
    try {
      await navigator.clipboard.writeText(url);
      showToast(t('toast.linkCopied'), 'success');
    } catch {
      showToast(t('toast.shareUrl', { url }), 'info');
    }
  };

  const handleExportPDF = () => {
    // In a real implementation, this would generate a PDF
    showToast(t('toast.pdfComingSoon'), 'info');
  };

  // Check if all listings use the same currency
  const currenciesMatch = () => {
    if (listings.length < 2) return true;
    const firstCurrency = listings[0]?.currency ?? 'EUR';
    return listings.every((l: ListingSummary) => (l.currency ?? 'EUR') === firstCurrency);
  };

  const getHighlightClass = (row: ComparisonRow, listing: ListingSummary) => {
    if (row.highlight === 'none' || !row.highlight) return '';

    // Disable price highlighting when currencies differ
    if ((row.id === 'price' || row.id === 'pricePerSqm') && !currenciesMatch()) {
      return '';
    }

    const values = listings
      .map((l: ListingSummary) => row.getValue(l))
      .filter((v: string | number | undefined) => v !== undefined) as number[];
    if (values.length < 2) return '';

    const currentValue = row.getValue(listing);
    if (currentValue === undefined) return '';

    // Note: When multiple properties share the same min/max value, all will be highlighted.
    // This is intentional - if properties are tied, they're all equally "best" for that metric.
    if (row.highlight === 'lowest' && currentValue === Math.min(...values)) {
      return 'highlight-best';
    }
    if (row.highlight === 'highest' && currentValue === Math.max(...values)) {
      return 'highlight-best';
    }

    return '';
  };

  return (
    <div className="comparison-view">
      <div className="actions">
        <button type="button" className="action-btn" onClick={handleShare}>
          <svg
            width="18"
            height="18"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            aria-hidden="true"
          >
            <circle cx="18" cy="5" r="3" />
            <circle cx="6" cy="12" r="3" />
            <circle cx="18" cy="19" r="3" />
            <line x1="8.59" y1="13.51" x2="15.42" y2="17.49" />
            <line x1="15.41" y1="6.51" x2="8.59" y2="10.49" />
          </svg>
          {t('actions.share')}
        </button>
        <button type="button" className="action-btn" onClick={handleExportPDF}>
          <svg
            width="18"
            height="18"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            aria-hidden="true"
          >
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
            <polyline points="14 2 14 8 20 8" />
            <line x1="12" y1="18" x2="12" y2="12" />
            <line x1="9" y1="15" x2="15" y2="15" />
          </svg>
          {t('actions.exportPdf')}
        </button>
        <button type="button" className="action-btn danger" onClick={clearComparison}>
          {t('actions.clearAll')}
        </button>
      </div>

      {shareUrl && (
        <div className="share-url">
          <input type="text" value={shareUrl} readOnly />
        </div>
      )}

      {!currenciesMatch() && (
        <div className="currency-warning" role="alert">
          <p>{t('currencyWarning')}</p>
        </div>
      )}

      <div className="comparison-table">
        <table>
          <thead>
            <tr>
              <th className="label-column">{t('propertyHeader')}</th>
              {listings.map((listing: ListingSummary) => (
                <th key={listing.id} className="property-column">
                  <div className="property-header">
                    <Link href={`/listings/${listing.slug}`} className="property-link">
                      <div className="property-image">
                        {listing.primaryPhoto ? (
                          <img src={listing.primaryPhoto.thumbnailUrl} alt={listing.title} />
                        ) : (
                          <div className="no-image">🏠</div>
                        )}
                      </div>
                      <h3 className="property-title">{listing.title}</h3>
                    </Link>
                    <button
                      type="button"
                      className="remove-btn"
                      onClick={() => removeFromComparison(listing.id)}
                      aria-label={t('removeAria', { title: listing.title })}
                    >
                      {t('remove')}
                    </button>
                  </div>
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {comparisonRows.map((row) => (
              <tr key={row.id}>
                <td className="label-cell">{row.label}</td>
                {listings.map((listing: ListingSummary) => {
                  const value = row.getValue(listing);
                  const formatted = row.format ? row.format(value, listing) : (value ?? '-');
                  const highlightClass = getHighlightClass(row, listing);

                  return (
                    <td key={listing.id} className={`value-cell ${highlightClass}`}>
                      {formatted}
                    </td>
                  );
                })}
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {toast && (
        <output className={`toast toast-${toast.type}`} aria-live="polite">
          {toast.message}
        </output>
      )}

      <style jsx>{`
        .comparison-view {
          padding: 24px;
        }

        .actions {
          display: flex;
          gap: 12px;
          margin-bottom: 24px;
          flex-wrap: wrap;
        }

        .action-btn {
          display: flex;
          align-items: center;
          gap: 8px;
          padding: 10px 16px;
          border: 1px solid var(--ppt-border-strong);
          border-radius: 8px;
          background: var(--ppt-bg-surface);
          color: var(--ppt-fg-secondary);
          font-size: 14px;
          font-weight: 500;
          cursor: pointer;
          transition: all 0.2s;
        }

        .action-btn:hover {
          border-color: var(--ppt-fg-subtle);
          background: var(--ppt-bg-app);
        }

        .action-btn.danger {
          color: var(--ppt-color-danger);
        }

        .action-btn.danger:hover {
          background: var(--ppt-color-danger-light);
          border-color: var(--ppt-color-danger-light);
        }

        .share-url {
          margin-bottom: 24px;
        }

        .share-url input {
          width: 100%;
          padding: 12px;
          border: 1px solid var(--ppt-border-strong);
          border-radius: 8px;
          font-size: 14px;
          background: var(--ppt-bg-app);
        }

        .currency-warning {
          background: var(--ppt-color-warning-light);
          border: 1px solid var(--ppt-color-warning);
          border-radius: 8px;
          padding: 12px 16px;
          margin-bottom: 24px;
          color: var(--ppt-color-warning-dark);
        }

        .currency-warning p {
          margin: 0;
          font-size: 14px;
        }

        .comparison-table {
          overflow-x: auto;
        }

        table {
          width: 100%;
          border-collapse: collapse;
          min-width: 600px;
        }

        th,
        td {
          padding: 16px;
          border-bottom: 1px solid var(--ppt-border-default);
          text-align: left;
        }

        .label-column {
          width: 150px;
          min-width: 150px;
        }

        .property-column {
          min-width: 200px;
        }

        .property-header {
          display: flex;
          flex-direction: column;
          gap: 12px;
        }

        .property-link {
          text-decoration: none;
        }

        .property-image {
          width: 100%;
          height: 120px;
          border-radius: 8px;
          overflow: hidden;
          background: var(--ppt-bg-subtle);
        }

        .property-image img {
          width: 100%;
          height: 100%;
          object-fit: cover;
        }

        .no-image {
          width: 100%;
          height: 100%;
          display: flex;
          align-items: center;
          justify-content: center;
          font-size: 32px;
          background: var(--ppt-border-default);
        }

        .property-title {
          font-size: 14px;
          font-weight: 600;
          color: var(--ppt-fg-primary);
          margin: 8px 0 0;
          display: -webkit-box;
          -webkit-line-clamp: 2;
          -webkit-box-orient: vertical;
          overflow: hidden;
        }

        .property-link:hover .property-title {
          color: var(--ppt-color-primary);
        }

        .remove-btn {
          padding: 6px 12px;
          border: 1px solid var(--ppt-color-danger-light);
          border-radius: 6px;
          background: var(--ppt-color-danger-light);
          color: var(--ppt-color-danger);
          font-size: 13px;
          cursor: pointer;
          transition: all 0.2s;
        }

        .remove-btn:hover {
          background: var(--ppt-color-danger-light);
        }

        .label-cell {
          font-weight: 500;
          color: var(--ppt-fg-muted);
          background: var(--ppt-bg-app);
        }

        .value-cell {
          font-weight: 500;
          color: var(--ppt-fg-primary);
        }

        .highlight-best {
          background: var(--ppt-color-success-light);
          color: var(--ppt-color-success-hover);
        }

        .toast {
          position: fixed;
          bottom: 24px;
          left: 50%;
          transform: translateX(-50%);
          padding: 12px 24px;
          border-radius: 8px;
          font-size: 14px;
          font-weight: 500;
          z-index: 1000;
          animation: toast-fade-in 0.2s ease;
        }

        .toast-success {
          background: var(--ppt-color-success-hover);
          color: var(--ppt-fg-on-accent);
        }

        .toast-info {
          background: var(--ppt-fg-secondary);
          color: var(--ppt-fg-on-accent);
        }

        @keyframes toast-fade-in {
          from {
            opacity: 0;
            transform: translateX(-50%) translateY(10px);
          }
          to {
            opacity: 1;
            transform: translateX(-50%) translateY(0);
          }
        }

        @media (max-width: 768px) {
          .comparison-view {
            padding: 16px;
          }

          th,
          td {
            padding: 12px;
          }
        }
      `}</style>
    </div>
  );
}
