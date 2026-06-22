/**
 * Delegations route group (Epic 3, Story 3.4).
 *
 * Mounts `features/delegations/` into ppt-web. The `@ppt/api-client` delegation
 * module ships its own hooks, so the page consumes them directly and this
 * wrapper only guards the route behind authentication.
 */
import { Route } from 'react-router-dom';
import { ProtectedRoute } from '../../components';
import { DelegationsPage } from '../lazyRoutes';

/** Delegation routes (Epic 3, Story 3.4). */
export function delegationRoutes() {
  return (
    <>
      <Route
        path="/delegations"
        element={
          <ProtectedRoute>
            <DelegationsPage />
          </ProtectedRoute>
        }
      />
    </>
  );
}
