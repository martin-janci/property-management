/**
 * GroupsPage — `/community/groups`.
 *
 * Groups of the selected building with the caller's membership
 * (`GET /community/buildings/{id}/groups`); Join / Leave call
 * `POST /community/groups/{id}/join|leave`. The server returns the whole list,
 * so search and filters run client-side.
 */

import {
  type CommunityGroupType,
  type CommunityGroupWithMembership,
  useGroups,
  useJoinGroup,
  useLeaveGroup,
} from '@ppt/api-client';
import { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { useToast } from '../../../components';
import { getErrorMessage } from '../../../lib/api';
import { CommunityLayout } from '../components/CommunityLayout';
import { GroupCard } from '../components/GroupCard';
import { EmptyState, ErrorState, LoadingState } from '../components/StateViews';
import { useCommunityBuilding, withBuilding } from '../hooks/useCommunityBuilding';

export const GROUP_TYPES: CommunityGroupType[] = [
  'interest',
  'floor',
  'building',
  'neighborhood',
  'official',
];

export function GroupsPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const building = useCommunityBuilding();

  return (
    <CommunityLayout
      active="groups"
      title={t('community.groupList.title')}
      subtitle={t('community.groupList.subtitle')}
      building={building}
      actions={
        <button
          type="button"
          onClick={() => navigate(withBuilding('/community/groups/new', building.buildingId))}
          className="px-4 py-2 text-sm font-medium bg-blue-600 text-white rounded-md hover:bg-blue-700"
        >
          {t('community.groupList.create')}
        </button>
      }
    >
      {(buildingId) => <GroupsContent buildingId={buildingId} />}
    </CommunityLayout>
  );
}

function matches(group: CommunityGroupWithMembership, search: string): boolean {
  if (!search) return true;
  const needle = search.toLowerCase();
  return (
    group.name.toLowerCase().includes(needle) ||
    (group.description ?? '').toLowerCase().includes(needle)
  );
}

function GroupsContent({ buildingId }: { buildingId: string }) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { showToast } = useToast();
  const [search, setSearch] = useState('');
  const [type, setType] = useState<CommunityGroupType | ''>('');
  const [mineOnly, setMineOnly] = useState(false);

  const groupsQuery = useGroups(buildingId);
  const joinGroup = useJoinGroup();
  const leaveGroup = useLeaveGroup();

  const groups = useMemo(
    () =>
      (groupsQuery.data ?? []).filter(
        (g) =>
          matches(g, search.trim()) && (!type || g.groupType === type) && (!mineOnly || g.isMember)
      ),
    [groupsQuery.data, search, type, mineOnly]
  );

  const handleJoin = (id: string) =>
    joinGroup.mutate(id, {
      onSuccess: () =>
        showToast({
          type: 'success',
          title: t('common.success'),
          message: t('community.groupJoined'),
        }),
      onError: (err) =>
        showToast({
          type: 'error',
          title: t('community.joinFailed'),
          message: getErrorMessage(err),
        }),
    });

  const handleLeave = (id: string) =>
    leaveGroup.mutate(id, {
      onSuccess: () =>
        showToast({
          type: 'success',
          title: t('common.success'),
          message: t('community.groupLeft'),
        }),
      onError: (err) =>
        showToast({
          type: 'error',
          title: t('community.leaveFailed'),
          message: getErrorMessage(err),
        }),
    });

  if (groupsQuery.isLoading) return <LoadingState />;
  if (groupsQuery.error) {
    return (
      <ErrorState message={t('community.hub.loadError')} onRetry={() => groupsQuery.refetch()} />
    );
  }

  return (
    <div className="space-y-6">
      <div className="bg-white rounded-lg shadow p-4 flex flex-wrap items-center gap-3">
        <input
          type="search"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder={t('community.groupList.searchPlaceholder')}
          aria-label={t('community.common.search')}
          className="flex-1 min-w-[12rem] px-3 py-2 text-sm border border-gray-300 rounded-md"
        />
        <select
          value={type}
          onChange={(e) => setType(e.target.value as CommunityGroupType | '')}
          aria-label={t('community.hub.groupType')}
          className="px-3 py-2 text-sm border border-gray-300 rounded-md"
        >
          <option value="">{t('community.hub.allGroupTypes')}</option>
          {GROUP_TYPES.map((gt) => (
            <option key={gt} value={gt}>
              {t(`community.groupTypes.${gt}`)}
            </option>
          ))}
        </select>
        <label className="flex items-center gap-2 text-sm text-gray-600">
          <input
            type="checkbox"
            checked={mineOnly}
            onChange={(e) => setMineOnly(e.target.checked)}
            className="rounded border-gray-300"
          />
          {t('community.groupList.myGroupsOnly')}
        </label>
      </div>

      {groups.length === 0 ? (
        <EmptyState
          title={t('community.groupList.emptyTitle')}
          body={
            mineOnly
              ? t('community.groupList.emptyBodyMember')
              : t('community.groupList.emptyBodyDefault')
          }
        />
      ) : (
        <>
          <p className="text-sm text-gray-500">
            {t('community.groupList.showing', {
              count: groups.length,
              total: groupsQuery.data?.length ?? 0,
            })}
          </p>
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
            {groups.map((group) => (
              <GroupCard
                key={group.id}
                group={group}
                onOpen={(id) => navigate(withBuilding(`/community/groups/${id}`, buildingId))}
                onJoin={handleJoin}
                onLeave={handleLeave}
                isJoining={joinGroup.isPending && joinGroup.variables === group.id}
                isLeaving={leaveGroup.isPending && leaveGroup.variables === group.id}
              />
            ))}
          </div>
        </>
      )}
    </div>
  );
}
