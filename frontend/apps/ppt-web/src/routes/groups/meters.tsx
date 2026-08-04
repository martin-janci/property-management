/**
 * Meters route group (Epic 12, UC — self-readings & consumption).
 *
 * Mounts the already-built `features/meters/` pages into ppt-web, wired to the
 * `@ppt/api-client` meters module (PAP-30). This group owns the API↔UI mappers
 * (backend wire shapes are snake_case with Decimal-as-string) and the thin
 * route-wrapper components.
 *
 * The backend has no "all meters" endpoint — listing is per-building
 * (`/meters/buildings/{id}`) — so the list/comparison wrappers fan out one
 * query per building (via `useQueries`) and flatten. Detail/submit/edit/pending
 * wrappers fetch their single entity and render it live.
 */
import type {
  Meter as ApiMeter,
  MeterReading as ApiMeterReading,
  MeterType as ApiMeterType,
  ReadingStatus as ApiReadingStatus,
} from '@ppt/api-client';
import {
  listMetersForBuilding,
  meterKeys,
  useBuildings,
  useMeter,
  usePendingReadings,
  useReading,
  useSubmitReading,
  useValidateReading,
} from '@ppt/api-client';
import { useQueries } from '@tanstack/react-query';
import { lazy } from 'react';
import { useTranslation } from 'react-i18next';
import { Route, useNavigate, useParams } from 'react-router-dom';
import { ProtectedRoute, useToast } from '../../components';
import type {
  Meter as UiMeter,
  MeterReading as UiMeterReading,
  MeterType as UiMeterType,
  MeterUnit as UiMeterUnit,
  ReadingStatus as UiReadingStatus,
  ValidationResult,
} from '../../features/meters/types';
import { useOrganization } from '../../hooks/useOrganization';

// Meters feature pages (Epic 12) — lazy-loaded for code splitting.
const MetersPage = lazy(() =>
  import('../../features/meters').then((m) => ({ default: m.MetersPage }))
);
const MeterDetailPage = lazy(() =>
  import('../../features/meters').then((m) => ({ default: m.MeterDetailPage }))
);
const SubmitReadingPage = lazy(() =>
  import('../../features/meters').then((m) => ({ default: m.SubmitReadingPage }))
);
const EditReadingPage = lazy(() =>
  import('../../features/meters').then((m) => ({ default: m.EditReadingPage }))
);
const PendingValidationsPage = lazy(() =>
  import('../../features/meters').then((m) => ({ default: m.PendingValidationsPage }))
);
const ReadingComparisonPage = lazy(() =>
  import('../../features/meters').then((m) => ({ default: m.ReadingComparisonPage }))
);

// ============================================================================
// API → UI mappers
// ============================================================================

/**
 * Map the backend `MeterType` (which has `solar`/`other` the UI doesn't model)
 * to the UI enum, falling back to `electricity` for the unmodelled variants.
 */
function mapMeterType(type: ApiMeterType): UiMeterType {
  switch (type) {
    case 'electricity':
    case 'gas':
    case 'water':
    case 'heat':
    case 'cold_water':
    case 'hot_water':
      return type;
    default:
      return 'electricity';
  }
}

/**
 * Map the backend `ReadingStatus` (`pending|approved|rejected|estimated`) to the
 * UI status (`pending|validated|rejected|corrected`). `approved` → `validated`;
 * `estimated` has no UI peer, so it falls back to `pending`.
 */
function mapReadingStatus(status: ApiReadingStatus): UiReadingStatus {
  switch (status) {
    case 'approved':
      return 'validated';
    case 'rejected':
      return 'rejected';
    case 'pending':
    case 'estimated':
      return 'pending';
    default:
      return 'pending';
  }
}

/** Transform an API meter (snake_case, Decimal-as-string) → UI Meter (camelCase). */
function mapApiMeterToUi(meter: ApiMeter): UiMeter {
  return {
    id: meter.id,
    organizationId: meter.organization_id,
    buildingId: meter.building_id,
    unitId: meter.unit_id ?? undefined,
    meterType: mapMeterType(meter.meter_type),
    serialNumber: meter.meter_number,
    unit: meter.unit_of_measure as UiMeterUnit,
    location: meter.location ?? undefined,
    installationDate: meter.installed_at ?? undefined,
    lastReadingValue: meter.current_reading != null ? Number(meter.current_reading) : undefined,
    lastReadingDate: meter.last_reading_date ?? undefined,
    isActive: meter.is_active,
    createdAt: meter.created_at,
    updatedAt: meter.updated_at,
  };
}

/** Transform an API reading (snake_case, Decimal-as-string) → UI MeterReading. */
function mapApiReadingToUi(reading: ApiMeterReading): UiMeterReading {
  return {
    id: reading.id,
    meterId: reading.meter_id,
    // The wire shape carries no org on a reading; the UI field is required so
    // we leave it empty (it is not surfaced in the reading views).
    organizationId: '',
    value: Number(reading.reading),
    readingDate: reading.reading_date,
    photoUrl: reading.photo_url ?? undefined,
    status: mapReadingStatus(reading.status),
    submittedById: reading.submitted_by ?? '',
    submittedAt: reading.created_at,
    validatedById: reading.validated_by ?? undefined,
    validatedAt: reading.validated_at ?? undefined,
    rejectionReason:
      reading.status === 'rejected' ? (reading.validation_notes ?? undefined) : undefined,
    notes: reading.validation_notes ?? undefined,
    createdAt: reading.created_at,
    updatedAt: reading.updated_at,
  };
}

/** Map a UI validation status → the backend `ReadingStatus`. */
function mapValidationStatus(status: ValidationResult['status']): ApiReadingStatus {
  // `corrected` is recorded as an approval that carries a corrected value.
  return status === 'rejected' ? 'rejected' : 'approved';
}

// ============================================================================
// Route wrappers
// ============================================================================

/**
 * Fetch meters across every building the user can see and flatten them. The
 * backend lists meters per building, so we fan out one query per building.
 */
function useAllMeters() {
  const { data: buildingsData } = useBuildings();
  const buildingIds = (buildingsData?.items ?? []).map((b) => b.id);
  const results = useQueries({
    queries: buildingIds.map((id) => ({
      queryKey: meterKeys.buildingList(id, {}),
      queryFn: () => listMetersForBuilding(id, {}),
    })),
  });
  const meters = results.flatMap((r) => r.data?.meters ?? []).map(mapApiMeterToUi);
  const isLoading = buildingIds.length > 0 && results.some((r) => r.isLoading);
  return { meters, isLoading };
}

/** Route wrapper: meters list (Epic 12), live across all buildings. */
function MetersPageRoute() {
  const navigate = useNavigate();
  const { meters, isLoading } = useAllMeters();
  return (
    <MetersPage
      meters={meters}
      isLoading={isLoading}
      onNavigateToDetail={(meterId) => navigate(`/meters/${meterId}`)}
      onNavigateToSubmitReading={(meterId) => navigate(`/meters/${meterId}/submit`)}
      onNavigateToComparison={() => navigate('/meters/comparison')}
    />
  );
}

/** Route wrapper: single meter detail (`/meters/:meterId`), live. */
function MeterDetailPageRoute() {
  const navigate = useNavigate();
  const { meterId } = useParams<{ meterId: string }>();
  const { data, isLoading } = useMeter(meterId);

  if (isLoading || !data) {
    return <MeterLoadingNotice onBack={() => navigate('/meters')} isLoading={isLoading} />;
  }

  const meter = mapApiMeterToUi(data.meter);
  const readings = data.recent_readings.map(mapApiReadingToUi);
  return (
    <MeterDetailPage
      meter={meter}
      readings={readings}
      onBack={() => navigate('/meters')}
      onSubmitReading={() => navigate(`/meters/${meter.id}/submit`)}
      onViewReading={(readingId) => navigate(`/meters/${meter.id}/readings/${readingId}/edit`)}
      onEditReading={(readingId) => navigate(`/meters/${meter.id}/readings/${readingId}/edit`)}
    />
  );
}

/** Route wrapper: submit a reading for a meter, live. */
function SubmitReadingPageRoute() {
  const navigate = useNavigate();
  const { showToast } = useToast();
  const { meterId } = useParams<{ meterId: string }>();
  const { data, isLoading } = useMeter(meterId);
  const submitReading = useSubmitReading();

  if (isLoading || !data) {
    return <MeterLoadingNotice onBack={() => navigate('/meters')} isLoading={isLoading} />;
  }

  const meter = mapApiMeterToUi(data.meter);
  return (
    <SubmitReadingPage
      meter={meter}
      isSubmitting={submitReading.isPending}
      onCancel={() => navigate(`/meters/${meter.id}`)}
      onSubmit={async (formData) => {
        try {
          await submitReading.mutateAsync({
            meter_id: meter.id,
            reading: formData.value,
            reading_date: formData.readingDate,
            source: 'manual',
          });
          showToast({ type: 'success', title: 'Submitted', message: 'Reading submitted' });
          navigate(`/meters/${meter.id}`);
        } catch (err) {
          showToast({
            type: 'error',
            title: 'Submit failed',
            message: err instanceof Error ? err.message : 'Could not submit reading',
          });
        }
      }}
    />
  );
}

/** Route wrapper: edit an existing reading, live (resubmits a corrected reading). */
function EditReadingPageRoute() {
  const navigate = useNavigate();
  const { showToast } = useToast();
  const { meterId, readingId } = useParams<{ meterId: string; readingId: string }>();
  const meterQuery = useMeter(meterId);
  const readingQuery = useReading(readingId);
  const submitReading = useSubmitReading();

  if (meterQuery.isLoading || readingQuery.isLoading || !meterQuery.data || !readingQuery.data) {
    const isLoading = meterQuery.isLoading || readingQuery.isLoading;
    return (
      <MeterLoadingNotice
        onBack={() => navigate(`/meters/${meterId ?? ''}`)}
        isLoading={isLoading}
      />
    );
  }

  const meter = mapApiMeterToUi(meterQuery.data.meter);
  const reading = mapApiReadingToUi(readingQuery.data);
  return (
    <EditReadingPage
      meter={meter}
      reading={reading}
      isSubmitting={submitReading.isPending}
      onCancel={() => navigate(`/meters/${meter.id}`)}
      onSubmit={async (formData) => {
        try {
          // No PUT-reading endpoint exists; a correction is recorded as a new
          // reading carrying the revised value (server supersedes the prior one).
          await submitReading.mutateAsync({
            meter_id: meter.id,
            reading: formData.value,
            reading_date: formData.readingDate,
            source: 'manual',
          });
          showToast({
            type: 'success',
            title: 'Updated',
            message: 'Reading correction submitted',
          });
          navigate(`/meters/${meter.id}`);
        } catch (err) {
          showToast({
            type: 'error',
            title: 'Update failed',
            message: err instanceof Error ? err.message : 'Could not update reading',
          });
        }
      }}
    />
  );
}

/** Route wrapper: manager pending-validations queue, live. */
function PendingValidationsPageRoute() {
  const navigate = useNavigate();
  const { showToast } = useToast();
  const { organizationId } = useOrganization();
  const { data, isLoading } = usePendingReadings({ organization_id: organizationId });
  const validateReading = useValidateReading();

  const readings = (data ?? []).map(mapApiReadingToUi);
  return (
    <PendingValidationsPage
      readings={readings}
      isLoading={isLoading}
      isProcessing={validateReading.isPending}
      onBack={() => navigate('/meters')}
      onValidate={async (result: ValidationResult) => {
        try {
          await validateReading.mutateAsync({
            id: result.readingId,
            body: {
              status: mapValidationStatus(result.status),
              corrected_reading: result.correctedValue,
              notes: result.rejectionReason ?? result.notes,
            },
          });
          showToast({ type: 'success', title: 'Done', message: 'Reading validated' });
        } catch (err) {
          showToast({
            type: 'error',
            title: 'Validation failed',
            message: err instanceof Error ? err.message : 'Could not validate reading',
          });
        }
      }}
    />
  );
}

/** Route wrapper: cross-meter consumption comparison. Meter selector is live. */
function ReadingComparisonPageRoute() {
  const navigate = useNavigate();
  const { meters, isLoading } = useAllMeters();
  return (
    <ReadingComparisonPage
      meters={meters}
      // The chart series are fetched per selected meter+range once the user runs
      // a comparison; that wiring is tracked as a meters follow-up. The meter
      // selector itself is live.
      comparisonData={[]}
      isLoading={isLoading}
      onBack={() => navigate('/meters')}
      onCompare={() => {}}
    />
  );
}

/**
 * Lightweight loading / not-found notice for single-entity routes while their
 * fetch is in flight (or when the entity is missing).
 */
function MeterLoadingNotice({ onBack, isLoading }: { onBack: () => void; isLoading?: boolean }) {
  const { t } = useTranslation();
  return (
    <div className="mx-auto max-w-md py-16 text-center">
      <h1 className="text-lg font-semibold text-gray-900">
        {isLoading ? 'Loading meter…' : t('errors.meterNotFound', 'Meter not found')}
      </h1>
      {!isLoading && (
        <p className="mt-2 text-sm text-gray-500">
          This meter could not be loaded. It may have been removed or you may not have access.
        </p>
      )}
      <button
        type="button"
        onClick={onBack}
        className="mt-6 rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700"
      >
        Back to meters
      </button>
    </div>
  );
}

/** Meters routes (Epic 12). */
export function meterRoutes() {
  return (
    <>
      <Route
        path="/meters"
        element={
          <ProtectedRoute>
            <MetersPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/meters/comparison"
        element={
          <ProtectedRoute>
            <ReadingComparisonPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/meters/validations"
        element={
          <ProtectedRoute>
            <PendingValidationsPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/meters/:meterId"
        element={
          <ProtectedRoute>
            <MeterDetailPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/meters/:meterId/submit"
        element={
          <ProtectedRoute>
            <SubmitReadingPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/meters/:meterId/readings/:readingId/edit"
        element={
          <ProtectedRoute>
            <EditReadingPageRoute />
          </ProtectedRoute>
        }
      />
    </>
  );
}
