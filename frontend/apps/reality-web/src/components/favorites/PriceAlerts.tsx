/**
 * PriceAlerts
 *
 * Dedicated price-alert subscription surface for the Reality Portal
 * (Story 84.3 — Price Tracking for Favorites). Consumes the existing
 * reality-server price-tracking feed at `GET /api/v1/favorites/alerts`
 * (populated by the `FavoriteAlertWorker`) and lets the user read /
 * dismiss price-change and back-on-market alerts on favorited listings.
 */

'use client';

import {
  type FavoriteAlert,
  useFavoriteAlerts,
  useMarkAllFavoriteAlertsRead,
  useMarkFavoriteAlertRead,
} from '@ppt/reality-api-client';
import Link from 'next/link';
import { useLocale, useTranslations } from 'next-intl';
import { formatPrice } from '@/lib/format';

const UNREAD_STATUS = 'pending';

function isUnread(alert: FavoriteAlert): boolean {
  return alert.status === UNREAD_STATUS;
}

/** Price alert direction — drop is the actionable case buyers care about. */
function priceDirection(alert: FavoriteAlert): 'drop' | 'increase' | null {
  if (alert.old_price == null || alert.new_price == null) return null;
  if (alert.new_price < alert.old_price) return 'drop';
  if (alert.new_price > alert.old_price) return 'increase';
  return null;
}

function AlertBody({ alert, locale }: { alert: FavoriteAlert; locale: string }) {
  const t = useTranslations('priceAlerts');
  const currency = alert.currency ?? 'EUR';

  if (alert.alert_type === 'back_on_market') {
    return <p className="alert-detail">{t('backOnMarket')}</p>;
  }

  const direction = priceDirection(alert);
  const pct = alert.change_percentage;
  const pctLabel =
    pct != null
      ? `${pct > 0 ? '+' : ''}${new Intl.NumberFormat(locale, {
          maximumFractionDigits: 1,
        }).format(pct)}%`
      : null;

  return (
    <p className="alert-detail">
      {alert.old_price != null && alert.new_price != null ? (
        <>
          <span className="old-price">{formatPrice(alert.old_price, locale, { currency })}</span>
          <span aria-hidden="true" className="arrow">
            →
          </span>
          <span className={`new-price ${direction ?? ''}`}>
            {formatPrice(alert.new_price, locale, { currency })}
          </span>
        </>
      ) : (
        <span>{t('priceChanged')}</span>
      )}
      {pctLabel && direction && (
        <span className={`pct ${direction}`}>
          {direction === 'drop' ? t('drop', { pct: pctLabel }) : t('increase', { pct: pctLabel })}
        </span>
      )}
    </p>
  );
}

export function PriceAlerts() {
  const t = useTranslations('priceAlerts');
  const locale = useLocale();
  const { data, isLoading, error } = useFavoriteAlerts();
  const markRead = useMarkFavoriteAlertRead();
  const markAllRead = useMarkAllFavoriteAlertsRead();

  if (isLoading) {
    return (
      <div className="alerts-list loading">
        {[1, 2, 3].map((i) => (
          <div key={`alert-skeleton-${i}`} className="skeleton-row" />
        ))}
        <style jsx>{`
          .alerts-list { display: flex; flex-direction: column; gap: 12px; }
          .skeleton-row {
            height: 84px;
            background: var(--ppt-border-default);
            border-radius: 12px;
            animation: pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
          }
          @keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.5; } }
        `}</style>
      </div>
    );
  }

  if (error) {
    return (
      <div className="error-state" role="alert">
        <p>{t('loadError')}</p>
        <style jsx>{`
          .error-state {
            padding: 48px 24px;
            text-align: center;
            color: var(--ppt-color-danger-hover);
          }
        `}</style>
      </div>
    );
  }

  const alerts = data?.alerts ?? [];
  const unreadCount = data?.unread_count ?? 0;

  if (alerts.length === 0) {
    return (
      <div className="empty-state">
        <h2 className="empty-title">{t('emptyTitle')}</h2>
        <p className="empty-text">{t('emptyText')}</p>
        <Link href="/favorites" className="browse-link">
          {t('goToFavorites')}
        </Link>
        <style jsx>{`
          .empty-state {
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            padding: 64px 24px;
            text-align: center;
            color: var(--ppt-fg-muted);
          }
          .empty-title {
            font-size: 1.5rem;
            font-weight: 600;
            color: var(--ppt-fg-primary);
            margin: 0 0 8px;
          }
          .empty-text { margin: 0 0 24px; max-width: 400px; }
          .browse-link {
            padding: 12px 24px;
            background: var(--ppt-color-primary);
            color: var(--ppt-fg-on-accent);
            text-decoration: none;
            border-radius: 8px;
            font-weight: 600;
          }
          .browse-link:hover { background: var(--ppt-color-primary-hover); }
        `}</style>
      </div>
    );
  }

  return (
    <>
      <div className="toolbar">
        <span className="unread-badge">{t('unread', { count: unreadCount })}</span>
        <button
          type="button"
          className="mark-all"
          disabled={unreadCount === 0 || markAllRead.isPending}
          onClick={() => markAllRead.mutate()}
        >
          {t('markAllRead')}
        </button>
      </div>

      <ul className="alerts-list">
        {alerts.map((alert) => (
          <li key={alert.id} className={`alert-row ${isUnread(alert) ? 'is-unread' : ''}`}>
            <div className="alert-main">
              <Link href={`/listings/${alert.listing_id}`} className="alert-title">
                {alert.title}
              </Link>
              <AlertBody alert={alert} locale={locale} />
            </div>
            {isUnread(alert) && (
              <button
                type="button"
                className="mark-read"
                disabled={markRead.isPending}
                onClick={() => markRead.mutate(alert.id)}
              >
                {t('markRead')}
              </button>
            )}
          </li>
        ))}
      </ul>

      <style jsx>{`
        .toolbar {
          display: flex;
          align-items: center;
          justify-content: space-between;
          gap: 16px;
          margin-bottom: 20px;
        }
        .unread-badge {
          font-size: 14px;
          font-weight: 600;
          color: var(--ppt-fg-muted);
        }
        .mark-all {
          padding: 8px 16px;
          background: var(--ppt-bg-surface);
          border: 1px solid var(--ppt-border-default);
          border-radius: 8px;
          font-size: 14px;
          cursor: pointer;
        }
        .mark-all:disabled { opacity: 0.5; cursor: not-allowed; }
        .mark-all:hover:not(:disabled) { background: var(--ppt-bg-app); }
        .alerts-list {
          list-style: none;
          margin: 0;
          padding: 0;
          display: flex;
          flex-direction: column;
          gap: 12px;
        }
        .alert-row {
          display: flex;
          align-items: center;
          justify-content: space-between;
          gap: 16px;
          padding: 16px;
          background: var(--ppt-bg-surface);
          border: 1px solid var(--ppt-border-default);
          border-radius: 12px;
        }
        .alert-row.is-unread {
          border-left: 3px solid var(--ppt-color-primary);
        }
        .alert-main { min-width: 0; }
        .alert-title {
          font-weight: 600;
          color: var(--ppt-fg-primary);
          text-decoration: none;
        }
        .alert-title:hover { text-decoration: underline; }
        .mark-read {
          flex-shrink: 0;
          padding: 6px 12px;
          background: transparent;
          border: 1px solid var(--ppt-border-default);
          border-radius: 8px;
          font-size: 13px;
          cursor: pointer;
          color: var(--ppt-fg-muted);
        }
        .mark-read:disabled { opacity: 0.5; cursor: not-allowed; }
        .mark-read:hover:not(:disabled) { background: var(--ppt-bg-app); }
      `}</style>
      <style jsx global>{`
        .alert-detail {
          margin: 4px 0 0;
          display: flex;
          flex-wrap: wrap;
          align-items: baseline;
          gap: 8px;
          font-size: 14px;
          color: var(--ppt-fg-muted);
        }
        .alert-detail .old-price { text-decoration: line-through; }
        .alert-detail .arrow { color: var(--ppt-fg-muted); }
        .alert-detail .new-price { font-weight: 600; color: var(--ppt-fg-primary); }
        .alert-detail .pct.drop,
        .alert-detail .new-price.drop { color: var(--ppt-color-success, #16a34a); }
        .alert-detail .pct.increase,
        .alert-detail .new-price.increase { color: var(--ppt-color-danger-hover); }
      `}</style>
    </>
  );
}
