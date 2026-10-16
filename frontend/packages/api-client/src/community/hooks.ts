/**
 * Community Hooks
 *
 * React Query hooks for the Community API (Epic 42).
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import * as api from './api';
import type {
  ContactSellerRequest,
  CreateEventRequest,
  CreateGroupRequest,
  CreateItemRequest,
  CreatePostCommentRequest,
  CreatePostRequest,
  ListEventsParams,
  ListGroupsParams,
  ListItemsParams,
  ListPostsParams,
  RsvpRequest,
  UpdateEventRequest,
  UpdateGroupRequest,
  UpdateItemRequest,
  UpdatePostRequest,
} from './types';

// Query Keys
export const communityKeys = {
  all: ['community'] as const,
  stats: () => [...communityKeys.all, 'stats'] as const,
  groups: () => [...communityKeys.all, 'groups'] as const,
  groupList: (params: ListGroupsParams) => [...communityKeys.groups(), 'list', params] as const,
  groupDetail: (id: string) => [...communityKeys.groups(), 'detail', id] as const,
  groupMembers: (groupId: string) => [...communityKeys.groups(), groupId, 'members'] as const,
  posts: () => [...communityKeys.all, 'posts'] as const,
  postList: (params: ListPostsParams) => [...communityKeys.posts(), 'list', params] as const,
  postDetail: (id: string) => [...communityKeys.posts(), 'detail', id] as const,
  postComments: (postId: string) => [...communityKeys.posts(), postId, 'comments'] as const,
  events: () => [...communityKeys.all, 'events'] as const,
  eventList: (params: ListEventsParams) => [...communityKeys.events(), 'list', params] as const,
  eventDetail: (id: string) => [...communityKeys.events(), 'detail', id] as const,
  eventAttendees: (eventId: string) => [...communityKeys.events(), eventId, 'attendees'] as const,
  items: () => [...communityKeys.all, 'items'] as const,
  itemList: (params: ListItemsParams) => [...communityKeys.items(), 'list', params] as const,
  itemDetail: (id: string) => [...communityKeys.items(), 'detail', id] as const,
};

// ============================================
// Community Stats
// ============================================

export function useCommunityStats() {
  return useQuery({
    queryKey: communityKeys.stats(),
    queryFn: () => api.getCommunityStats(),
  });
}

// ============================================
// Story 42.1: Community Groups
// ============================================

export function useGroups(params: ListGroupsParams = {}) {
  return useQuery({
    queryKey: communityKeys.groupList(params),
    queryFn: () => api.listGroups(params),
  });
}

export function useGroup(id: string) {
  return useQuery({
    queryKey: communityKeys.groupDetail(id),
    queryFn: () => api.getGroup(id),
    enabled: !!id,
  });
}

export function useGroupMembers(groupId: string) {
  return useQuery({
    queryKey: communityKeys.groupMembers(groupId),
    queryFn: () => api.getGroupMembers(groupId),
    enabled: !!groupId,
  });
}

export function useCreateGroup() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (data: CreateGroupRequest) => api.createGroup(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: communityKeys.groups() });
    },
  });
}

export function useUpdateGroup() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, data }: { id: string; data: UpdateGroupRequest }) =>
      api.updateGroup(id, data),
    onSuccess: (_, { id }) => {
      queryClient.invalidateQueries({ queryKey: communityKeys.groupDetail(id) });
      queryClient.invalidateQueries({ queryKey: communityKeys.groups() });
    },
  });
}

export function useDeleteGroup() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (groupId: string) => api.deleteGroup(groupId),
    onSuccess: (_, groupId) => {
      // Drop the now-deleted group's detail cache and refresh any group list.
      queryClient.removeQueries({ queryKey: communityKeys.groupDetail(groupId) });
      queryClient.invalidateQueries({ queryKey: communityKeys.groups() });
    },
  });
}

export function useJoinGroup() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (groupId: string) => api.joinGroup(groupId),
    onSuccess: (_, groupId) => {
      queryClient.invalidateQueries({ queryKey: communityKeys.groupDetail(groupId) });
      queryClient.invalidateQueries({ queryKey: communityKeys.groups() });
    },
  });
}

export function useLeaveGroup() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (groupId: string) => api.leaveGroup(groupId),
    onSuccess: (_, groupId) => {
      queryClient.invalidateQueries({ queryKey: communityKeys.groupDetail(groupId) });
      queryClient.invalidateQueries({ queryKey: communityKeys.groups() });
    },
  });
}

// ============================================
// Story 42.2: Community Posts
// ============================================

export function usePosts(params: ListPostsParams = {}) {
  return useQuery({
    queryKey: communityKeys.postList(params),
    queryFn: () => api.listPosts(params),
  });
}

export function usePost(id: string) {
  return useQuery({
    queryKey: communityKeys.postDetail(id),
    queryFn: () => api.getPost(id),
    enabled: !!id,
  });
}

export function usePostComments(postId: string) {
  return useQuery({
    queryKey: communityKeys.postComments(postId),
    queryFn: () => api.getPostComments(postId),
    enabled: !!postId,
  });
}

export function useCreatePost() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (data: CreatePostRequest) => api.createPost(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: communityKeys.posts() });
    },
  });
}

export function useUpdatePost() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, data }: { id: string; data: UpdatePostRequest }) => api.updatePost(id, data),
    onSuccess: (_, { id }) => {
      queryClient.invalidateQueries({ queryKey: communityKeys.postDetail(id) });
      queryClient.invalidateQueries({ queryKey: communityKeys.posts() });
    },
  });
}

export function useDeletePost() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: string) => api.deletePost(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: communityKeys.posts() });
    },
  });
}

export function useLikePost() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (postId: string) => api.likePost(postId),
    onSuccess: (_, postId) => {
      queryClient.invalidateQueries({ queryKey: communityKeys.postDetail(postId) });
      queryClient.invalidateQueries({ queryKey: communityKeys.posts() });
    },
  });
}

export function useUnlikePost() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (postId: string) => api.unlikePost(postId),
    onSuccess: (_, postId) => {
      queryClient.invalidateQueries({ queryKey: communityKeys.postDetail(postId) });
      queryClient.invalidateQueries({ queryKey: communityKeys.posts() });
    },
  });
}

export function useCreatePostComment() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ postId, data }: { postId: string; data: CreatePostCommentRequest }) =>
      api.createPostComment(postId, data),
    onSuccess: (_, { postId }) => {
      queryClient.invalidateQueries({ queryKey: communityKeys.postComments(postId) });
      queryClient.invalidateQueries({ queryKey: communityKeys.postDetail(postId) });
    },
  });
}

// ============================================
// Story 42.3: Community Events
// ============================================

export function useEvents(params: ListEventsParams = {}) {
  return useQuery({
    queryKey: communityKeys.eventList(params),
    queryFn: () => api.listEvents(params),
  });
}

export function useEvent(id: string) {
  return useQuery({
    queryKey: communityKeys.eventDetail(id),
    queryFn: () => api.getEvent(id),
    enabled: !!id,
  });
}

export function useEventAttendees(eventId: string) {
  return useQuery({
    queryKey: communityKeys.eventAttendees(eventId),
    queryFn: () => api.getEventAttendees(eventId),
    enabled: !!eventId,
  });
}

export function useCreateEvent() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (data: CreateEventRequest) => api.createEvent(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: communityKeys.events() });
    },
  });
}

export function useUpdateEvent() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, data }: { id: string; data: UpdateEventRequest }) =>
      api.updateEvent(id, data),
    onSuccess: (_, { id }) => {
      queryClient.invalidateQueries({ queryKey: communityKeys.eventDetail(id) });
      queryClient.invalidateQueries({ queryKey: communityKeys.events() });
    },
  });
}

export function useRsvpEvent() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ eventId, data }: { eventId: string; data: RsvpRequest }) =>
      api.rsvpEvent(eventId, data),
    onSuccess: (_, { eventId }) => {
      queryClient.invalidateQueries({ queryKey: communityKeys.eventDetail(eventId) });
      queryClient.invalidateQueries({ queryKey: communityKeys.eventAttendees(eventId) });
      queryClient.invalidateQueries({ queryKey: communityKeys.events() });
    },
  });
}

export function useCancelRsvp() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (eventId: string) => api.cancelRsvp(eventId),
    onSuccess: (_, eventId) => {
      queryClient.invalidateQueries({ queryKey: communityKeys.eventDetail(eventId) });
      queryClient.invalidateQueries({ queryKey: communityKeys.eventAttendees(eventId) });
      queryClient.invalidateQueries({ queryKey: communityKeys.events() });
    },
  });
}

// ============================================
// Story 42.4: Item Marketplace
// ============================================

export function useMarketplaceItems(params: ListItemsParams = {}) {
  return useQuery({
    queryKey: communityKeys.itemList(params),
    queryFn: () => api.listItems(params),
  });
}

export function useMarketplaceItem(id: string) {
  return useQuery({
    queryKey: communityKeys.itemDetail(id),
    queryFn: () => api.getItem(id),
    enabled: !!id,
  });
}

export function useCreateItem() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (data: CreateItemRequest) => api.createItem(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: communityKeys.items() });
    },
  });
}

export function useUpdateItem() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, data }: { id: string; data: UpdateItemRequest }) => api.updateItem(id, data),
    onSuccess: (_, { id }) => {
      queryClient.invalidateQueries({ queryKey: communityKeys.itemDetail(id) });
      queryClient.invalidateQueries({ queryKey: communityKeys.items() });
    },
  });
}

export function useDeleteItem() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: string) => api.deleteItem(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: communityKeys.items() });
    },
  });
}

export function useContactSeller() {
  return useMutation({
    mutationFn: ({ itemId, data }: { itemId: string; data: ContactSellerRequest }) =>
      api.contactSeller(itemId, data),
  });
}
