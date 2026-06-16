/**
 * Application route table.
 *
 * Composes the per-feature route-group fragments into a single `<Routes>`.
 * Each `*Routes()` helper lives in `routes/groups/<feature>.tsx` and owns its
 * own route-wrapper components + API↔UI mappers, so concurrent feature PRs edit
 * their own group file instead of colliding on this central aggregator.
 *
 * Each group helper returns a `<React.Fragment>` of `<Route>` elements; React
 * Router traverses fragment children of `<Routes>`, so calling them inline here
 * keeps every `<Route>` a recognised direct child of `<Routes>` (behaviour is
 * identical to the previous inline table).
 */
import { Route, Routes } from 'react-router-dom';
import { accountingRoutes } from './groups/accounting';
import { announcementRoutes } from './groups/announcements';
import { buildingRoutes } from './groups/buildings';
import { communityRoutes } from './groups/community';
import { authRoutes, dashboardRoutes, errorRoutes, Home, settingsRoutes } from './groups/core';
import { disputeRoutes } from './groups/disputes';
import { documentRoutes, newsRoutes } from './groups/documents';
import { faultRoutes } from './groups/faults';
import { financialRoutes } from './groups/financial';
import { iotRoutes } from './groups/iot';
import { leaseRoutes } from './groups/leases';
import { messagingRoutes } from './groups/messaging';
import { meterRoutes } from './groups/meters';
import { neighborRoutes } from './groups/neighbors';
import { outageRoutes } from './groups/outages';
import { rentalRoutes } from './groups/rentals';
import { reportRoutes } from './groups/reports';
import { votingRoutes } from './groups/voting';

export function AppRoutes() {
  return (
    <Routes>
      <Route path="/" element={<Home />} />
      {authRoutes()}
      {dashboardRoutes()}
      {documentRoutes()}
      {newsRoutes()}
      {disputeRoutes()}
      {outageRoutes()}
      {announcementRoutes()}
      {messagingRoutes()}
      {neighborRoutes()}
      {faultRoutes()}
      {communityRoutes()}
      {financialRoutes()}
      {accountingRoutes()}
      {rentalRoutes()}
      {meterRoutes()}
      {leaseRoutes()}
      {votingRoutes()}
      {iotRoutes()}
      {reportRoutes()}
      {buildingRoutes()}
      {settingsRoutes()}
      {/* Phase 5 admin is at admin.rlt.sk now; see frontend/apps/admin-web. */}
      {errorRoutes()}
    </Routes>
  );
}
