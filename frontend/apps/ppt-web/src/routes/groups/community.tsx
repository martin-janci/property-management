/**
 * Community route group (Epic 42).
 *
 * Owns the `<Route>` table fragment for the Community section. Every page is
 * self-contained and data-wired (see `features/community`): lists are scoped
 * to the building selected via `?building=`, and every action calls a real
 * api-server Community endpoint. Routes exist only for screens the server can
 * back — there is no event-detail or post-detail endpoint, so neither has a
 * route.
 */
import { Route } from 'react-router-dom';
import { ProtectedRoute } from '../../components';
import {
  CreateCommunityEventPage,
  CreateGroupPage,
  CreateMarketplaceItemPage,
  EventsPage,
  FeedPage,
  GroupDetailPage,
  GroupsPage,
  MarketplaceItemDetailPage,
  MarketplacePage,
} from '../lazyRoutes';

/**
 * Community routes (Epic 42).
 *
 * Auth-only (not role-gated): the community feed, groups, events and
 * marketplace are resident-accessible features, so every route is wrapped in
 * `<ProtectedRoute>` — matching the guarded peer groups and the per-user
 * `/settings/privacy` precedent. An anonymous visitor is redirected to /login
 * (with return-URL capture) instead of rendering the page chrome.
 */
export function communityRoutes() {
  return (
    <>
      <Route
        path="/community"
        element={
          <ProtectedRoute>
            <FeedPage />
          </ProtectedRoute>
        }
      />
      <Route
        path="/community/groups"
        element={
          <ProtectedRoute>
            <GroupsPage />
          </ProtectedRoute>
        }
      />
      <Route
        path="/community/groups/new"
        element={
          <ProtectedRoute>
            <CreateGroupPage />
          </ProtectedRoute>
        }
      />
      <Route
        path="/community/groups/:groupId"
        element={
          <ProtectedRoute>
            <GroupDetailPage />
          </ProtectedRoute>
        }
      />
      <Route
        path="/community/events"
        element={
          <ProtectedRoute>
            <EventsPage />
          </ProtectedRoute>
        }
      />
      <Route
        path="/community/events/new"
        element={
          <ProtectedRoute>
            <CreateCommunityEventPage />
          </ProtectedRoute>
        }
      />
      <Route
        path="/community/marketplace"
        element={
          <ProtectedRoute>
            <MarketplacePage />
          </ProtectedRoute>
        }
      />
      <Route
        path="/community/marketplace/new"
        element={
          <ProtectedRoute>
            <CreateMarketplaceItemPage />
          </ProtectedRoute>
        }
      />
      <Route
        path="/community/marketplace/:itemId"
        element={
          <ProtectedRoute>
            <MarketplaceItemDetailPage />
          </ProtectedRoute>
        }
      />
    </>
  );
}
