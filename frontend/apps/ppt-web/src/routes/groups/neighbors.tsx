/**
 * Neighbors route group (Epic 6, Story 6.6).
 *
 * Owns the neighbor route-wrapper components and the `<Route>` table fragment.
 * Extracted from App.tsx to isolate neighbor work.
 */
import { useBuildings } from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Route, useNavigate, useParams } from 'react-router-dom';
import { useToast } from '../../components';
import type { PrivacySettings } from '../../features/neighbors';
import { useNeighbors, usePrivacySettings } from '../../features/neighbors';
import { NeighborDetailPage, NeighborsPage, NeighborsPrivacySettingsPage } from '../lazyRoutes';

/**
 * Neighbors listing page — wired to backend GET /api/v1/buildings/{id}/neighbors.
 *
 * Building context: The route uses the first building from `useBuildings()`.
 * Once a building-selector is added to the app-shell, swap for the selected ID.
 */
function NeighborsPageRoute() {
  const navigate = useNavigate();
  const { data: buildingsData, isLoading: isLoadingBuildings } = useBuildings();

  // Pick the first building the user has access to; URL-param selection deferred.
  const buildingId = buildingsData?.items?.[0]?.id ?? '';
  const buildingName = buildingsData?.items?.[0]?.name;

  const { neighbors, isLoading, error } = useNeighbors(buildingId, !isLoadingBuildings);

  return (
    <NeighborsPage
      neighbors={neighbors}
      buildingName={buildingName}
      isLoading={isLoading || isLoadingBuildings}
      error={error}
      onViewProfile={(n) => navigate(`/neighbors/${n.id}`)}
      onContact={(n) => navigate(`/messages/new?recipientId=${n.id}`)}
      onManagePrivacy={() => navigate('/neighbors/privacy')}
    />
  );
}

/**
 * Neighbor detail page — extracts :neighborId from the URL.
 * Fetches the neighbor from the building list and finds by id.
 */
function NeighborDetailRoute() {
  const { neighborId } = useParams<{ neighborId: string }>();
  const navigate = useNavigate();
  const { t } = useTranslation();
  const { data: buildingsData } = useBuildings();

  const buildingId = buildingsData?.items?.[0]?.id ?? '';
  const { neighbors, isLoading } = useNeighbors(buildingId, !!buildingId);
  const neighbor = neighbors.find((n) => n.id === neighborId);

  if (!neighborId) {
    return (
      <div className="error-page">
        <h1>{t('errors.notFound')}</h1>
        <p>{t('errors.neighborNotFound', 'Neighbor not found.')}</p>
      </div>
    );
  }

  if (isLoading) {
    return (
      <div className="loading-page">
        <p>{t('common.loading')}</p>
      </div>
    );
  }

  if (!neighbor) {
    return (
      <div className="error-page">
        <h1>{t('errors.notFound')}</h1>
        <p>{t('errors.neighborNotFound', 'Neighbor not found.')}</p>
      </div>
    );
  }

  return (
    <NeighborDetailPage
      neighbor={neighbor}
      onBack={() => navigate('/neighbors')}
      onMessage={(n) => navigate(`/messages/new?recipientId=${n.id}`)}
    />
  );
}

/**
 * Neighbor privacy settings page — wired to GET/PUT /api/v1/users/me/privacy.
 */
function NeighborsPrivacySettingsRoute() {
  const navigate = useNavigate();
  const { showToast } = useToast();
  const { t } = useTranslation();

  const { settings, isLoading, isSubmitting, error, updateSettings } = usePrivacySettings();
  const [successMessage, setSuccessMessage] = useState<string | null>(null);

  const handleSubmit = async (newSettings: PrivacySettings) => {
    try {
      await updateSettings(newSettings);
      setSuccessMessage(t('neighbors.privacy.saved', 'Privacy settings saved.'));
      showToast({
        type: 'success',
        title: t('common.success'),
        message: t('neighbors.privacy.saved', 'Privacy settings saved.'),
      });
    } catch {
      showToast({
        type: 'error',
        title: t('common.error'),
        message: t('neighbors.privacy.saveFailed', 'Failed to save privacy settings.'),
      });
    }
  };

  return (
    <NeighborsPrivacySettingsPage
      settings={settings}
      isLoading={isLoading}
      error={error}
      isSubmitting={isSubmitting}
      successMessage={successMessage}
      onSubmit={handleSubmit}
      onBack={() => navigate('/neighbors')}
    />
  );
}

/** Neighbors routes (Epic 6, Story 6.6). */
export function neighborRoutes() {
  return (
    <>
      <Route path="/neighbors" element={<NeighborsPageRoute />} />
      <Route path="/neighbors/:neighborId" element={<NeighborDetailRoute />} />
      <Route path="/neighbors/privacy" element={<NeighborsPrivacySettingsRoute />} />
    </>
  );
}
