/**
 * Voting route group (Epic 5: Building Voting & Decisions, FR37-44).
 *
 * Mounts the `features/voting/` pages into ppt-web. Unlike the meters/leases
 * groups, the `@ppt/api-client` voting module already ships, so these wrappers
 * are thin: the pages consume the voting hooks directly and the wrappers only
 * supply navigation callbacks and the `:voteId` route param.
 */
import { Route, useNavigate, useParams } from 'react-router-dom';
import { ProtectedRoute } from '../../components';
import { VoteCreatePage, VoteDetailPage, VotingPage } from '../lazyRoutes';

/** Route wrapper: voting list (`/voting`). */
function VotingPageRoute() {
  const navigate = useNavigate();
  return (
    <VotingPage
      onOpenVote={(voteId) => navigate(`/voting/${voteId}`)}
      onCreateVote={() => navigate('/voting/new')}
    />
  );
}

/** Route wrapper: create vote (`/voting/new`). */
function VoteCreatePageRoute() {
  const navigate = useNavigate();
  return (
    <VoteCreatePage
      onCreated={(voteId) => navigate(`/voting/${voteId}`)}
      onCancel={() => navigate('/voting')}
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
          <ProtectedRoute>
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
