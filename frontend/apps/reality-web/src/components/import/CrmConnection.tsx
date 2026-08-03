/**
 * CrmConnection Component
 *
 * Connect and manage CRM systems (Epic 46, Story 46.2).
 */

'use client';

import type {
  CrmConnection as CrmConnectionType,
  CrmFieldMapping,
  CrmProvider,
} from '@ppt/reality-api-client';
import {
  useCreateCrmConnection,
  useCrmConnections,
  useDeleteCrmConnection,
  useMyAgency,
  useSyncCrmConnection,
  useTestCrmConnection,
} from '@ppt/reality-api-client';
import { useTranslations } from 'next-intl';
import { useState } from 'react';

const CRM_PROVIDERS: { id: CrmProvider; name: string; logo: string }[] = [
  { id: 'salesforce', name: 'Salesforce', logo: '🔵' },
  { id: 'hubspot', name: 'HubSpot', logo: '🟠' },
  { id: 'pipedrive', name: 'Pipedrive', logo: '🟢' },
  { id: 'zoho', name: 'Zoho CRM', logo: '🔴' },
  { id: 'custom', name: 'Custom API', logo: '⚙️' },
];

const DEFAULT_FIELD_MAPPING: CrmFieldMapping = {
  title: 'name',
  description: 'description',
  price: 'amount',
  propertyType: 'property_type',
  address: 'address',
  city: 'city',
  rooms: 'bedrooms',
  size: 'square_feet',
};

export function CrmConnection() {
  const t = useTranslations('import.crm');
  const [showModal, setShowModal] = useState(false);
  const [selectedProvider, setSelectedProvider] = useState<CrmProvider | null>(null);

  const { data: agency } = useMyAgency();
  const { data: connections, isLoading } = useCrmConnections(agency?.id || '');

  return (
    <div className="crm-connection">
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
          {t('addConnection')}
        </button>
      </div>

      {isLoading ? (
        <ConnectionsSkeleton />
      ) : connections?.length === 0 ? (
        <EmptyState onAdd={() => setShowModal(true)} />
      ) : (
        <div className="connections-grid">
          {connections?.map((conn) => (
            <ConnectionCard key={conn.id} connection={conn} agencyId={agency?.id || ''} />
          ))}
        </div>
      )}

      {showModal && (
        <AddConnectionModal
          agencyId={agency?.id || ''}
          selectedProvider={selectedProvider}
          onSelectProvider={setSelectedProvider}
          onClose={() => {
            setShowModal(false);
            setSelectedProvider(null);
          }}
        />
      )}

      <style jsx>{`
        .crm-connection {
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

        .connections-grid {
          display: grid;
          grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
          gap: 20px;
        }
      `}</style>
    </div>
  );
}

function ConnectionCard({
  connection,
  agencyId,
}: {
  connection: CrmConnectionType;
  agencyId: string;
}) {
  const t = useTranslations('import.crm');
  const tFreq = useTranslations('import.feed.frequency');
  const deleteMutation = useDeleteCrmConnection(agencyId);
  const syncMutation = useSyncCrmConnection(agencyId, connection.id);

  const provider = CRM_PROVIDERS.find((p) => p.id === connection.provider);
  const statusConfig = getStatusConfig(connection.status);

  const handleDelete = async () => {
    if (confirm(t('confirmRemove'))) {
      await deleteMutation.mutateAsync(connection.id);
    }
  };

  return (
    <div className="connection-card">
      <div className="card-header">
        <div className="provider-info">
          <span className="provider-logo">{provider?.logo}</span>
          <div>
            <h3>{connection.name}</h3>
            <span className="provider-name">{provider?.name}</span>
          </div>
        </div>
        <span
          className="status-badge"
          style={{ background: statusConfig.bg, color: statusConfig.color }}
        >
          {t(`status.${connection.status}`)}
        </span>
      </div>

      <div className="card-body">
        {connection.lastSyncAt && (
          <div className="sync-info">
            <span className="label">{t('lastSync')}</span>
            <span className="value">{new Date(connection.lastSyncAt).toLocaleString()}</span>
          </div>
        )}
        {connection.nextSyncAt && (
          <div className="sync-info">
            <span className="label">{t('nextSync')}</span>
            <span className="value">{new Date(connection.nextSyncAt).toLocaleString()}</span>
          </div>
        )}
        <div className="sync-info">
          <span className="label">{t('frequency')}</span>
          <span className="value">{tFreq(connection.syncFrequency)}</span>
        </div>
      </div>

      <div className="card-actions">
        <button
          type="button"
          className="action-button sync"
          onClick={() => syncMutation.mutate()}
          disabled={syncMutation.isPending || connection.status === 'syncing'}
        >
          {syncMutation.isPending ? t('syncing') : t('syncNow')}
        </button>
        <button type="button" className="action-button settings">
          {t('settings')}
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

      <style jsx>{`
        .connection-card {
          background: var(--ppt-bg-surface);
          border: 1px solid var(--ppt-border-default);
          border-radius: 12px;
          overflow: hidden;
        }

        .card-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          padding: 20px;
          border-bottom: 1px solid var(--ppt-bg-subtle);
        }

        .provider-info {
          display: flex;
          align-items: center;
          gap: 12px;
        }

        .provider-logo {
          font-size: 24px;
        }

        h3 {
          font-size: 1rem;
          color: var(--ppt-fg-primary);
          margin: 0;
        }

        .provider-name {
          font-size: 13px;
          color: var(--ppt-fg-muted);
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

        .sync-info {
          display: flex;
          justify-content: space-between;
          margin-bottom: 8px;
          font-size: 13px;
        }

        .sync-info:last-child {
          margin-bottom: 0;
        }

        .sync-info .label {
          color: var(--ppt-fg-muted);
        }

        .sync-info .value {
          color: var(--ppt-fg-secondary);
        }

        .card-actions {
          display: flex;
          gap: 8px;
          padding: 16px 20px;
          background: var(--ppt-bg-app);
          border-top: 1px solid var(--ppt-bg-subtle);
        }

        .action-button {
          flex: 1;
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

        .action-button.settings {
          background: var(--ppt-bg-surface);
          border: 1px solid var(--ppt-border-strong);
          color: var(--ppt-fg-secondary);
        }

        .action-button.delete {
          background: var(--ppt-bg-surface);
          border: 1px solid var(--ppt-color-danger-light);
          color: var(--ppt-color-danger);
        }

        .action-button.delete:hover {
          background: var(--ppt-color-danger-light);
        }
      `}</style>
    </div>
  );
}

function AddConnectionModal({
  agencyId,
  selectedProvider,
  onSelectProvider,
  onClose,
}: {
  agencyId: string;
  selectedProvider: CrmProvider | null;
  onSelectProvider: (provider: CrmProvider) => void;
  onClose: () => void;
}) {
  const t = useTranslations('import.crm');
  const [step, setStep] = useState<'select' | 'configure' | 'mapping'>('select');
  const [name, setName] = useState('');
  const [apiKey, setApiKey] = useState('');
  const [instanceUrl, setInstanceUrl] = useState('');
  const [fieldMapping, setFieldMapping] = useState<CrmFieldMapping>(DEFAULT_FIELD_MAPPING);
  const [testResult, setTestResult] = useState<{ success: boolean; message: string } | null>(null);

  const testMutation = useTestCrmConnection(agencyId);
  const createMutation = useCreateCrmConnection(agencyId);

  const handleTest = async () => {
    if (!selectedProvider) return;
    try {
      const result = await testMutation.mutateAsync({
        provider: selectedProvider,
        apiKey,
        instanceUrl: selectedProvider === 'salesforce' ? instanceUrl : undefined,
      });
      setTestResult(result);
      if (result.success) {
        setStep('mapping');
      }
    } catch (error) {
      setTestResult({
        success: false,
        message: error instanceof Error ? error.message : t('connectionFailed'),
      });
    }
  };

  const handleCreate = async () => {
    if (!selectedProvider) return;
    await createMutation.mutateAsync({
      provider: selectedProvider,
      name:
        name ||
        `${CRM_PROVIDERS.find((p) => p.id === selectedProvider)?.name} ${t('connectionSuffix')}`,
      credentials: {
        apiKey,
        instanceUrl: selectedProvider === 'salesforce' ? instanceUrl : undefined,
      },
      fieldMapping,
      syncFrequency: 'daily',
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
            {step === 'select' && t('modalSelect')}
            {step === 'configure' && t('modalConfigure')}
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
          {step === 'select' && (
            <div className="provider-grid">
              {CRM_PROVIDERS.map((provider) => (
                <button
                  key={provider.id}
                  type="button"
                  className={`provider-card ${selectedProvider === provider.id ? 'selected' : ''}`}
                  onClick={() => {
                    onSelectProvider(provider.id);
                    setStep('configure');
                  }}
                >
                  <span className="provider-logo">{provider.logo}</span>
                  <span className="provider-name">{provider.name}</span>
                </button>
              ))}
            </div>
          )}

          {step === 'configure' && selectedProvider && (
            <div className="configure-form">
              <div className="form-group">
                <label htmlFor="conn-name">{t('connectionName')}</label>
                <input
                  id="conn-name"
                  type="text"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  placeholder={t('connectionNamePlaceholder', {
                    provider: CRM_PROVIDERS.find((p) => p.id === selectedProvider)?.name ?? '',
                  })}
                />
              </div>

              <div className="form-group">
                <label htmlFor="api-key">{t('apiKey')}</label>
                <input
                  id="api-key"
                  type="password"
                  value={apiKey}
                  onChange={(e) => setApiKey(e.target.value)}
                  placeholder={t('apiKeyPlaceholder')}
                />
              </div>

              {selectedProvider === 'salesforce' && (
                <div className="form-group">
                  <label htmlFor="instance-url">{t('instanceUrl')}</label>
                  <input
                    id="instance-url"
                    type="url"
                    value={instanceUrl}
                    onChange={(e) => setInstanceUrl(e.target.value)}
                    placeholder="https://yourorg.salesforce.com"
                  />
                </div>
              )}

              {testResult && (
                <div className={`test-result ${testResult.success ? 'success' : 'error'}`}>
                  {testResult.success ? '✓' : '✕'} {testResult.message}
                </div>
              )}
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
          {step === 'configure' && (
            <>
              <button type="button" className="secondary" onClick={() => setStep('select')}>
                {t('back')}
              </button>
              <button
                type="button"
                className="primary"
                onClick={handleTest}
                disabled={!apiKey || testMutation.isPending}
              >
                {testMutation.isPending ? t('testing') : t('testConnection')}
              </button>
            </>
          )}

          {step === 'mapping' && (
            <>
              <button type="button" className="secondary" onClick={() => setStep('configure')}>
                {t('back')}
              </button>
              <button
                type="button"
                className="primary"
                onClick={handleCreate}
                disabled={createMutation.isPending}
              >
                {createMutation.isPending ? t('creating') : t('createConnection')}
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
          max-width: 560px;
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

        .provider-grid {
          display: grid;
          grid-template-columns: repeat(2, 1fr);
          gap: 12px;
        }

        .provider-card {
          display: flex;
          flex-direction: column;
          align-items: center;
          gap: 8px;
          padding: 24px;
          border: 2px solid var(--ppt-border-default);
          border-radius: 12px;
          background: var(--ppt-bg-surface);
          cursor: pointer;
          transition: all 0.2s;
        }

        .provider-card:hover {
          border-color: var(--ppt-color-primary);
        }

        .provider-card.selected {
          border-color: var(--ppt-color-primary);
          background: var(--ppt-color-primary-soft-bg);
        }

        .provider-card .provider-logo {
          font-size: 32px;
        }

        .provider-card .provider-name {
          font-size: 14px;
          font-weight: 500;
          color: var(--ppt-fg-secondary);
        }

        .configure-form,
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

        .form-group input {
          padding: 10px 12px;
          border: 1px solid var(--ppt-border-strong);
          border-radius: 8px;
          font-size: 14px;
        }

        .form-group input:focus-visible {
          outline: var(--ppt-focus-ring-width) solid var(--ppt-focus-ring-color);
          outline-offset: var(--ppt-focus-ring-offset);
          border-color: var(--ppt-color-primary);
        }

        .test-result {
          padding: 12px 16px;
          border-radius: 8px;
          font-size: 14px;
        }

        .test-result.success {
          background: var(--ppt-color-success-light);
          color: var(--ppt-color-success-dark);
        }

        .test-result.error {
          background: var(--ppt-color-danger-light);
          color: var(--ppt-color-danger-dark);
        }

        .mapping-hint {
          color: var(--ppt-fg-muted);
          margin: 0 0 8px;
          font-size: 14px;
        }

        .mapping-row {
          display: flex;
          align-items: center;
          gap: 12px;
        }

        .mapping-row label {
          width: 120px;
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

function ConnectionsSkeleton() {
  return (
    <div className="skeleton-grid">
      {[1, 2].map((i) => (
        <div key={`skel-${i}`} className="skeleton-card" />
      ))}
      <style jsx>{`
        .skeleton-grid {
          display: grid;
          grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
          gap: 20px;
        }
        .skeleton-card {
          height: 200px;
          background: var(--ppt-border-default);
          border-radius: 12px;
        }
      `}</style>
    </div>
  );
}

function EmptyState({ onAdd }: { onAdd: () => void }) {
  const t = useTranslations('import.crm');
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
        <path d="M20 7h-4m0 0V3m0 4l4-4M4 17h4m0 0v4m0-4l-4 4" />
        <circle cx="12" cy="12" r="3" />
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

function getStatusConfig(status: CrmConnectionType['status']) {
  const configs = {
    connected: {
      color: 'var(--ppt-color-success-hover)',
      bg: 'var(--ppt-color-success-light)',
    },
    disconnected: {
      color: 'var(--ppt-fg-muted)',
      bg: 'var(--ppt-border-default)',
    },
    error: {
      color: 'var(--ppt-color-danger)',
      bg: 'var(--ppt-color-danger-light)',
    },
    syncing: {
      color: 'var(--ppt-color-primary)',
      bg: 'var(--ppt-color-primary-soft-bg)',
    },
  };
  return configs[status];
}

function formatFieldLabel(field: string): string {
  return field.replace(/([A-Z])/g, ' $1').replace(/^./, (s) => s.toUpperCase());
}
