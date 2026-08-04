/**
 * Community route group (Epic 42).
 *
 * Owns the community route-wrapper components and the `<Route>` table fragment.
 * Extracted from App.tsx to isolate community work.
 */
import type { CommunityGroup } from '@ppt/api-client';
import { useTranslation } from 'react-i18next';
import { Route, useNavigate, useParams } from 'react-router-dom';
import { useToast } from '../../components';
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

  return (
    <CreateGroupPage
      onSubmit={() => {
        showToast({ type: 'success', title: 'Created', message: 'Group created' });
        navigate('/community/groups');
      }}
      onCancel={() => navigate('/community/groups')}
    />
  );
}

function GroupDetailPageRoute() {
  const { groupId } = useParams<{ groupId: string }>();
  const navigate = useNavigate();
  const { t } = useTranslation();

  if (!groupId) {
    return <div>{t('errors.groupNotFound', 'Group not found')}</div>;
  }

  // Mock group data
  const mockGroup: CommunityGroup = {
    id: groupId,
    buildingId: 'bld-1',
    name: 'Sample Group',
    description: 'A sample community group',
    category: 'general',
    visibility: 'public',
    memberCount: 10,
    postCount: 0,
    isOfficial: false,
    createdBy: 'user-1',
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  };

  return (
    <GroupDetailPage
      group={mockGroup}
      members={[]}
      onNavigateBack={() => navigate('/community/groups')}
      onJoin={() => {}}
      onLeave={() => {}}
      onEdit={() => {}}
      onDelete={() => navigate('/community/groups')}
      onNavigateToSettings={() => {}}
      onPromoteMember={() => {}}
      onRemoveMember={() => {}}
      onBanMember={() => {}}
    />
  );
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
