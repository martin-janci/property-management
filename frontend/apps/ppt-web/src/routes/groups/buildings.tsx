/**
 * Buildings + Facilities route group (Epic 3).
 *
 * Owns the building route-wrapper components and the `<Route>` table fragment
 * for buildings + facilities. Extracted from App.tsx to isolate this work.
 */
import { useTranslation } from 'react-i18next';
import { Route, useNavigate, useParams } from 'react-router-dom';
import { ProtectedRoute } from '../../components';
import {
  BookFacilityPage,
  BuildingDetailPage,
  BuildingsPage,
  CreateFacilityPage,
  EditFacilityPage,
  FacilitiesPage,
  MyBookingsPage,
  MyUnitPage,
  PendingBookingsPage,
} from '../lazyRoutes';

/** Route wrapper for buildings list (Epic 3, Story 3.1) — gap-sweep */
function BuildingsPageRoute() {
  const navigate = useNavigate();
  return (
    <BuildingsPage
      onNavigateToView={(id) => navigate(`/buildings/${id}`)}
      onNavigateToEdit={(id) => navigate(`/buildings/${id}`)}
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
          <ProtectedRoute>
            <CreateFacilityPage />
          </ProtectedRoute>
        }
      />
      <Route
        path="/buildings/:buildingId/facilities/:facilityId/edit"
        element={
          <ProtectedRoute>
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
          <ProtectedRoute>
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
