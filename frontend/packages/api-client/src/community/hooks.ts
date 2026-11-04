/**
 * Community Hooks
 *
 * React Query hooks over the Community API (`./api`). Lists are scoped per
 * building (the server has no cross-building listing); a group's posts are
 * scoped per group. Mutations invalidate the narrowest affected keys.
 */

import { useMutation, useQueries, useQuery, useQueryClient } from '@tanstack/react-query';
import * as api from './api';
import type {
  CommunityPageParams,
  CommunityPost,
  CommunityReactionType,
  CommunityRsvpRequest,
  CreateCommunityCommentRequest,
  CreateCommunityEventRequest,
  CreateCommunityGroupRequest,
  CreateCommunityPostRequest,
  CreateMarketplaceInquiryRequest,
  CreateMarketplaceItemRequest,
  ListMarketplaceItemsParams,
} from './types';

export const communityKeys = {
  all: ['community'] as const,
  groups: () => [...communityKeys.all, 'groups'] as const,
  groupList: (buildingId: string) => [...communityKeys.groups(), 'list', buildingId] as const,
  groupDetail: (id: string) => [...communityKeys.groups(), 'detail', id] as const,
  posts: () => [...communityKeys.all, 'posts'] as const,
  groupPosts: (groupId: string, params: CommunityPageParams = {}) =>
    [...communityKeys.posts(), 'group', groupId, params] as const,
  events: () => [...communityKeys.all, 'events'] as const,
  eventList: (buildingId: string) => [...communityKeys.events(), 'list', buildingId] as const,
  items: () => [...communityKeys.all, 'items'] as const,
  itemList: (buildingId: string, params: ListMarketplaceItemsParams = {}) =>
    [...communityKeys.items(), 'list', buildingId, params] as const,
  itemDetail: (id: string) => [...communityKeys.items(), 'detail', id] as const,
};

// ============================================
// Groups
// ============================================

/** Groups of a building, with the caller's membership. Disabled without a building. */
export function useGroups(buildingId: string | undefined) {
  return useQuery({
    queryKey: communityKeys.groupList(buildingId ?? ''),
    queryFn: ({ signal }) => api.listGroups(buildingId as string, signal),
    enabled: !!buildingId,
  });
}

export function useGroup(id: string | undefined) {
  return useQuery({
    queryKey: communityKeys.groupDetail(id ?? ''),
    queryFn: ({ signal }) => api.getGroup(id as string, signal),
    enabled: !!id,
  });
}

export function useCreateGroup() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ buildingId, data }: { buildingId: string; data: CreateCommunityGroupRequest }) =>
      api.createGroup(buildingId, data),
    onSuccess: (_group, { buildingId }) => {
      queryClient.invalidateQueries({ queryKey: communityKeys.groupList(buildingId) });
    },
  });
}

export function useDeleteGroup() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.deleteGroup(id),
    onSuccess: (_void, id) => {
      queryClient.removeQueries({ queryKey: communityKeys.groupDetail(id) });
      queryClient.invalidateQueries({ queryKey: communityKeys.groups() });
    },
  });
}

function useMembershipMutation(fn: (id: string) => Promise<void>) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: fn,
    onSuccess: (_void, id) => {
      // Membership flags live on the per-building list; counts on both.
      queryClient.invalidateQueries({ queryKey: communityKeys.groups() });
      queryClient.invalidateQueries({ queryKey: communityKeys.groupDetail(id) });
      queryClient.invalidateQueries({ queryKey: communityKeys.posts() });
    },
  });
}

export function useJoinGroup() {
  return useMembershipMutation(api.joinGroup);
}

export function useLeaveGroup() {
  return useMembershipMutation(api.leaveGroup);
}

// ============================================
// Posts
// ============================================

export function useGroupPosts(groupId: string | undefined, params: CommunityPageParams = {}) {
  return useQuery({
    queryKey: communityKeys.groupPosts(groupId ?? '', params),
    queryFn: ({ signal }) => api.listGroupPosts(groupId as string, params, signal),
    enabled: !!groupId,
  });
}

/**
 * Feed = the newest posts of every given group, merged newest-first. The
 * server has no cross-group feed endpoint, so this fans out one
 * `GET /groups/{id}/posts` per group (bounded by `limit` each).
 */
export function useFeedPosts(groupIds: string[], limit = 20) {
  const results = useQueries({
    queries: groupIds.map((groupId) => ({
      queryKey: communityKeys.groupPosts(groupId, { limit }),
      queryFn: ({ signal }: { signal: AbortSignal }) =>
        api.listGroupPosts(groupId, { limit }, signal),
    })),
  });
  const posts: CommunityPost[] = results
    .flatMap((r) => r.data ?? [])
    .sort((a, b) => (a.createdAt < b.createdAt ? 1 : a.createdAt > b.createdAt ? -1 : 0));
  return {
    posts,
    isLoading: results.some((r) => r.isLoading),
    error: results.find((r) => r.error)?.error ?? null,
    refetch: () => Promise.all(results.map((r) => r.refetch())),
  };
}

export function useCreatePost() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ groupId, data }: { groupId: string; data: CreateCommunityPostRequest }) =>
      api.createPost(groupId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: communityKeys.posts() });
      queryClient.invalidateQueries({ queryKey: communityKeys.groups() });
    },
  });
}

export function useAddPostReaction() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      postId,
      reactionType = 'like',
    }: {
      postId: string;
      reactionType?: CommunityReactionType;
    }) => api.addPostReaction(postId, reactionType),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: communityKeys.posts() });
    },
  });
}

export function useCreatePostComment() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ postId, data }: { postId: string; data: CreateCommunityCommentRequest }) =>
      api.createPostComment(postId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: communityKeys.posts() });
    },
  });
}

// ============================================
// Events
// ============================================

export function useEvents(buildingId: string | undefined) {
  return useQuery({
    queryKey: communityKeys.eventList(buildingId ?? ''),
    queryFn: ({ signal }) => api.listEvents(buildingId as string, { limit: 50 }, signal),
    enabled: !!buildingId,
  });
}

export function useCreateEvent() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ buildingId, data }: { buildingId: string; data: CreateCommunityEventRequest }) =>
      api.createEvent(buildingId, data),
    onSuccess: (_event, { buildingId }) => {
      queryClient.invalidateQueries({ queryKey: communityKeys.eventList(buildingId) });
    },
  });
}

export function useRsvpEvent() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ eventId, data }: { eventId: string; data: CommunityRsvpRequest }) =>
      api.rsvpEvent(eventId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: communityKeys.events() });
    },
  });
}

// ============================================
// Marketplace
// ============================================

export function useMarketplaceItems(
  buildingId: string | undefined,
  params: ListMarketplaceItemsParams = {}
) {
  return useQuery({
    queryKey: communityKeys.itemList(buildingId ?? '', params),
    queryFn: ({ signal }) => api.listItems(buildingId as string, params, signal),
    enabled: !!buildingId,
  });
}

export function useMarketplaceItem(id: string | undefined) {
  return useQuery({
    queryKey: communityKeys.itemDetail(id ?? ''),
    queryFn: ({ signal }) => api.getItem(id as string, signal),
    enabled: !!id,
  });
}

export function useCreateItem() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      buildingId,
      data,
    }: {
      buildingId: string;
      data: CreateMarketplaceItemRequest;
    }) => api.createItem(buildingId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: communityKeys.items() });
    },
  });
}

export function useCreateItemInquiry() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ itemId, data }: { itemId: string; data: CreateMarketplaceInquiryRequest }) =>
      api.createItemInquiry(itemId, data),
    onSuccess: (_inq, { itemId }) => {
      queryClient.invalidateQueries({ queryKey: communityKeys.itemDetail(itemId) });
    },
  });
}
