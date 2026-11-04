/**
 * Voting route group (Epic 5: Building Voting & Decisions, FR37-44).
 *
 * Mounts the `features/voting/` pages into ppt-web. Unlike the meters/leases
 * groups, the `@ppt/api-client` voting module already ships, so these wrappers
 * are thin: the pages consume the voting hooks directly and the wrappers only
 * supply navigation callbacks and the `:voteId` route param.
 */

import { Route, useLocation, useNavigate, useParams } from 'react-router-dom';
import { ProtectedRoute } from '../../components';
import { VoteCreatePage, VoteDetailPage, VotingPage } from '../lazyRoutes';
import { MANAGER_ROLES } from '../shared';

/** Route wrapper: voting list (`/voting`). */
function VotingPageRoute() {
  const navigate = useNavigate();
  // Carry the selected building (`?building=`) into the create wizard.
  const { search } = useLocation();
  return (
    <VotingPage
      onOpenVote={(voteId) => navigate(`/voting/${voteId}`)}
      onCreateVote={() => navigate(`/voting/new${search}`)}
    />
  );
}

/** Route wrapper: create vote (`/voting/new`). */
function VoteCreatePageRoute() {
  const navigate = useNavigate();
  const { search } = useLocation();
  return (
    <VoteCreatePage
      onCreated={(voteId) => navigate(`/voting/${voteId}`)}
      onCancel={() => navigate(`/voting${search}`)}
    />
  );
}

/** Route wrapper: vote detail / ballot / administer (`/voting/:voteId`). */
function VoteDetailPageRoute() {
  const navigate = useNavigate();
  const { voteId } = useParams<{ voteId: string }>();
  return <VoteDetailPage voteId={voteId ?? ''} onBack={() => navigate('/voting')} />;
}

/** Voting routes (Epic 5, FR37-44). */
export function votingRoutes() {
  return (
    <>
      <Route
        path="/voting"
        element={
          <ProtectedRoute>
            <VotingPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/voting/new"
        element={
          <ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>
            <VoteCreatePageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/voting/:voteId"
        element={
          <ProtectedRoute>
            <VoteDetailPageRoute />
          </ProtectedRoute>
        }
      />
    </>
  );
}
