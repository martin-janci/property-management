/**
 * Document template React Query hooks (Epic 7B, Story 7B.2).
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import * as api from './api';
import type {
  CreateTemplateRequest,
  GenerateDocumentRequest,
  ListDocumentTemplatesParams,
  UpdateTemplateRequest,
} from './types';

// Query keys
export const templateKeys = {
  all: ['templates'] as const,
  lists: () => [...templateKeys.all, 'list'] as const,
  list: (params?: ListDocumentTemplatesParams) => [...templateKeys.lists(), params ?? {}] as const,
  details: () => [...templateKeys.all, 'detail'] as const,
  detail: (id: string) => [...templateKeys.details(), id] as const,
};

/**
 * List document templates (optionally filtered by type / search).
 */
export function useTemplates(params?: ListDocumentTemplatesParams) {
  return useQuery({
    queryKey: templateKeys.list(params),
    queryFn: () => api.listDocumentTemplates(params),
    staleTime: 30_000,
  });
}

/**
 * Get a single template by ID (placeholders + content for the generate flow).
 */
export function useTemplate(id: string | undefined) {
  return useQuery({
    queryKey: templateKeys.detail(id ?? ''),
    queryFn: () => api.getDocumentTemplate(id!),
    enabled: !!id,
    staleTime: 30_000,
  });
}

/**
 * Generate a document from a template.
 * Invalidates the template lists on success (server bumps `usage_count`).
 */
export function useGenerateDocument(templateId: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (body: GenerateDocumentRequest) => api.generateDocument(templateId, body),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: templateKeys.lists() });
      queryClient.invalidateQueries({ queryKey: templateKeys.detail(templateId) });
    },
  });
}

/**
 * Create a template. Invalidates template lists on success.
 */
export function useCreateTemplate() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (body: CreateTemplateRequest) => api.createTemplate(body),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: templateKeys.lists() });
    },
  });
}

/**
 * Update a template. Invalidates the affected detail + lists on success.
 */
export function useUpdateTemplate() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, body }: { id: string; body: UpdateTemplateRequest }) =>
      api.updateTemplate(id, body),
    onSuccess: (_data, { id }) => {
      queryClient.invalidateQueries({ queryKey: templateKeys.detail(id) });
      queryClient.invalidateQueries({ queryKey: templateKeys.lists() });
    },
  });
}

/**
 * Delete a template. Invalidates template lists on success.
 */
export function useDeleteTemplate() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: string) => api.deleteTemplate(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: templateKeys.lists() });
    },
  });
}
