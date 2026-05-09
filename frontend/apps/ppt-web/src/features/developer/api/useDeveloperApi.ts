/**
 * Developer Portal React Query Hooks (Epic 69)
 *
 * TanStack Query hooks for developer portal operations.
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import type {
  ApiKeyQuery,
  CreateApiKey,
  CreateWebhookSubscription,
  SdkLanguage,
  TestWebhookRequest,
  UpdateWebhookSubscription,
  WebhookDeliveryQuery,
  WebhookSubscriptionQuery,
} from '../types';
import * as api from './developerApi';

// Query keys
const QUERY_KEYS = {
  account: ['developer', 'account'] as const,
  usage: ['developer', 'usage'] as const,
  apiKeys: (query?: ApiKeyQuery) => ['developer', 'apiKeys', query] as const,
  apiKeyUsage: (id: string) => ['developer', 'apiKeys', id, 'usage'] as const,
  webhooks: (query?: WebhookSubscriptionQuery) => ['developer', 'webhooks', query] as const,
  webhookDeliveries: (webhookId: string, query?: WebhookDeliveryQuery) =>
    ['developer', 'webhooks', webhookId, 'deliveries', query] as const,
  rateLimitStatus: ['developer', 'rateLimits', 'status'] as const,
  rateLimitTiers: ['developer', 'rateLimits', 'tiers'] as const,
  sdkLanguages: ['developer', 'sdks'] as const,
  sdkInfo: (language: SdkLanguage) => ['developer', 'sdks', language] as const,
  sdkVersions: (language: SdkLanguage) => ['developer', 'sdks', language, 'versions'] as const,
};

// ==================== Developer Account Hooks ====================

export function useDeveloperAccount() {
  return useQuery({
    queryKey: QUERY_KEYS.account,
    queryFn: api.getMyDeveloperAccount,
  });
}

export function useDeveloperUsage() {
  return useQuery({
    queryKey: QUERY_KEYS.usage,
    queryFn: api.getMyUsageSummary,
  });
}

// ==================== API Key Hooks ====================

export function useApiKeys(query?: ApiKeyQuery) {
  return useQuery({
    queryKey: QUERY_KEYS.apiKeys(query),
    queryFn: () => api.listApiKeys(query),
  });
}

export function useApiKeyUsage(id: string, startDate?: string, endDate?: string) {
  return useQuery({
    queryKey: QUERY_KEYS.apiKeyUsage(id),
    queryFn: () => api.getApiKeyUsage(id, startDate, endDate),
    enabled: Boolean(id),
  });
}

export function useCreateApiKey() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: CreateApiKey) => api.createApiKey(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['developer', 'apiKeys'] });
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.usage });
    },
  });
}

export function useRotateApiKey() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.rotateApiKey(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['developer', 'apiKeys'] });
    },
  });
}

export function useRevokeApiKey() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.revokeApiKey(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['developer', 'apiKeys'] });
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.usage });
    },
  });
}

// ==================== Webhook Hooks ====================

export function useWebhooks(query?: WebhookSubscriptionQuery) {
  return useQuery({
    queryKey: QUERY_KEYS.webhooks(query),
    queryFn: () => api.listWebhooks(query),
  });
}

export function useWebhookDeliveries(webhookId: string, query?: WebhookDeliveryQuery) {
  return useQuery({
    queryKey: QUERY_KEYS.webhookDeliveries(webhookId, query),
    queryFn: () => api.listWebhookDeliveries(webhookId, query),
    enabled: Boolean(webhookId),
  });
}

export function useCreateWebhook() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: CreateWebhookSubscription) => api.createWebhook(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['developer', 'webhooks'] });
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.usage });
    },
  });
}

export function useUpdateWebhook() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, data }: { id: string; data: UpdateWebhookSubscription }) =>
      api.updateWebhook(id, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['developer', 'webhooks'] });
    },
  });
}

export function useDeleteWebhook() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.deleteWebhook(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['developer', 'webhooks'] });
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.usage });
    },
  });
}

export function useTestWebhook() {
  return useMutation({
    mutationFn: ({ id, request }: { id: string; request: TestWebhookRequest }) =>
      api.testWebhook(id, request),
  });
}

export function useRotateWebhookSecret() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.rotateWebhookSecret(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['developer', 'webhooks'] });
    },
  });
}

// ==================== Rate Limit Hooks ====================

export function useRateLimitStatus() {
  return useQuery({
    queryKey: QUERY_KEYS.rateLimitStatus,
    queryFn: api.getRateLimitStatus,
    refetchInterval: 60000, // Refresh every minute
  });
}

export function useRateLimitTiers() {
  return useQuery({
    queryKey: QUERY_KEYS.rateLimitTiers,
    queryFn: api.listRateLimitTiers,
    staleTime: 5 * 60 * 1000, // Cache for 5 minutes
  });
}

// ==================== SDK Hooks ====================

export function useSdkLanguages() {
  return useQuery({
    queryKey: QUERY_KEYS.sdkLanguages,
    queryFn: api.listSdkLanguages,
    staleTime: 5 * 60 * 1000,
  });
}

export function useSdkInfo(language: SdkLanguage) {
  return useQuery({
    queryKey: QUERY_KEYS.sdkInfo(language),
    queryFn: () => api.getSdkInfo(language),
    enabled: Boolean(language),
  });
}

export function useSdkVersions(language: SdkLanguage) {
  return useQuery({
    queryKey: QUERY_KEYS.sdkVersions(language),
    queryFn: () => api.listSdkVersions(language),
    enabled: Boolean(language),
  });
}

export function useDownloadSdk() {
  return useMutation({
    mutationFn: (language: SdkLanguage) => api.downloadSdk(language),
  });
}
