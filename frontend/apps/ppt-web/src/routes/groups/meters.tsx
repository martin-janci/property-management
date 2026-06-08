/**
 * Meters route group (Epic 12, UC — self-readings & consumption).
 *
 * Mounts the already-built `features/meters/` pages into ppt-web. Until the
 * `@ppt/api-client` meters module lands (client-lag — see PAP-20 follow-up),
 * the list/dashboard pages render their designed empty states and the
 * entity-detail/form pages render a `PendingDataNotice` instead of fabricating
 * a stub `Meter`. Wrappers are intentionally thin so swapping the empty
 * fallbacks for real TanStack Query calls is a localized edit per route — the
 * same shape the rentals group uses for not-yet-generated endpoints.
 */
import { lazy } from 'react';
import { Route, useNavigate } from 'react-router-dom';
import { ProtectedRoute } from '../../components';

// Meters feature pages (Epic 12) — lazy-loaded for code splitting.
const MetersPage = lazy(() =>
  import('../../features/meters').then((m) => ({ default: m.MetersPage }))
);
const PendingValidationsPage = lazy(() =>
  import('../../features/meters').then((m) => ({ default: m.PendingValidationsPage }))
);
const ReadingComparisonPage = lazy(() =>
  import('../../features/meters').then((m) => ({ default: m.ReadingComparisonPage }))
);
// MeterDetailPage / SubmitReadingPage / EditReadingPage require a fetched
// `Meter` (and reading) the generated client cannot yet provide; their routes
// render `PendingDataNotice` until the meters api-client module lands, so they
// are intentionally not imported here yet.

/**
 * Placeholder for routes that need a single fetched entity (meter / reading)
 * which the generated client cannot yet provide. Keeps the route reachable and
 * renders without crashing until the meters api-client module lands.
 */
function PendingDataNotice({ onBack }: { onBack: () => void }) {
  return (
    <div className="mx-auto max-w-md py-16 text-center">
      <h1 className="text-lg font-semibold text-gray-900">Meter data unavailable</h1>
      <p className="mt-2 text-sm text-gray-500">
        This screen is wired into the app but is waiting on the meters API client. Detail data
        will appear once the client module ships.
      </p>
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

/** Route wrapper: meters list (Epic 12). Renders empty until the api-client lands. */
function MetersPageRoute() {
  const navigate = useNavigate();
  return (
    <MetersPage
      meters={[]}
      onNavigateToDetail={(meterId) => navigate(`/meters/${meterId}`)}
      onNavigateToSubmitReading={(meterId) => navigate(`/meters/${meterId}/submit`)}
      onNavigateToComparison={() => navigate('/meters/comparison')}
    />
  );
}

/** Route wrapper: single meter detail (`/meters/:meterId`). Needs a fetched Meter (pending client). */
function MeterDetailPageRoute() {
  const navigate = useNavigate();
  return <PendingDataNotice onBack={() => navigate('/meters')} />;
}

/** Route wrapper: submit a reading for a meter. Needs a fetched Meter (pending client). */
function SubmitReadingPageRoute() {
  const navigate = useNavigate();
  return <PendingDataNotice onBack={() => navigate('/meters')} />;
}

/** Route wrapper: edit an existing reading. Needs a fetched Meter + reading (pending client). */
function EditReadingPageRoute() {
  const navigate = useNavigate();
  return <PendingDataNotice onBack={() => navigate('/meters')} />;
}

/** Route wrapper: manager pending-validations queue. Renders empty until the api-client lands. */
function PendingValidationsPageRoute() {
  const navigate = useNavigate();
  return <PendingValidationsPage readings={[]} onValidate={() => {}} onBack={() => navigate('/meters')} />;
}

/** Route wrapper: cross-meter consumption comparison. Renders empty until the api-client lands. */
function ReadingComparisonPageRoute() {
  const navigate = useNavigate();
  return (
    <ReadingComparisonPage
      meters={[]}
      comparisonData={[]}
      onBack={() => navigate('/meters')}
      onCompare={() => {}}
    />
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
