/**
 * Leases route group (Epic 19 — Lease Management & Tenant Screening).
 *
 * Mounts the already-built `features/leases/` pages into ppt-web. Until the
 * `@ppt/api-client` leases module lands (client-lag — see PAP-20 follow-up),
 * the list/dashboard/create pages render their designed empty states and the
 * entity-detail pages render a `PendingDataNotice` instead of fabricating a
 * stub entity. Wrappers are intentionally thin so swapping the empty fallbacks
 * for real TanStack Query calls is a localized edit per route — the same shape
 * the rentals group uses for not-yet-generated endpoints.
 */
import { lazy } from 'react';
import { Route, useNavigate } from 'react-router-dom';
import { ProtectedRoute } from '../../components';
import type { ExpirationOverview, LeaseStatistics } from '../../features/leases/types';

// Leases feature pages (Epic 19) — lazy-loaded for code splitting.
const LeasesDashboardPage = lazy(() =>
  import('../../features/leases').then((m) => ({ default: m.LeasesDashboardPage }))
);
const LeasesListPage = lazy(() =>
  import('../../features/leases').then((m) => ({ default: m.LeasesListPage }))
);
const CreateLeasePage = lazy(() =>
  import('../../features/leases').then((m) => ({ default: m.CreateLeasePage }))
);
const TemplatesPage = lazy(() =>
  import('../../features/leases').then((m) => ({ default: m.TemplatesPage }))
);
const ApplicationsPage = lazy(() =>
  import('../../features/leases').then((m) => ({ default: m.ApplicationsPage }))
);
const ViolationsPage = lazy(() =>
  import('../../features/leases').then((m) => ({ default: m.ViolationsPage }))
);
const CreateViolationPage = lazy(() =>
  import('../../features/leases').then((m) => ({ default: m.CreateViolationPage }))
);
// LeaseDetailPage / ApplicationDetailPage / ViolationDetailPage require a single
// fetched entity the generated client cannot yet provide; their routes render
// `PendingDataNotice` until the leases api-client module lands.

const EMPTY_LEASE_STATS: LeaseStatistics = {
  totalLeases: 0,
  activeLeases: 0,
  expiringLeases: 0,
  expiredLeases: 0,
  pendingApplications: 0,
  occupancyRate: 0,
  totalMonthlyRent: 0,
  overduePayments: 0,
  currency: 'EUR',
};

const EMPTY_EXPIRATION_OVERVIEW: ExpirationOverview = {
  expiringIn30Days: [],
  expiringIn60Days: [],
  expiringIn90Days: [],
  expired: [],
};

/**
 * Placeholder for routes that need a single fetched entity (lease /
 * application / violation) which the generated client cannot yet provide.
 * Keeps the route reachable and renders without crashing until the leases
 * api-client module lands.
 */
function PendingDataNotice({ onBack }: { onBack: () => void }) {
  return (
    <div className="mx-auto max-w-md py-16 text-center">
      <h1 className="text-lg font-semibold text-gray-900">Lease data unavailable</h1>
      <p className="mt-2 text-sm text-gray-500">
        This screen is wired into the app but is waiting on the leases API client. Detail data will
        appear once the client module ships.
      </p>
      <button
        type="button"
        onClick={onBack}
        className="mt-6 rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700"
      >
        Back to leases
      </button>
    </div>
  );
}

/** Route wrapper: leases dashboard. Renders zeroed stats until the api-client lands. */
function LeasesDashboardPageRoute() {
  const navigate = useNavigate();
  return (
    <LeasesDashboardPage
      statistics={EMPTY_LEASE_STATS}
      expirationOverview={EMPTY_EXPIRATION_OVERVIEW}
      onNavigateToLease={(id) => navigate(`/leases/${id}`)}
      onNavigateToLeases={() => navigate('/leases/list')}
      onNavigateToApplications={() => navigate('/leases/applications')}
      onNavigateToCreateLease={() => navigate('/leases/new')}
    />
  );
}

/** Route wrapper: leases list. Renders empty until the api-client lands. */
function LeasesListPageRoute() {
  const navigate = useNavigate();
  return (
    <LeasesListPage
      leases={[]}
      total={0}
      buildings={[]}
      onNavigateToCreate={() => navigate('/leases/new')}
      onNavigateToView={(id) => navigate(`/leases/${id}`)}
      onRenewLease={() => {}}
      onTerminateLease={() => {}}
      onFilterChange={() => {}}
    />
  );
}

/** Route wrapper: create lease. Renders empty selects until the api-client lands. */
function CreateLeasePageRoute() {
  const navigate = useNavigate();
  return (
    <CreateLeasePage
      units={[]}
      tenants={[]}
      templates={[]}
      onSubmit={() => {}}
      onCancel={() => navigate('/leases/list')}
    />
  );
}

/** Route wrapper: lease templates. Renders empty until the api-client lands. */
function TemplatesPageRoute() {
  return (
    <TemplatesPage
      templates={[]}
      onCreateTemplate={() => {}}
      onUpdateTemplate={() => {}}
      onDeleteTemplate={() => {}}
      onToggleActive={() => {}}
    />
  );
}

/** Route wrapper: applications list. Renders empty until the api-client lands. */
function ApplicationsPageRoute() {
  const navigate = useNavigate();
  return (
    <ApplicationsPage
      applications={[]}
      total={0}
      units={[]}
      onNavigateToView={(id) => navigate(`/leases/applications/${id}`)}
      onNavigateToReview={(id) => navigate(`/leases/applications/${id}`)}
      onFilterChange={() => {}}
    />
  );
}

/** Route wrapper: violations list. Renders empty until the api-client lands. */
function ViolationsPageRoute() {
  const navigate = useNavigate();
  return (
    <ViolationsPage
      violations={[]}
      leases={[]}
      onViewViolation={(id) => navigate(`/leases/violations/${id}`)}
      onResolveViolation={() => {}}
      onDisputeViolation={() => {}}
      onCreateViolation={() => navigate('/leases/violations/new')}
      onExportReport={() => {}}
    />
  );
}

/** Route wrapper: create violation. Renders empty until the api-client lands. */
function CreateViolationPageRoute() {
  const navigate = useNavigate();
  return (
    <CreateViolationPage
      leases={[]}
      onBack={() => navigate('/leases/violations')}
      onSubmit={() => {}}
    />
  );
}

/** Route wrapper: lease detail (`/leases/:leaseId`). Needs a fetched lease (pending client). */
function LeaseDetailPageRoute() {
  const navigate = useNavigate();
  return <PendingDataNotice onBack={() => navigate('/leases/list')} />;
}

/** Route wrapper: application detail (`/leases/applications/:applicationId`). Pending client. */
function ApplicationDetailPageRoute() {
  const navigate = useNavigate();
  return <PendingDataNotice onBack={() => navigate('/leases/applications')} />;
}

/** Route wrapper: violation detail (`/leases/violations/:violationId`). Pending client. */
function ViolationDetailPageRoute() {
  const navigate = useNavigate();
  return <PendingDataNotice onBack={() => navigate('/leases/violations')} />;
}

/** Leases routes (Epic 19). */
export function leaseRoutes() {
  return (
    <>
      <Route
        path="/leases"
        element={
          <ProtectedRoute>
            <LeasesDashboardPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/leases/list"
        element={
          <ProtectedRoute>
            <LeasesListPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/leases/new"
        element={
          <ProtectedRoute>
            <CreateLeasePageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/leases/templates"
        element={
          <ProtectedRoute>
            <TemplatesPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/leases/applications"
        element={
          <ProtectedRoute>
            <ApplicationsPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/leases/applications/:applicationId"
        element={
          <ProtectedRoute>
            <ApplicationDetailPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/leases/violations"
        element={
          <ProtectedRoute>
            <ViolationsPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/leases/violations/new"
        element={
          <ProtectedRoute>
            <CreateViolationPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/leases/violations/:violationId"
        element={
          <ProtectedRoute>
            <ViolationDetailPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/leases/:leaseId"
        element={
          <ProtectedRoute>
            <LeaseDetailPageRoute />
          </ProtectedRoute>
        }
      />
    </>
  );
}
