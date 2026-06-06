/**
 * Outages route group (UC-12).
 *
 * Owns the outage route-wrapper components and the `<Route>` table fragment.
 * Extracted from App.tsx to isolate outage work from the central aggregator.
 */
import type {
  OutageCommodity as ApiOutageCommodity,
  OutageSeverity as ApiOutageSeverity,
  OutageStatus as ApiOutageStatus,
  OutageSummary as ApiOutageSummary,
  OutageListQuery,
} from '@ppt/api-client';
import {
  useBuildings,
  useCancelOutage,
  useCreateOutage,
  useOutage,
  useOutages,
  useResolveOutage,
  useStartOutage,
  useUpdateOutage,
} from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Link, Route, useNavigate, useParams } from 'react-router-dom';
import { AuthRequiredGate, useToast } from '../../components';
import { useAuth } from '../../contexts';
import type { ListOutagesParams, OutageDetail } from '../../features/outages';
import { CreateOutagePage, EditOutagePage, OutagesPage, ViewOutagePage } from '../lazyRoutes';
import { transformBuildingForUI } from '../shared';

/**
 * Route wrapper for outages list page (UC-12).
 * Manages filter state and navigation callbacks.
 */
function OutagesPageRoute() {
  const navigate = useNavigate();
  const { user } = useAuth();
  const [queryParams, setQueryParams] = useState<OutageListQuery>({ limit: 10, offset: 0 });

  const { data, isLoading } = useOutages(queryParams);

  if (!user?.organizationId) {
    return <AuthRequiredGate />;
  }

  const outages: ApiOutageSummary[] = data?.outages ?? [];
  const total = data?.total ?? 0;

  const handleNavigateToCreate = () => navigate('/outages/new');
  const handleNavigateToView = (id: string) => navigate(`/outages/${id}`);
  const handleNavigateToEdit = (id: string) => navigate(`/outages/${id}/edit`);
  const handleFilterChange = (params: ListOutagesParams) => {
    setQueryParams({
      status: params.status as ApiOutageStatus | undefined,
      commodity: params.commodity as ApiOutageCommodity | undefined,
      severity: params.severity as ApiOutageSeverity | undefined,
      limit: params.pageSize,
      offset: (params.page - 1) * params.pageSize,
    });
  };

  return (
    <OutagesPage
      outages={outages}
      total={total}
      isLoading={isLoading}
      onNavigateToCreate={handleNavigateToCreate}
      onNavigateToView={handleNavigateToView}
      onNavigateToEdit={handleNavigateToEdit}
      onFilterChange={handleFilterChange}
    />
  );
}

/**
 * Route wrapper for create outage page (UC-12).
 */
function CreateOutagePageRoute() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { user } = useAuth();
  const { showToast } = useToast();
  const createOutage = useCreateOutage();

  // Fetch buildings from API
  const { data: buildingsData, isLoading: isLoadingBuildings } = useBuildings();

  if (!user?.organizationId) {
    return <AuthRequiredGate />;
  }

  // Transform API buildings to UI format
  const buildings = (buildingsData?.items ?? []).map(transformBuildingForUI);

  const handleSubmit = async (data: {
    title: string;
    description: string;
    commodity: ApiOutageCommodity;
    severity: ApiOutageSeverity;
    buildingIds: string[];
    scheduledStart: string;
    scheduledEnd: string;
  }) => {
    try {
      await createOutage.mutateAsync({
        title: data.title,
        description: data.description,
        commodity: data.commodity,
        severity: data.severity,
        buildingIds: data.buildingIds,
        scheduledStart: data.scheduledStart,
        scheduledEnd: data.scheduledEnd || undefined,
      });
      showToast({
        type: 'success',
        title: t('outages.createdSuccessfully'),
        message: t('outages.outageCreatedMsg'),
      });
      navigate('/outages');
    } catch (error) {
      showToast({
        type: 'error',
        title: t('outages.failedToCreate'),
        message: error instanceof Error ? error.message : t('auth.unexpectedError'),
      });
    }
  };

  const handleCancel = () => navigate('/outages');

  return (
    <CreateOutagePage
      buildings={buildings}
      isLoading={createOutage.isPending || isLoadingBuildings}
      onSubmit={handleSubmit}
      onCancel={handleCancel}
    />
  );
}

/**
 * Route wrapper for view outage page (UC-12).
 */
function ViewOutagePageRoute() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { outageId } = useParams<{ outageId: string }>();
  const { showToast } = useToast();

  const { data: outageData, isLoading } = useOutage(outageId ?? '');
  const startOutage = useStartOutage();
  const resolveOutage = useResolveOutage();
  const cancelOutage = useCancelOutage();

  if (!outageId) {
    return (
      <div className="error-page">
        <h1>{t('errors.outageNotFound')}</h1>
        <p>{t('errors.outageNotFoundDesc')}</p>
        <Link to="/outages">{t('common.backToOutages')}</Link>
      </div>
    );
  }

  const handleEdit = () => navigate(`/outages/${outageId}/edit`);

  const handleStart = async () => {
    try {
      await startOutage.mutateAsync({ id: outageId });
      showToast({ type: 'success', title: t('outages.started'), message: '' });
    } catch (error) {
      showToast({
        type: 'error',
        title: t('outages.failedToStart'),
        message: error instanceof Error ? error.message : t('auth.unexpectedError'),
      });
    }
  };

  const handleResolve = async (notes: string) => {
    try {
      await resolveOutage.mutateAsync({ id: outageId, data: { resolutionNotes: notes } });
      showToast({ type: 'success', title: t('outages.resolved'), message: '' });
    } catch (error) {
      showToast({
        type: 'error',
        title: t('outages.failedToResolve'),
        message: error instanceof Error ? error.message : t('auth.unexpectedError'),
      });
    }
  };

  const handleCancel = async (reason: string) => {
    try {
      await cancelOutage.mutateAsync({ id: outageId, data: { reason } });
      showToast({ type: 'success', title: t('outages.cancelled'), message: '' });
    } catch (error) {
      showToast({
        type: 'error',
        title: t('outages.failedToCancel'),
        message: error instanceof Error ? error.message : t('auth.unexpectedError'),
      });
    }
  };

  const handleBack = () => navigate('/outages');

  if (isLoading || !outageData) {
    return (
      <div className="flex items-center justify-center py-12">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
      </div>
    );
  }

  // Map API outage to UI OutageDetail format
  const outage: OutageDetail = {
    id: outageData.id,
    organizationId: outageData.organizationId,
    createdBy: outageData.createdBy,
    title: outageData.title,
    description: outageData.description ?? '',
    commodity: outageData.commodity,
    severity: outageData.severity,
    status: outageData.status,
    buildingIds: outageData.buildingIds,
    scheduledStart: outageData.scheduledStart,
    scheduledEnd: outageData.scheduledEnd,
    actualStart: outageData.actualStart,
    actualEnd: outageData.actualEnd,
    resolutionNotes: outageData.resolutionNotes,
    cancelReason: outageData.cancelReason,
    createdAt: outageData.createdAt,
    updatedAt: outageData.updatedAt,
    createdByName: outageData.creatorName ?? outageData.createdBy ?? 'Unknown',
    buildingNames: outageData.buildings?.map((b) => b.name) ?? [],
  };

  return (
    <ViewOutagePage
      outage={outage}
      isLoading={isLoading}
      onEdit={handleEdit}
      onStart={handleStart}
      onResolve={handleResolve}
      onCancel={handleCancel}
      onBack={handleBack}
    />
  );
}

/**
 * Route wrapper for edit outage page (UC-12).
 */
function EditOutagePageRoute() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { outageId } = useParams<{ outageId: string }>();
  const { showToast } = useToast();

  const { data: outageData, isLoading: isLoadingOutage } = useOutage(outageId ?? '');
  const updateOutage = useUpdateOutage();

  // Fetch buildings from API
  const { data: buildingsData, isLoading: isLoadingBuildings } = useBuildings();

  if (!outageId) {
    return (
      <div className="error-page">
        <h1>{t('errors.outageNotFound')}</h1>
        <p>{t('errors.outageNotFoundDesc')}</p>
        <Link to="/outages">{t('common.backToOutages')}</Link>
      </div>
    );
  }

  // Transform API buildings to UI format
  const buildings = (buildingsData?.items ?? []).map(transformBuildingForUI);

  const handleSubmit = async (data: {
    title: string;
    description: string;
    commodity: ApiOutageCommodity;
    severity: ApiOutageSeverity;
    buildingIds: string[];
    scheduledStart: string;
    scheduledEnd: string;
  }) => {
    try {
      await updateOutage.mutateAsync({
        id: outageId,
        data: {
          title: data.title,
          description: data.description,
          commodity: data.commodity,
          severity: data.severity,
          buildingIds: data.buildingIds,
          scheduledStart: data.scheduledStart,
          scheduledEnd: data.scheduledEnd || undefined,
        },
      });
      showToast({
        type: 'success',
        title: t('outages.updatedSuccessfully'),
        message: t('outages.outageUpdatedMsg'),
      });
      navigate(`/outages/${outageId}`);
    } catch (error) {
      showToast({
        type: 'error',
        title: t('outages.failedToUpdate'),
        message: error instanceof Error ? error.message : t('auth.unexpectedError'),
      });
    }
  };

  const handleCancel = () => navigate(`/outages/${outageId}`);

  if (isLoadingOutage || isLoadingBuildings || !outageData) {
    return (
      <div className="flex items-center justify-center py-12">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
      </div>
    );
  }

  // Map API outage to UI OutageDetail format
  const outage: OutageDetail = {
    id: outageData.id,
    organizationId: outageData.organizationId,
    createdBy: outageData.createdBy,
    title: outageData.title,
    description: outageData.description ?? '',
    commodity: outageData.commodity,
    severity: outageData.severity,
    status: outageData.status,
    buildingIds: outageData.buildingIds,
    scheduledStart: outageData.scheduledStart,
    scheduledEnd: outageData.scheduledEnd,
    actualStart: outageData.actualStart,
    actualEnd: outageData.actualEnd,
    resolutionNotes: outageData.resolutionNotes,
    cancelReason: outageData.cancelReason,
    createdAt: outageData.createdAt,
    updatedAt: outageData.updatedAt,
    createdByName: outageData.creatorName ?? outageData.createdBy ?? 'Unknown',
    buildingNames: outageData.buildings?.map((b) => b.name) ?? [],
  };

  return (
    <EditOutagePage
      outage={outage}
      buildings={buildings}
      isLoading={updateOutage.isPending}
      onSubmit={handleSubmit}
      onCancel={handleCancel}
    />
  );
}

/** Outages routes (UC-12). */
export function outageRoutes() {
  return (
    <>
      <Route path="/outages" element={<OutagesPageRoute />} />
      <Route path="/outages/new" element={<CreateOutagePageRoute />} />
      <Route path="/outages/:outageId" element={<ViewOutagePageRoute />} />
      <Route path="/outages/:outageId/edit" element={<EditOutagePageRoute />} />
    </>
  );
}
