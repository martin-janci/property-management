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

/** Community routes (Epic 42). */
export function communityRoutes() {
  return (
    <>
      <Route path="/community" element={<FeedPage />} />
      <Route path="/community/groups" element={<GroupsPage />} />
      <Route path="/community/groups/new" element={<CreateGroupPage />} />
      <Route path="/community/groups/:groupId" element={<GroupDetailPage />} />
      <Route path="/community/events" element={<EventsPage />} />
      <Route path="/community/events/new" element={<CreateCommunityEventPage />} />
      <Route path="/community/marketplace" element={<MarketplacePage />} />
      <Route path="/community/marketplace/new" element={<CreateMarketplaceItemPage />} />
      <Route path="/community/marketplace/:itemId" element={<MarketplaceItemDetailPage />} />
    </>
  );
}
