/**
 * Buildings + Facilities route group (Epic 3).
 *
 * Owns the building route-wrapper components and the `<Route>` table fragment
 * for buildings + facilities. Extracted from App.tsx to isolate this work.
 */
import { useTranslation } from 'react-i18next';
import { Route, useNavigate, useParams } from 'react-router-dom';
import { ProtectedRoute } from '../../components';
import { useAuth } from '../../contexts';
import {
  BookFacilityPage,
  BuildingDetailPage,
  BuildingFormPage,
  BuildingsPage,
  CreateFacilityPage,
  EditFacilityPage,
  FacilitiesPage,
  MyBookingsPage,
  MyUnitPage,
  PendingBookingsPage,
} from '../lazyRoutes';
import { isManagerRole, MANAGER_ROLES } from '../shared';

/** Route wrapper for buildings list (Epic 3, Story 3.1) — gap-sweep */
function BuildingsPageRoute() {
  const navigate = useNavigate();
  const { user } = useAuth();
  const isManager = isManagerRole(user?.role);
  return (
    <BuildingsPage
      onNavigateToView={(id) => navigate(`/buildings/${id}`)}
      onNavigateToEdit={isManager ? (id) => navigate(`/buildings/${id}/edit`) : undefined}
      onNavigateToCreate={isManager ? () => navigate('/buildings/new') : undefined}
    />
  );
}

/** Create / edit building (manager only). */
function BuildingFormPageRoute() {
  const { buildingId } = useParams<{ buildingId?: string }>();
  const navigate = useNavigate();
  return (
    <BuildingFormPage
      buildingId={buildingId}
      onSaved={(id) => navigate(`/buildings/${id}`)}
      onCancel={() => navigate(buildingId ? `/buildings/${buildingId}` : '/buildings')}
    />
  );
}

/** Route wrapper for building detail (Epic 3, Story 3.1) — gap-sweep */
function BuildingDetailPageRoute() {
  const { buildingId } = useParams<{ buildingId: string }>();
  const { t } = useTranslation();
  if (!buildingId) {
    return <div>{t('errors.buildingNotFound', 'Building not found')}</div>;
  }
  return <BuildingDetailPage buildingId={buildingId} />;
}

/** Buildings + facilities routes (Epic 3, Stories 3.1 / 3.7). */
export function buildingRoutes() {
  return (
    <>
      <Route
        path="/buildings"
        element={
          <ProtectedRoute>
            <BuildingsPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/buildings/new"
        element={
          <ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>
            <BuildingFormPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/buildings/:buildingId/edit"
        element={
          <ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>
            <BuildingFormPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/buildings/:buildingId"
        element={
          <ProtectedRoute>
            <BuildingDetailPageRoute />
          </ProtectedRoute>
        }
      />
      {/* Resident "My Unit" view (Epic 3, Story 3.6) — gap-sweep.
        Self-resolves the caller's units via useMyUnits(); no params. */}
      <Route
        path="/my-unit"
        element={
          <ProtectedRoute>
            <MyUnitPage />
          </ProtectedRoute>
        }
      />
      {/* Facilities & bookings (Epic 3, Story 3.7) — gap-sweep.
        Pages self-resolve params via useParams/useNavigate. */}
      <Route
        path="/buildings/:buildingId/facilities"
        element={
          <ProtectedRoute>
            <FacilitiesPage />
          </ProtectedRoute>
        }
      />
      <Route
        path="/buildings/:buildingId/facilities/new"
        element={
          <ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>
            <CreateFacilityPage />
          </ProtectedRoute>
        }
      />
      <Route
        path="/buildings/:buildingId/facilities/:facilityId/edit"
        element={
          <ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>
            <EditFacilityPage />
          </ProtectedRoute>
        }
      />
      <Route
        path="/buildings/:buildingId/facilities/:facilityId/book"
        element={
          <ProtectedRoute>
            <BookFacilityPage />
          </ProtectedRoute>
        }
      />
      <Route
        path="/buildings/:buildingId/facilities/pending"
        element={
          <ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>
            <PendingBookingsPage />
          </ProtectedRoute>
        }
      />
      <Route
        path="/facilities/bookings/my"
        element={
          <ProtectedRoute>
            <MyBookingsPage />
          </ProtectedRoute>
        }
      />
    </>
  );
}
