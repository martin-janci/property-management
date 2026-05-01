/**
 * Main action queue component showing prioritized items needing attention.
 * Aggregates items from multiple sources and provides inline actions.
 *
 * @module features/dashboard/components/ActionQueue
 */

import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate, useSearchParams } from 'react-router-dom';
import type {
  ActionButton,
  ActionItem as ActionItemType,
  ActionQueueFilters,
} from '../hooks/useActionQueue';
import { useActionQueue } from '../hooks/useActionQueue';
import { ActionFilters } from './ActionFilters';
import { ActionItem } from './ActionItem';
import { KeyboardShortcutsHelp } from './KeyboardShortcutsHelp';
import './ActionQueue.css';

interface ActionQueueProps {
  userRole: 'manager' | 'resident';
  onItemAction?: (itemId: string, action: ActionButton['action'], item: ActionItemType) => void;
}

export function ActionQueue({ userRole, onItemAction }: ActionQueueProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();
  const [filters, setFilters] = useState<ActionQueueFilters>({});
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [showFilters, setShowFilters] = useState(false);
  const [showShortcutsHelp, setShowShortcutsHelp] = useState(false);
  const [highlightedItemId, setHighlightedItemId] = useState<string | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const itemRefs = useRef<Map<string, HTMLDivElement>>(new Map());

  const { items, stats, isLoading, isError, error, handleAction, isExecuting, refetch } =
    useActionQueue(userRole, filters);

  // Handle deep link from notification - select and highlight the specified item
  useEffect(() => {
    const itemId = searchParams.get('itemId');
    if (itemId && items.length > 0) {
      const index = items.findIndex((item) => item.id === itemId);
      if (index !== -1) {
        setSelectedIndex(index);
        setHighlightedItemId(itemId);

        // Scroll the item into view
        const itemElement = itemRefs.current.get(itemId);
        if (itemElement) {
          itemElement.scrollIntoView({ behavior: 'smooth', block: 'center' });
          itemElement.focus();
        }

        // Clear the query param after handling (prevents re-highlighting on refetch)
        const newParams = new URLSearchParams(searchParams);
        newParams.delete('itemId');
        setSearchParams(newParams, { replace: true });

        // Clear highlight after animation
        setTimeout(() => {
          setHighlightedItemId(null);
        }, 2000);
      }
    }
  }, [searchParams, items, setSearchParams]);

  // Handle action execution with navigation
  const handleItemAction = useCallback(
    (itemId: string, action: ActionButton['action']) => {
      const item = items.find((i) => i.id === itemId);
      if (!item) return;

      // Call external handler if provided
      if (onItemAction) {
        onItemAction(itemId, action, item);
      }

      // Navigate to detail pages for 'view' action
      if (action === 'view') {
        const routeMap: Record<string, string> = {
          fault: `/faults/${item.entityId}`,
          budget_approval: `/approvals/${item.entityId}`,
          vote: `/votes/${item.entityId}`,
          message: `/messages/${item.entityId}`,
          meter_reading: '/meters/reading',
          person_months: '/person-months',
          announcement: `/announcements/${item.entityId}`,
        };
        const route = routeMap[item.entityType];
        if (route) {
          navigate(route);
          return;
        }
      }

      // Execute inline action
      handleAction(itemId, action);
    },
    [items, handleAction, navigate, onItemAction]
  );

  // Keyboard navigation
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Only handle if queue container is focused or contains focus
      if (!containerRef.current?.contains(document.activeElement)) {
        return;
      }

      switch (e.key) {
        case 'j':
        case 'ArrowDown':
          e.preventDefault();
          setSelectedIndex((prev) => Math.min(prev + 1, items.length - 1));
          break;
        case 'k':
        case 'ArrowUp':
          e.preventDefault();
          setSelectedIndex((prev) => Math.max(prev - 1, 0));
          break;
        case 'Enter':
          e.preventDefault();
          if (items[selectedIndex]) {
            handleItemAction(items[selectedIndex].id, 'view');
          }
          break;
        case 'a':
          e.preventDefault();
          if (items[selectedIndex]?.actions.some((a) => a.action === 'approve')) {
            handleItemAction(items[selectedIndex].id, 'approve');
          }
          break;
        case 'r':
          e.preventDefault();
          if (items[selectedIndex]?.actions.some((a) => a.action === 'reject')) {
            handleItemAction(items[selectedIndex].id, 'reject');
          }
          break;
        case 'Escape':
          e.preventDefault();
          setShowFilters(false);
          break;
        case '?':
          e.preventDefault();
          setShowShortcutsHelp(true);
          break;
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [items, selectedIndex, handleItemAction]);

  // Reset selected index when items change
  useEffect(() => {
    if (selectedIndex >= items.length) {
      setSelectedIndex(Math.max(0, items.length - 1));
    }
  }, [items.length, selectedIndex]);

  if (isError) {
    return (
      <div className="action-queue__error">
        <h3 className="action-queue__error-title">{t('dashboard.errorLoadingQueue')}</h3>
        <p className="action-queue__error-message">
          {error instanceof Error ? error.message : t('errors.unknown')}
        </p>
        <button type="button" onClick={() => refetch()} className="action-queue__retry-btn">
          {t('common.tryAgain')}
        </button>
      </div>
    );
  }

  return (
    <div ref={containerRef} className="action-queue" tabIndex={-1}>
      <div className="action-queue__header">
        <div>
          <h2 className="action-queue__heading">{t(`dashboard.${userRole}ActionQueue`)}</h2>
          <p className="action-queue__subheading">
            {t('dashboard.itemsNeedingAttention', { count: stats.total })}
          </p>
        </div>

        <div className="action-queue__controls">
          <div className="action-queue__priority-stats">
            {stats.urgent > 0 && (
              <span className="action-queue__stat-badge action-queue__stat-badge--urgent">
                {stats.urgent} {t('dashboard.priority.urgent')}
              </span>
            )}
            {stats.high > 0 && (
              <span className="action-queue__stat-badge action-queue__stat-badge--high">
                {stats.high} {t('dashboard.priority.high')}
              </span>
            )}
          </div>

          <button
            type="button"
            onClick={() => setShowFilters(!showFilters)}
            className={`action-queue__btn${showFilters ? ' action-queue__btn--active' : ''}`}
            aria-expanded={showFilters}
          >
            <span aria-hidden="true">🔍</span> {t('dashboard.filters')}
          </button>

          <button
            type="button"
            onClick={() => refetch()}
            disabled={isLoading}
            className="action-queue__btn"
            aria-label={t('common.refresh')}
          >
            <span className={isLoading ? 'action-queue__spin' : ''} aria-hidden="true">🔄</span>
          </button>
        </div>
      </div>

      {showFilters && (
        <ActionFilters
          filters={filters}
          onFilterChange={setFilters}
          stats={stats}
          userRole={userRole}
        />
      )}

      {isLoading && items.length === 0 && (
        <div className="action-queue__spinner">
          <div className="action-queue__spinner-ring" aria-label={t('common.loading')} />
        </div>
      )}

      {!isLoading && items.length === 0 && (
        <div className="action-queue__empty">
          <span className="action-queue__empty-icon" aria-hidden="true">✅</span>
          <h3 className="action-queue__empty-title">{t('dashboard.allCaughtUp')}</h3>
          <p className="action-queue__empty-subtitle">{t('dashboard.noItemsNeedingAttention')}</p>
        </div>
      )}

      {items.length > 0 && (
        <div className="action-queue__list" role="list" aria-label={t('dashboard.actionQueue')}>
          {items.map((item, index) => (
            <ActionItem
              key={item.id}
              ref={(el) => {
                if (el) {
                  itemRefs.current.set(item.id, el);
                } else {
                  itemRefs.current.delete(item.id);
                }
              }}
              item={item}
              isSelected={index === selectedIndex}
              isHighlighted={item.id === highlightedItemId}
              isExecuting={isExecuting}
              onAction={handleItemAction}
              onSelect={() => setSelectedIndex(index)}
            />
          ))}
        </div>
      )}

      <p className="action-queue__kbd-hint">
        {t('dashboard.keyboardHint')}{' '}
        <kbd className="action-queue__kbd">j</kbd>/<kbd className="action-queue__kbd">k</kbd>{' '}
        {t('dashboard.toNavigate')},{' '}
        <kbd className="action-queue__kbd">Enter</kbd>{' '}
        {t('dashboard.toOpen')},{' '}
        <kbd className="action-queue__kbd">?</kbd>{' '}
        {t('dashboard.forHelp')}
      </p>

      <KeyboardShortcutsHelp
        isOpen={showShortcutsHelp}
        onClose={() => setShowShortcutsHelp(false)}
      />
    </div>
  );
}
