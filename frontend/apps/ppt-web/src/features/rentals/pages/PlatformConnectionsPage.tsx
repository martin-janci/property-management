/**
 * PlatformConnectionsPage - Manage platform integrations.
 * Epic 18: Short-Term Rental Integration (Story 18.1)
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ConfirmationDialog } from '../../../components/ConfirmationDialog';
import { ConnectionSettingsDialog } from '../components/ConnectionSettingsDialog';
import { PlatformConnectionCard } from '../components/PlatformConnectionCard';
import type { ConnectionSettings, PlatformConnection, PlatformType } from '../types';

const PLATFORM_NAMES: Record<PlatformType, string> = {
  airbnb: 'Airbnb',
  booking: 'Booking.com',
};

interface Unit {
  id: string;
  name: string;
  buildingName: string;
}

interface PlatformConnectionsPageProps {
  connections: PlatformConnection[];
  units: Unit[];
  isLoading?: boolean;
  syncingConnectionId?: string;
  /** Reactivate a disconnected connection, or start authorizing a pending one. */
  onConnect: (connectionId: string) => void;
  /** Called only after the user confirmed; the dialog stays open until it settles. */
  onDisconnect: (connectionId: string) => Promise<void>;
  onSync: (connectionId: string) => void;
  onLoadSettings: (connectionId: string) => Promise<ConnectionSettings>;
  onSaveSettings: (
    connectionId: string,
    next: ConnectionSettings,
    previous: ConnectionSettings
  ) => Promise<void>;
  onRemoveConnection: (connectionId: string) => Promise<void>;
  onCreateConnection: (unitId: string, platform: PlatformType) => void;
  onBack: () => void;
}

export function PlatformConnectionsPage({
  connections,
  units,
  isLoading,
  syncingConnectionId,
  onConnect,
  onDisconnect,
  onSync,
  onLoadSettings,
  onSaveSettings,
  onRemoveConnection,
  onCreateConnection,
  onBack,
}: PlatformConnectionsPageProps) {
  const { t } = useTranslation();
  const [disconnectTarget, setDisconnectTarget] = useState<PlatformConnection | null>(null);
  const [isDisconnecting, setIsDisconnecting] = useState(false);
  const [settingsTarget, setSettingsTarget] = useState<PlatformConnection | null>(null);

  const handleConfirmDisconnect = async () => {
    if (!disconnectTarget) return;
    setIsDisconnecting(true);
    try {
      await onDisconnect(disconnectTarget.id);
    } catch {
      // The caller surfaces the failure (toast); just close the dialog.
    } finally {
      setIsDisconnecting(false);
      setDisconnectTarget(null);
    }
  };
  const [showAddModal, setShowAddModal] = useState(false);
  const [selectedUnitId, setSelectedUnitId] = useState<string>('');
  const [selectedPlatform, setSelectedPlatform] = useState<PlatformType>('airbnb');
  const [filterPlatform, setFilterPlatform] = useState<PlatformType | ''>('');

  const filteredConnections = filterPlatform
    ? connections.filter((c) => c.platform === filterPlatform)
    : connections;

  const connectedCount = connections.filter((c) => c.status === 'connected').length;
  const errorCount = connections.filter((c) => c.status === 'error').length;

  const handleAddConnection = () => {
    if (selectedUnitId && selectedPlatform) {
      onCreateConnection(selectedUnitId, selectedPlatform);
      setShowAddModal(false);
      setSelectedUnitId('');
      setSelectedPlatform('airbnb');
    }
  };

  return (
    <div className="min-h-screen bg-gray-100">
      {/* Header */}
      <div className="bg-white shadow">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-4">
              <button type="button" onClick={onBack} className="text-gray-500 hover:text-gray-700">
                <svg
                  className="w-6 h-6"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                  aria-hidden="true"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M10 19l-7-7m0 0l7-7m-7 7h18"
                  />
                </svg>
              </button>
              <div>
                <h1 className="text-2xl font-bold text-gray-900">
                  {t('rentals.connections.title')}
                </h1>
                <p className="mt-1 text-sm text-gray-500">{t('rentals.connections.subtitle')}</p>
              </div>
            </div>
            <button
              type="button"
              onClick={() => setShowAddModal(true)}
              className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700"
            >
              {t('rentals.connections.addConnection')}
            </button>
          </div>
        </div>
      </div>

      {/* Stats */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6">
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div className="bg-white rounded-lg shadow p-4 flex items-center gap-4">
            <div className="p-3 bg-blue-100 rounded-full">
              <svg
                className="w-6 h-6 text-blue-600"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
                aria-hidden="true"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1"
                />
              </svg>
            </div>
            <div>
              <p className="text-sm text-gray-500">{t('rentals.connections.total')}</p>
              <p className="text-2xl font-bold text-gray-900">{connections.length}</p>
            </div>
          </div>
          <div className="bg-white rounded-lg shadow p-4 flex items-center gap-4">
            <div className="p-3 bg-green-100 rounded-full">
              <svg
                className="w-6 h-6 text-green-600"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
                aria-hidden="true"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M5 13l4 4L19 7"
                />
              </svg>
            </div>
            <div>
              <p className="text-sm text-gray-500">{t('rentals.connections.connected')}</p>
              <p className="text-2xl font-bold text-green-600">{connectedCount}</p>
            </div>
          </div>
          <div className="bg-white rounded-lg shadow p-4 flex items-center gap-4">
            <div className="p-3 bg-red-100 rounded-full">
              <svg
                className="w-6 h-6 text-red-600"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
                aria-hidden="true"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                />
              </svg>
            </div>
            <div>
              <p className="text-sm text-gray-500">{t('rentals.connections.errors')}</p>
              <p className="text-2xl font-bold text-red-600">{errorCount}</p>
            </div>
          </div>
        </div>
      </div>

      {/* Filters */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 pb-4">
        <div className="flex items-center gap-4">
          <span className="text-sm text-gray-500">{t('rentals.connections.filterBy')}</span>
          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={() => setFilterPlatform('')}
              className={`px-3 py-1 text-sm rounded-full ${
                filterPlatform === ''
                  ? 'bg-blue-100 text-blue-700'
                  : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
              }`}
            >
              {t('common.all')}
            </button>
            <button
              type="button"
              onClick={() => setFilterPlatform('airbnb')}
              className={`px-3 py-1 text-sm rounded-full ${
                filterPlatform === 'airbnb'
                  ? 'bg-rose-100 text-rose-700'
                  : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
              }`}
            >
              Airbnb
            </button>
            <button
              type="button"
              onClick={() => setFilterPlatform('booking')}
              className={`px-3 py-1 text-sm rounded-full ${
                filterPlatform === 'booking'
                  ? 'bg-blue-100 text-blue-700'
                  : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
              }`}
            >
              Booking.com
            </button>
          </div>
        </div>
      </div>

      {/* Content */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 pb-8">
        {isLoading ? (
          <div className="flex items-center justify-center h-64">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
          </div>
        ) : filteredConnections.length === 0 ? (
          <div className="bg-white rounded-lg shadow p-12 text-center">
            <svg
              className="w-16 h-16 mx-auto text-gray-400"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              aria-hidden="true"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1"
              />
            </svg>
            <h2 className="mt-4 text-lg font-medium text-gray-900">
              {t('rentals.connections.noConnections')}
            </h2>
            <p className="mt-2 text-sm text-gray-500">
              {t('rentals.connections.noConnectionsDesc')}
            </p>
            <button
              type="button"
              onClick={() => setShowAddModal(true)}
              className="mt-4 px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700"
            >
              {t('rentals.connections.addFirstConnection')}
            </button>
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {filteredConnections.map((connection) => (
              <PlatformConnectionCard
                key={connection.id}
                connection={connection}
                onConnect={onConnect}
                onDisconnect={() => setDisconnectTarget(connection)}
                onSync={onSync}
                onSettings={() => setSettingsTarget(connection)}
                isSyncing={syncingConnectionId === connection.id}
              />
            ))}
          </div>
        )}
      </div>

      {/* Add Connection Modal */}
      {showAddModal && (
        <div className="fixed inset-0 z-50 overflow-y-auto">
          <div className="flex min-h-full items-center justify-center p-4">
            <div
              className="fixed inset-0 bg-black bg-opacity-25"
              onClick={() => setShowAddModal(false)}
            />
            <div className="relative bg-white rounded-lg shadow-xl max-w-md w-full p-6">
              <h3 className="text-lg font-semibold text-gray-900">
                {t('rentals.connections.addConnection')}
              </h3>
              <p className="mt-2 text-sm text-gray-500">
                {t('rentals.connections.addConnectionDesc')}
              </p>

              <div className="mt-6 space-y-4">
                <div>
                  <label htmlFor="unit-select" className="block text-sm font-medium text-gray-700">
                    {t('rentals.connections.selectUnit')}
                  </label>
                  <select
                    id="unit-select"
                    value={selectedUnitId}
                    onChange={(e) => setSelectedUnitId(e.target.value)}
                    className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500"
                  >
                    <option value="">{t('common.select')}</option>
                    {units.map((unit) => (
                      <option key={unit.id} value={unit.id}>
                        {unit.name} - {unit.buildingName}
                      </option>
                    ))}
                  </select>
                </div>

                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-2">
                    {t('rentals.connections.selectPlatform')}
                  </label>
                  <div className="grid grid-cols-2 gap-3">
                    <button
                      type="button"
                      onClick={() => setSelectedPlatform('airbnb')}
                      className={`p-4 rounded-lg border-2 transition-colors ${
                        selectedPlatform === 'airbnb'
                          ? 'border-rose-500 bg-rose-50'
                          : 'border-gray-200 hover:border-gray-300'
                      }`}
                    >
                      <div className="w-10 h-10 mx-auto bg-rose-100 rounded-lg flex items-center justify-center">
                        <span className="text-xl font-bold text-rose-500">A</span>
                      </div>
                      <p className="mt-2 text-sm font-medium text-gray-900">Airbnb</p>
                    </button>
                    <button
                      type="button"
                      onClick={() => setSelectedPlatform('booking')}
                      className={`p-4 rounded-lg border-2 transition-colors ${
                        selectedPlatform === 'booking'
                          ? 'border-blue-500 bg-blue-50'
                          : 'border-gray-200 hover:border-gray-300'
                      }`}
                    >
                      <div className="w-10 h-10 mx-auto bg-blue-100 rounded-lg flex items-center justify-center">
                        <span className="text-xl font-bold text-blue-600">B</span>
                      </div>
                      <p className="mt-2 text-sm font-medium text-gray-900">Booking.com</p>
                    </button>
                  </div>
                </div>
              </div>

              <div className="mt-6 flex items-center justify-end gap-3">
                <button
                  type="button"
                  onClick={() => setShowAddModal(false)}
                  className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50"
                >
                  {t('common.cancel')}
                </button>
                <button
                  type="button"
                  onClick={handleAddConnection}
                  disabled={!selectedUnitId}
                  className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  {t('rentals.connections.create')}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      <ConfirmationDialog
        isOpen={disconnectTarget !== null}
        title={t('rentals.connections.disconnectTitle')}
        message={
          disconnectTarget
            ? t('rentals.connections.disconnectMessage', {
                platform: PLATFORM_NAMES[disconnectTarget.platform],
                unit: disconnectTarget.unitName ?? '',
              })
            : ''
        }
        confirmLabel={t('rentals.connection.disconnect')}
        cancelLabel={t('common.cancel')}
        variant="danger"
        isLoading={isDisconnecting}
        onConfirm={handleConfirmDisconnect}
        onCancel={() => setDisconnectTarget(null)}
      />

      {settingsTarget && (
        <ConnectionSettingsDialog
          connection={settingsTarget}
          platformName={PLATFORM_NAMES[settingsTarget.platform]}
          onLoad={onLoadSettings}
          onSave={onSaveSettings}
          onRemove={onRemoveConnection}
          onClose={() => setSettingsTarget(null)}
        />
      )}
    </div>
  );
}
