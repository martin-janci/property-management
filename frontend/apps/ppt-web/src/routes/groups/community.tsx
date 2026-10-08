/**
 * Community route group (Epic 42).
 *
 * Owns the community route-wrapper components and the `<Route>` table fragment.
 * Extracted from App.tsx to isolate community work.
 */
import { useGroup, useGroupMembers, useJoinGroup, useLeaveGroup } from '@ppt/api-client';
import { useTranslation } from 'react-i18next';
import { Route, useNavigate, useParams } from 'react-router-dom';
import { Spinner, useToast } from '../../components';
import { useAuth } from '../../contexts';
import {
  CreateGroupPage,
  EventsPage,
  FeedPage,
  GroupDetailPage,
  GroupsPage,
  MarketplacePage,
} from '../lazyRoutes';

function FeedPageRoute() {
  const navigate = useNavigate();
  const { user } = useAuth();

  return (
    <FeedPage
      posts={[]}
      total={0}
      userName={user?.firstName ?? 'User'}
      onCreatePost={() => {}}
      onLikePost={() => {}}
      onViewPost={(id) => navigate(`/community/posts/${id}`)}
      onCommentPost={() => {}}
      onSharePost={() => {}}
      onEditPost={() => {}}
      onDeletePost={() => {}}
      onFilterChange={() => {}}
      onLoadMore={() => {}}
    />
  );
}

function GroupsPageRoute() {
  const navigate = useNavigate();

  return (
    <GroupsPage
      groups={[]}
      total={0}
      onNavigateToGroup={(id) => navigate(`/community/groups/${id}`)}
      onNavigateToCreate={() => navigate('/community/groups/new')}
      onJoinGroup={() => {}}
      onLeaveGroup={() => {}}
      onFilterChange={() => {}}
    />
  );
}

function CreateGroupPageRoute() {
  const navigate = useNavigate();
  const { showToast } = useToast();
  const { t } = useTranslation();

  return (
    <CreateGroupPage
      onSubmit={() => {
        showToast({
          type: 'success',
          title: t('common.created', { defaultValue: 'Created' }),
          message: t('community.groupCreated', { defaultValue: 'Group created' }),
        });
        navigate('/community/groups');
      }}
      onCancel={() => navigate('/community/groups')}
    />
  );
}

/**
 * Data-wired inner component for the group-detail route.
 *
 * Fetches the real group + members for `groupId` from the Community API and
 * derives the current viewer's membership/role from the member list. Renders a
 * loading spinner while fetching and a not-found state when the group is
 * missing or the request errors — never a fabricated placeholder group.
 *
 * Exported for the route-wiring test, which exercises the loading / not-found /
 * loaded branches and the join/leave wiring in isolation (mirrors
 * `ViewAnnouncementPageInner`).
 */
export function GroupDetailPageInner({ groupId }: { groupId: string }) {
  const navigate = useNavigate();
  const { t } = useTranslation();
  const { user } = useAuth();

  const { data: group, isLoading, error } = useGroup(groupId);
  const { data: members = [] } = useGroupMembers(groupId);
  const joinGroup = useJoinGroup();
  const leaveGroup = useLeaveGroup();

  if (isLoading) {
    return (
      <div className="flex justify-center py-16">
        <Spinner size="lg" />
      </div>
    );
  }

  if (error || !group) {
    return (
      <div className="max-w-4xl mx-auto px-4 py-16 text-center">
        <p className="text-gray-500">{t('errors.groupNotFound')}</p>
        <button
          type="button"
          onClick={() => navigate('/community/groups')}
          className="mt-4 text-sm text-blue-600 hover:text-blue-800"
        >
          {t('common.back')}
        </button>
      </div>
    );
  }

  const currentUserId = user?.id;
  const currentMember = currentUserId
    ? members.find((member) => member.userId === currentUserId)
    : undefined;
  const isMember = currentMember?.status === 'active';
  const isOwner = currentMember?.role === 'owner';
  const isAdmin = currentMember?.role === 'admin';

  return (
    <GroupDetailPage
      group={group}
      members={members}
      currentUserId={currentUserId}
      isMember={isMember}
      isAdmin={isAdmin}
      isOwner={isOwner}
      isJoining={joinGroup.isPending}
      isLeaving={leaveGroup.isPending}
      onNavigateBack={() => navigate('/community/groups')}
      onJoin={() => joinGroup.mutate(groupId)}
      onLeave={() => leaveGroup.mutate(groupId)}
      onEdit={() => {}}
      onDelete={() => navigate('/community/groups')}
      onNavigateToSettings={() => {}}
      onPromoteMember={() => {}}
      onRemoveMember={() => {}}
      onBanMember={() => {}}
    />
  );
}

function GroupDetailPageRoute() {
  const { groupId } = useParams<{ groupId: string }>();
  const { t } = useTranslation();

  if (!groupId) {
    return <div>{t('errors.groupNotFound', 'Group not found')}</div>;
  }

  return <GroupDetailPageInner groupId={groupId} />;
}

function EventsPageRoute() {
  const navigate = useNavigate();

  return (
    <EventsPage
      events={[]}
      total={0}
      onNavigateToEvent={(id) => navigate(`/community/events/${id}`)}
      onNavigateToCreate={() => navigate('/community/events/new')}
      onRsvp={() => {}}
      onEditEvent={() => {}}
      onDeleteEvent={() => {}}
      onExportCalendar={() => {}}
      onFilterChange={() => {}}
    />
  );
}

function MarketplacePageRoute() {
  const navigate = useNavigate();

  return (
    <MarketplacePage
      items={[]}
      total={0}
      onNavigateToItem={(id) => navigate(`/community/marketplace/${id}`)}
      onNavigateToCreate={() => navigate('/community/marketplace/new')}
      onContactSeller={() => {}}
      onEditItem={() => {}}
      onDeleteItem={() => {}}
      onMarkSold={() => {}}
      onFilterChange={() => {}}
    />
  );
}

/** Community routes (Epic 42). */
export function communityRoutes() {
  return (
    <>
      <Route path="/community" element={<FeedPageRoute />} />
      <Route path="/community/groups" element={<GroupsPageRoute />} />
      <Route path="/community/groups/new" element={<CreateGroupPageRoute />} />
      <Route path="/community/groups/:groupId" element={<GroupDetailPageRoute />} />
      <Route path="/community/events" element={<EventsPageRoute />} />
      <Route path="/community/marketplace" element={<MarketplacePageRoute />} />
    </>
  );
}
