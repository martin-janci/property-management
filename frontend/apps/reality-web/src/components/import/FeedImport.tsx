/**
 * FeedImport Component
 *
 * Import from XML/RSS feeds (Epic 46, Story 46.4).
 */

'use client';

import type {
  FeedFieldMapping,
  FeedFormat,
  FeedPreview,
  FeedSource,
  SyncFrequency,
} from '@ppt/reality-api-client';
import {
  useCreateFeedSource,
  useDeleteFeedSource,
  useFeedPreview,
  useFeedSources,
  useFeedSyncHistory,
  useMyAgency,
  useSyncFeedSource,
  useUpdateFeedSource,
} from '@ppt/reality-api-client';
import { useTranslations } from 'next-intl';
import { useState } from 'react';

const DEFAULT_FIELD_MAPPING: FeedFieldMapping = {
  title: 'title',
  description: 'description',
  price: 'price',
  propertyType: 'type',
  address: 'address',
  city: 'city',
  rooms: 'bedrooms',
  size: 'area',
  photos: 'images',
};

const FREQUENCY_VALUES: SyncFrequency[] = ['manual', 'hourly', 'daily', 'weekly'];

export function FeedImport() {
  const t = useTranslations('import.feed');
  const [showModal, setShowModal] = useState(false);

  const { data: agency } = useMyAgency();
  const { data: feeds, isLoading } = useFeedSources(agency?.id || '');

  return (
    <div className="feed-import">
      <div className="header">
        <div>
          <h2>{t('title')}</h2>
          <p className="subtitle">{t('subtitle')}</p>
        </div>
        <button type="button" className="add-button" onClick={() => setShowModal(true)}>
          <svg
            width="20"
            height="20"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            aria-hidden="true"
          >
            <line x1="12" y1="5" x2="12" y2="19" />
            <line x1="5" y1="12" x2="19" y2="12" />
          </svg>
          {t('addFeed')}
        </button>
      </div>

      {isLoading ? (
        <FeedsSkeleton />
      ) : feeds?.length === 0 ? (
        <EmptyState onAdd={() => setShowModal(true)} />
      ) : (
        <div className="feeds-list">
          {feeds?.map((feed) => (
            <FeedCard key={feed.id} feed={feed} agencyId={agency?.id || ''} />
          ))}
        </div>
      )}

      {showModal && (
        <AddFeedModal agencyId={agency?.id || ''} onClose={() => setShowModal(false)} />
      )}

      <style jsx>{`
        .feed-import {
          padding: 24px;
        }

        .header {
          display: flex;
          justify-content: space-between;
          align-items: flex-start;
          margin-bottom: 24px;
          flex-wrap: wrap;
          gap: 16px;
        }

        h2 {
          font-size: 1.5rem;
          color: var(--ppt-fg-primary);
          margin: 0 0 4px;
        }

        .subtitle {
          color: var(--ppt-fg-muted);
          margin: 0;
        }

        .add-button {
          display: flex;
          align-items: center;
          gap: 8px;
          padding: 12px 20px;
          background: var(--ppt-color-primary);
          color: var(--ppt-fg-on-accent);
          border: none;
          border-radius: 8px;
          font-size: 14px;
          font-weight: 500;
          cursor: pointer;
        }

        .add-button:hover {
          background: var(--ppt-color-primary-hover);
        }

        .feeds-list {
          display: flex;
          flex-direction: column;
          gap: 16px;
        }
      `}</style>
    </div>
  );
}

function FeedCard({ feed, agencyId }: { feed: FeedSource; agencyId: string }) {
  const t = useTranslations('import.feed');
  const tHist = useTranslations('import.schedule.historyStatus');
  const [showHistory, setShowHistory] = useState(false);
  const deleteMutation = useDeleteFeedSource(agencyId);
  const syncMutation = useSyncFeedSource(agencyId, feed.id);
  const updateMutation = useUpdateFeedSource(agencyId, feed.id);
  const { data: history } = useFeedSyncHistory(agencyId, feed.id, showHistory ? 5 : 0);

  const statusConfig = getFeedStatusConfig(feed.status);

  const handleTogglePause = async () => {
    await updateMutation.mutateAsync({
      status: feed.status === 'active' ? 'paused' : 'active',
    });
  };

  const handleDelete = async () => {
    if (confirm(t('confirmRemove'))) {
      await deleteMutation.mutateAsync(feed.id);
    }
  };

  return (
    <div className="feed-card">
      <div className="card-header">
        <div className="feed-info">
          <div className="format-badge">{getFormatLabel(feed.format)}</div>
          <div>
            <h3>{feed.name}</h3>
            <p className="feed-url">{feed.url}</p>
          </div>
        </div>
        <span
          className="status-badge"
          style={{ background: statusConfig.bg, color: statusConfig.color }}
        >
          {t(`status.${feed.status}`)}
        </span>
      </div>

      <div className="card-body">
        <div className="stats-row">
          <div className="stat">
            <span className="stat-value">{feed.totalListings}</span>
            <span className="stat-label">{t('listings')}</span>
          </div>
          <div className="stat">
            <span className="stat-value">{t(`frequency.${feed.syncFrequency}`)}</span>
            <span className="stat-label">{t('syncFrequency')}</span>
          </div>
          {feed.lastFetchAt && (
            <div className="stat">
              <span className="stat-value">{new Date(feed.lastFetchAt).toLocaleDateString()}</span>
              <span className="stat-label">{t('lastSync')}</span>
            </div>
          )}
        </div>
      </div>

      <div className="card-actions">
        <button
          type="button"
          className="action-button sync"
          onClick={() => syncMutation.mutate()}
          disabled={syncMutation.isPending || feed.status === 'error'}
        >
          {syncMutation.isPending ? t('syncing') : t('syncNow')}
        </button>
        <button
          type="button"
          className="action-button pause"
          onClick={handleTogglePause}
          disabled={updateMutation.isPending}
        >
          {feed.status === 'active' ? t('pause') : t('resume')}
        </button>
        <button
          type="button"
          className="action-button history"
          onClick={() => setShowHistory(!showHistory)}
        >
          {t('history')}
        </button>
        <button
          type="button"
          className="action-button delete"
          onClick={handleDelete}
          disabled={deleteMutation.isPending}
        >
          {t('remove')}
        </button>
      </div>

      {showHistory && history && (
        <div className="history-section">
          <h4>{t('syncHistory')}</h4>
          {history.length === 0 ? (
            <p className="no-history">{t('noHistory')}</p>
          ) : (
            <div className="history-list">
              {history.map((item) => (
                <div key={item.id} className="history-item">
                  <span className="history-date">{new Date(item.startedAt).toLocaleString()}</span>
                  <span className={`history-status ${item.status}`}>{tHist(item.status)}</span>
                  <span className="history-stats">
                    {t('historyStats', {
                      created: item.recordsCreated,
                      updated: item.recordsUpdated,
                    })}
                  </span>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      <style jsx>{`
        .feed-card {
          background: var(--ppt-bg-surface);
          border: 1px solid var(--ppt-border-default);
          border-radius: 12px;
          overflow: hidden;
        }

        .card-header {
          display: flex;
          justify-content: space-between;
          align-items: flex-start;
          padding: 20px;
          border-bottom: 1px solid var(--ppt-bg-subtle);
        }

        .feed-info {
          display: flex;
          gap: 12px;
        }

        .format-badge {
          padding: 6px 10px;
          background: var(--ppt-border-default);
          border-radius: 6px;
          font-size: 11px;
          font-weight: 600;
          color: var(--ppt-fg-secondary);
          text-transform: uppercase;
        }

        h3 {
          font-size: 1rem;
          color: var(--ppt-fg-primary);
          margin: 0 0 4px;
        }

        .feed-url {
          font-size: 13px;
          color: var(--ppt-fg-muted);
          margin: 0;
          max-width: 400px;
          overflow: hidden;
          text-overflow: ellipsis;
          white-space: nowrap;
        }

        .status-badge {
          padding: 4px 10px;
          border-radius: 12px;
          font-size: 12px;
          font-weight: 500;
        }

        .card-body {
          padding: 16px 20px;
        }

        .stats-row {
          display: flex;
          gap: 32px;
        }

        .stat {
          display: flex;
          flex-direction: column;
        }

        .stat-value {
          font-size: 1.25rem;
          font-weight: 600;
          color: var(--ppt-fg-primary);
        }

        .stat-label {
          font-size: 12px;
          color: var(--ppt-fg-muted);
        }

        .card-actions {
          display: flex;
          gap: 8px;
          padding: 16px 20px;
          background: var(--ppt-bg-app);
          border-top: 1px solid var(--ppt-bg-subtle);
        }

        .action-button {
          padding: 8px 12px;
          border-radius: 6px;
          font-size: 13px;
          font-weight: 500;
          cursor: pointer;
        }

        .action-button.sync {
          background: var(--ppt-color-primary);
          color: var(--ppt-fg-on-accent);
          border: none;
        }

        .action-button.sync:disabled {
          opacity: 0.5;
        }

        .action-button.pause,
        .action-button.history {
          background: var(--ppt-bg-surface);
          border: 1px solid var(--ppt-border-strong);
          color: var(--ppt-fg-secondary);
        }

        .action-button.delete {
          background: var(--ppt-bg-surface);
          border: 1px solid var(--ppt-color-danger-light);
          color: var(--ppt-color-danger);
          margin-left: auto;
        }

        .action-button.delete:hover {
          background: var(--ppt-color-danger-light);
        }

        .history-section {
          padding: 16px 20px;
          border-top: 1px solid var(--ppt-border-default);
          background: var(--ppt-bg-subtle);
        }

        .history-section h4 {
          font-size: 14px;
          color: var(--ppt-fg-secondary);
          margin: 0 0 12px;
        }

        .history-list {
          display: flex;
          flex-direction: column;
          gap: 8px;
        }

        .history-item {
          display: flex;
          align-items: center;
          gap: 12px;
          padding: 10px 12px;
          background: var(--ppt-bg-surface);
          border-radius: 6px;
          font-size: 13px;
        }

        .history-date {
          color: var(--ppt-fg-secondary);
        }

        .history-status {
          padding: 2px 8px;
          border-radius: 4px;
          font-size: 11px;
          font-weight: 500;
        }

        .history-status.completed {
          background: var(--ppt-color-success-light);
          color: var(--ppt-color-success-hover);
        }

        .history-status.failed {
          background: var(--ppt-color-danger-light);
          color: var(--ppt-color-danger);
        }

        .history-status.running {
          background: var(--ppt-color-primary-soft-bg);
          color: var(--ppt-color-primary);
        }

        .history-stats {
          color: var(--ppt-fg-muted);
          margin-left: auto;
        }

        .no-history {
          color: var(--ppt-fg-muted);
          font-size: 13px;
          text-align: center;
          padding: 12px;
          margin: 0;
        }
      `}</style>
    </div>
  );
}

function AddFeedModal({ agencyId, onClose }: { agencyId: string; onClose: () => void }) {
  const t = useTranslations('import.feed');
  const tFreq = useTranslations('import.feed.frequency');
  const [step, setStep] = useState<'url' | 'preview' | 'mapping'>('url');
  const [url, setUrl] = useState('');
  const [name, setName] = useState('');
  const [frequency, setFrequency] = useState<SyncFrequency>('daily');
  const [fieldMapping, setFieldMapping] = useState<FeedFieldMapping>(DEFAULT_FIELD_MAPPING);
  const [previewData, setPreviewData] = useState<FeedPreview | null>(null);

  const previewMutation = useFeedPreview(agencyId);
  const createMutation = useCreateFeedSource(agencyId);

  const handlePreview = async () => {
    try {
      const result = await previewMutation.mutateAsync(url);
      setPreviewData(result);
      if (result.success) {
        setStep('preview');
      }
    } catch {
      // Error handled by mutation
    }
  };

  const handleCreate = async () => {
    await createMutation.mutateAsync({
      name: name || t('defaultName'),
      url,
      format: previewData?.format,
      fieldMapping,
      syncFrequency: frequency,
    });
    onClose();
  };

  return (
    <div
      className="modal-overlay"
      onClick={onClose}
      onKeyDown={(e) => e.key === 'Escape' && onClose()}
      role="dialog"
      aria-modal="true"
      aria-labelledby="modal-title"
    >
      <div className="modal-content" onClick={(e) => e.stopPropagation()} onKeyDown={() => {}}>
        <div className="modal-header">
          <h2 id="modal-title">
            {step === 'url' && t('modalAdd')}
            {step === 'preview' && t('modalPreview')}
            {step === 'mapping' && t('modalMapping')}
          </h2>
          <button
            type="button"
            className="close-button"
            onClick={onClose}
            aria-label={t('closeModal')}
          >
            <svg
              width="24"
              height="24"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              aria-hidden="true"
            >
              <line x1="18" y1="6" x2="6" y2="18" />
              <line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        </div>

        <div className="modal-body">
          {step === 'url' && (
            <div className="url-form">
              <div className="form-group">
                <label htmlFor="feed-url">{t('feedUrl')}</label>
                <input
                  id="feed-url"
                  type="url"
                  value={url}
                  onChange={(e) => setUrl(e.target.value)}
                  placeholder="https://example.com/feed.xml"
                />
                <p className="hint">{t('feedUrlHint')}</p>
              </div>

              <div className="form-group">
                <label htmlFor="feed-name">{t('feedName')}</label>
                <input
                  id="feed-name"
                  type="text"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  placeholder={t('feedNamePlaceholder')}
                />
              </div>

              {previewMutation.error && <div className="error-message">{t('fetchError')}</div>}
            </div>
          )}

          {step === 'preview' && previewData && (
            <div className="preview-section">
              <div className="preview-info">
                <div className="info-card">
                  <span className="info-label">{t('format')}</span>
                  <span className="info-value">{getFormatLabel(previewData.format)}</span>
                </div>
                <div className="info-card">
                  <span className="info-label">{t('itemsFound')}</span>
                  <span className="info-value">{previewData.totalItems}</span>
                </div>
              </div>

              <div className="available-fields">
                <h4>{t('availableFields')}</h4>
                <div className="fields-list">
                  {previewData.availableFields.map((field) => (
                    <span key={field} className="field-tag">
                      {field}
                    </span>
                  ))}
                </div>
              </div>

              {previewData.sampleItems.length > 0 && (
                <div className="sample-items">
                  <h4>{t('sampleData')}</h4>
                  <pre>{JSON.stringify(previewData.sampleItems[0], null, 2)}</pre>
                </div>
              )}

              <div className="form-group">
                <label htmlFor="sync-freq">{t('syncFrequency')}</label>
                <select
                  id="sync-freq"
                  value={frequency}
                  onChange={(e) => setFrequency(e.target.value as SyncFrequency)}
                >
                  {FREQUENCY_VALUES.map((value) => (
                    <option key={value} value={value}>
                      {tFreq(value)}
                    </option>
                  ))}
                </select>
              </div>
            </div>
          )}

          {step === 'mapping' && (
            <div className="mapping-form">
              <p className="mapping-hint">{t('mappingHint')}</p>
              {Object.entries(DEFAULT_FIELD_MAPPING).map(([localField, defaultRemote]) => (
                <div key={localField} className="mapping-row">
                  <label htmlFor={`map-${localField}`}>{formatFieldLabel(localField)}</label>
                  <input
                    id={`map-${localField}`}
                    type="text"
                    value={fieldMapping[localField] || ''}
                    onChange={(e) =>
                      setFieldMapping((prev) => ({ ...prev, [localField]: e.target.value }))
                    }
                    placeholder={defaultRemote}
                  />
                </div>
              ))}
            </div>
          )}
        </div>

        <div className="modal-footer">
          {step === 'url' && (
            <button
              type="button"
              className="primary"
              onClick={handlePreview}
              disabled={!url || previewMutation.isPending}
            >
              {previewMutation.isPending ? t('fetching') : t('fetchFeed')}
            </button>
          )}

          {step === 'preview' && (
            <>
              <button type="button" className="secondary" onClick={() => setStep('url')}>
                {t('back')}
              </button>
              <button type="button" className="primary" onClick={() => setStep('mapping')}>
                {t('configureMapping')}
              </button>
            </>
          )}

          {step === 'mapping' && (
            <>
              <button type="button" className="secondary" onClick={() => setStep('preview')}>
                {t('back')}
              </button>
              <button
                type="button"
                className="primary"
                onClick={handleCreate}
                disabled={createMutation.isPending}
              >
                {createMutation.isPending ? t('creating') : t('createFeed')}
              </button>
            </>
          )}
        </div>
      </div>

      <style jsx>{`
        .modal-overlay {
          position: fixed;
          inset: 0;
          background: rgba(0, 0, 0, 0.5);
          display: flex;
          align-items: center;
          justify-content: center;
          z-index: 100;
        }

        .modal-content {
          background: var(--ppt-bg-surface);
          border-radius: 16px;
          width: 90%;
          max-width: 600px;
          max-height: 90vh;
          overflow-y: auto;
        }

        .modal-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          padding: 20px 24px;
          border-bottom: 1px solid var(--ppt-border-default);
        }

        .modal-header h2 {
          font-size: 1.25rem;
          color: var(--ppt-fg-primary);
          margin: 0;
        }

        .close-button {
          background: none;
          border: none;
          padding: 4px;
          cursor: pointer;
          color: var(--ppt-fg-muted);
        }

        .modal-body {
          padding: 24px;
        }

        .url-form,
        .preview-section,
        .mapping-form {
          display: flex;
          flex-direction: column;
          gap: 16px;
        }

        .form-group {
          display: flex;
          flex-direction: column;
          gap: 6px;
        }

        .form-group label {
          font-size: 13px;
          font-weight: 500;
          color: var(--ppt-fg-secondary);
        }

        .form-group input,
        .form-group select {
          padding: 10px 12px;
          border: 1px solid var(--ppt-border-strong);
          border-radius: 8px;
          font-size: 14px;
        }

        .form-group input:focus-visible,
        .form-group select:focus-visible {
          outline: var(--ppt-focus-ring-width) solid var(--ppt-focus-ring-color);
          outline-offset: var(--ppt-focus-ring-offset);
          border-color: var(--ppt-color-primary);
        }

        .hint {
          font-size: 12px;
          color: var(--ppt-fg-muted);
          margin: 0;
        }

        .error-message {
          padding: 12px 16px;
          background: var(--ppt-color-danger-light);
          color: var(--ppt-color-danger-dark);
          border-radius: 8px;
          font-size: 14px;
        }

        .preview-info {
          display: flex;
          gap: 16px;
        }

        .info-card {
          flex: 1;
          padding: 16px;
          background: var(--ppt-bg-app);
          border-radius: 8px;
          text-align: center;
        }

        .info-label {
          display: block;
          font-size: 12px;
          color: var(--ppt-fg-muted);
          margin-bottom: 4px;
        }

        .info-value {
          font-size: 1.25rem;
          font-weight: 600;
          color: var(--ppt-fg-primary);
        }

        .available-fields h4,
        .sample-items h4 {
          font-size: 14px;
          color: var(--ppt-fg-secondary);
          margin: 0 0 12px;
        }

        .fields-list {
          display: flex;
          flex-wrap: wrap;
          gap: 8px;
        }

        .field-tag {
          padding: 4px 10px;
          background: var(--ppt-border-default);
          border-radius: 4px;
          font-size: 12px;
          color: var(--ppt-fg-secondary);
        }

        .sample-items pre {
          padding: 12px;
          background: var(--ppt-bg-app);
          border-radius: 8px;
          font-size: 11px;
          overflow-x: auto;
          max-height: 150px;
        }

        .mapping-hint {
          color: var(--ppt-fg-muted);
          font-size: 14px;
          margin: 0 0 8px;
        }

        .mapping-row {
          display: flex;
          align-items: center;
          gap: 12px;
        }

        .mapping-row label {
          width: 100px;
          font-size: 13px;
          font-weight: 500;
          color: var(--ppt-fg-secondary);
        }

        .mapping-row input {
          flex: 1;
          padding: 8px 12px;
          border: 1px solid var(--ppt-border-strong);
          border-radius: 6px;
          font-size: 13px;
        }

        .modal-footer {
          display: flex;
          justify-content: flex-end;
          gap: 12px;
          padding: 16px 24px;
          border-top: 1px solid var(--ppt-border-default);
          background: var(--ppt-bg-app);
        }

        .modal-footer button {
          padding: 10px 20px;
          border-radius: 8px;
          font-size: 14px;
          font-weight: 500;
          cursor: pointer;
        }

        .secondary {
          background: var(--ppt-bg-surface);
          border: 1px solid var(--ppt-border-strong);
          color: var(--ppt-fg-secondary);
        }

        .primary {
          background: var(--ppt-color-primary);
          border: none;
          color: var(--ppt-fg-on-accent);
        }

        .primary:disabled {
          opacity: 0.5;
          cursor: not-allowed;
        }
      `}</style>
    </div>
  );
}

function FeedsSkeleton() {
  return (
    <div className="skeleton-list">
      {[1, 2].map((i) => (
        <div key={`skel-${i}`} className="skeleton-card" />
      ))}
      <style jsx>{`
        .skeleton-list {
          display: flex;
          flex-direction: column;
          gap: 16px;
        }
        .skeleton-card {
          height: 180px;
          background: var(--ppt-border-default);
          border-radius: 12px;
        }
      `}</style>
    </div>
  );
}

function EmptyState({ onAdd }: { onAdd: () => void }) {
  const t = useTranslations('import.feed');
  return (
    <div className="empty-state">
      <svg
        width="64"
        height="64"
        viewBox="0 0 24 24"
        fill="none"
        stroke="var(--ppt-fg-subtle)"
        strokeWidth="1.5"
        aria-hidden="true"
      >
        <path d="M4 11a9 9 0 0 1 9 9" />
        <path d="M4 4a16 16 0 0 1 16 16" />
        <circle cx="5" cy="19" r="2" />
      </svg>
      <h3>{t('emptyTitle')}</h3>
      <p>{t('emptyText')}</p>
      <button type="button" onClick={onAdd}>
        {t('addFirst')}
      </button>
      <style jsx>{`
        .empty-state {
          display: flex;
          flex-direction: column;
          align-items: center;
          padding: 64px 24px;
          text-align: center;
          background: var(--ppt-bg-surface);
          border: 1px solid var(--ppt-border-default);
          border-radius: 12px;
        }
        h3 {
          font-size: 1.25rem;
          color: var(--ppt-fg-primary);
          margin: 24px 0 8px;
        }
        p {
          color: var(--ppt-fg-muted);
          margin: 0 0 24px;
        }
        button {
          padding: 12px 24px;
          background: var(--ppt-color-primary);
          color: var(--ppt-fg-on-accent);
          border: none;
          border-radius: 8px;
          font-weight: 500;
          cursor: pointer;
        }
      `}</style>
    </div>
  );
}

function getFeedStatusConfig(status: FeedSource['status']) {
  const configs = {
    active: {
      color: 'var(--ppt-color-success-hover)',
      bg: 'var(--ppt-color-success-light)',
    },
    paused: { color: 'var(--ppt-fg-muted)', bg: 'var(--ppt-border-default)' },
    error: {
      color: 'var(--ppt-color-danger)',
      bg: 'var(--ppt-color-danger-light)',
    },
  };
  return configs[status];
}

function getFormatLabel(format: FeedFormat): string {
  const labels: Record<FeedFormat, string> = {
    xml: 'XML',
    rss: 'RSS',
    atom: 'Atom',
    json: 'JSON',
  };
  return labels[format] || format.toUpperCase();
}

function formatFieldLabel(field: string): string {
  return field.replace(/([A-Z])/g, ' $1').replace(/^./, (s) => s.toUpperCase());
}
