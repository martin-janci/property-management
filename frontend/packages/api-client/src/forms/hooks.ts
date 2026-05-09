/**
 * Forms Management TanStack Query Hooks
 *
 * React hooks for managing forms with server state caching (Epic 54).
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import type { FormsApi } from './api';
import type {
  CreateFormField,
  CreateFormRequest,
  FieldOrderRequest,
  FormExportFormat,
  ListFormSubmissionsParams,
  ListFormsParams,
  ReviewSubmissionRequest,
  SubmitFormRequest,
  UpdateFormField,
  UpdateFormRequest,
} from './types';

// Query keys factory for cache management
export const formKeys = {
  all: ['forms'] as const,
  lists: () => [...formKeys.all, 'list'] as const,
  list: (params?: ListFormsParams) => [...formKeys.lists(), params] as const,
  available: () => [...formKeys.all, 'available'] as const,
  details: () => [...formKeys.all, 'detail'] as const,
  detail: (id: string) => [...formKeys.details(), id] as const,
  fields: (formId: string) => [...formKeys.detail(formId), 'fields'] as const,
  submissions: (formId: string) => [...formKeys.detail(formId), 'submissions'] as const,
  submissionList: (formId: string, params?: ListFormSubmissionsParams) =>
    [...formKeys.submissions(formId), params] as const,
  submission: (formId: string, submissionId: string) =>
    [...formKeys.submissions(formId), submissionId] as const,
  mySubmissions: () => [...formKeys.all, 'my-submissions'] as const,
  statistics: () => [...formKeys.all, 'statistics'] as const,
  categories: () => [...formKeys.all, 'categories'] as const,
};

export const createFormHooks = (api: FormsApi) => ({
  // ========================================================================
  // Form Queries
  // ========================================================================

  /**
   * List forms with filters (managers)
   */
  useList: (params?: ListFormsParams) =>
    useQuery({
      queryKey: formKeys.list(params),
      queryFn: () => api.list(params),
    }),

  /**
   * List available forms for current user
   */
  useListAvailable: (params?: {
    page?: number;
    pageSize?: number;
    category?: string;
    search?: string;
  }) =>
    useQuery({
      queryKey: formKeys.available(),
      queryFn: () => api.listAvailable(params),
    }),

  /**
   * Get form details
   */
  useGet: (id: string, enabled = true) =>
    useQuery({
      queryKey: formKeys.detail(id),
      queryFn: () => api.get(id),
      enabled: enabled && !!id,
    }),

  /**
   * List fields for a form
   */
  useFields: (formId: string, enabled = true) =>
    useQuery({
      queryKey: formKeys.fields(formId),
      queryFn: () => api.listFields(formId),
      enabled: enabled && !!formId,
    }),

  /**
   * Get form statistics
   */
  useStatistics: () =>
    useQuery({
      queryKey: formKeys.statistics(),
      queryFn: () => api.getStatistics(),
    }),

  /**
   * Get available categories
   */
  useCategories: () =>
    useQuery({
      queryKey: formKeys.categories(),
      queryFn: () => api.getCategories(),
    }),

  // ========================================================================
  // Form Mutations
  // ========================================================================

  /**
   * Create form mutation
   */
  useCreate: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: (data: CreateFormRequest) => api.create(data),
      onSuccess: () => {
        queryClient.invalidateQueries({ queryKey: formKeys.lists() });
        queryClient.invalidateQueries({ queryKey: formKeys.statistics() });
        queryClient.invalidateQueries({ queryKey: formKeys.categories() });
      },
    });
  },

  /**
   * Update form mutation
   */
  useUpdate: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: ({ id, data }: { id: string; data: UpdateFormRequest }) => api.update(id, data),
      onSuccess: (_, { id }) => {
        queryClient.invalidateQueries({ queryKey: formKeys.detail(id) });
        queryClient.invalidateQueries({ queryKey: formKeys.lists() });
        queryClient.invalidateQueries({ queryKey: formKeys.categories() });
      },
    });
  },

  /**
   * Delete form mutation
   */
  useDelete: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: (id: string) => api.delete(id),
      onSuccess: () => {
        queryClient.invalidateQueries({ queryKey: formKeys.lists() });
        queryClient.invalidateQueries({ queryKey: formKeys.statistics() });
      },
    });
  },

  /**
   * Publish form mutation
   */
  usePublish: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: (id: string) => api.publish(id),
      onSuccess: (_data: unknown, id: string) => {
        queryClient.invalidateQueries({ queryKey: formKeys.detail(id) });
        queryClient.invalidateQueries({ queryKey: formKeys.lists() });
        queryClient.invalidateQueries({ queryKey: formKeys.available() });
        queryClient.invalidateQueries({ queryKey: formKeys.statistics() });
      },
    });
  },

  /**
   * Archive form mutation
   */
  useArchive: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: (id: string) => api.archive(id),
      onSuccess: (_data: unknown, id: string) => {
        queryClient.invalidateQueries({ queryKey: formKeys.detail(id) });
        queryClient.invalidateQueries({ queryKey: formKeys.lists() });
        queryClient.invalidateQueries({ queryKey: formKeys.available() });
        queryClient.invalidateQueries({ queryKey: formKeys.statistics() });
      },
    });
  },

  /**
   * Duplicate form mutation
   */
  useDuplicate: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: (id: string) => api.duplicate(id),
      onSuccess: () => {
        queryClient.invalidateQueries({ queryKey: formKeys.lists() });
        queryClient.invalidateQueries({ queryKey: formKeys.statistics() });
      },
    });
  },

  // ========================================================================
  // Field Mutations
  // ========================================================================

  /**
   * Add field mutation
   */
  useAddField: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: ({ formId, data }: { formId: string; data: CreateFormField }) =>
        api.addField(formId, data),
      onSuccess: (_, { formId }) => {
        queryClient.invalidateQueries({ queryKey: formKeys.fields(formId) });
        queryClient.invalidateQueries({ queryKey: formKeys.detail(formId) });
      },
    });
  },

  /**
   * Update field mutation
   */
  useUpdateField: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: ({
        formId,
        fieldId,
        data,
      }: {
        formId: string;
        fieldId: string;
        data: UpdateFormField;
      }) => api.updateField(formId, fieldId, data),
      onSuccess: (_, { formId }) => {
        queryClient.invalidateQueries({ queryKey: formKeys.fields(formId) });
        queryClient.invalidateQueries({ queryKey: formKeys.detail(formId) });
      },
    });
  },

  /**
   * Delete field mutation
   */
  useDeleteField: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: ({ formId, fieldId }: { formId: string; fieldId: string }) =>
        api.deleteField(formId, fieldId),
      onSuccess: (_, { formId }) => {
        queryClient.invalidateQueries({ queryKey: formKeys.fields(formId) });
        queryClient.invalidateQueries({ queryKey: formKeys.detail(formId) });
      },
    });
  },

  /**
   * Reorder fields mutation
   */
  useReorderFields: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: ({ formId, data }: { formId: string; data: FieldOrderRequest }) =>
        api.reorderFields(formId, data),
      onSuccess: (_, { formId }) => {
        queryClient.invalidateQueries({ queryKey: formKeys.fields(formId) });
      },
    });
  },

  // ========================================================================
  // Download/Export
  // ========================================================================

  /**
   * Download form PDF mutation
   */
  useDownloadPdf: () => {
    return useMutation({
      mutationFn: (id: string) => api.downloadPdf(id),
    });
  },

  /**
   * Export submissions mutation
   */
  useExportSubmissions: () => {
    return useMutation({
      mutationFn: ({ id, format }: { id: string; format: FormExportFormat }) =>
        api.exportSubmissions(id, format),
    });
  },

  // ========================================================================
  // Submissions
  // ========================================================================

  /**
   * Submit form mutation
   */
  useSubmit: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: ({ id, data }: { id: string; data: SubmitFormRequest }) => api.submit(id, data),
      onSuccess: (_, { id }) => {
        queryClient.invalidateQueries({ queryKey: formKeys.submissions(id) });
        queryClient.invalidateQueries({ queryKey: formKeys.detail(id) });
        queryClient.invalidateQueries({ queryKey: formKeys.mySubmissions() });
        queryClient.invalidateQueries({ queryKey: formKeys.statistics() });
      },
    });
  },

  /**
   * List submissions for a form (managers)
   */
  useSubmissions: (formId: string, params?: ListFormSubmissionsParams, enabled = true) =>
    useQuery({
      queryKey: formKeys.submissionList(formId, params),
      queryFn: () => api.listSubmissions(formId, params),
      enabled: enabled && !!formId,
    }),

  /**
   * Get submission details
   */
  useSubmission: (formId: string, submissionId: string, enabled = true) =>
    useQuery({
      queryKey: formKeys.submission(formId, submissionId),
      queryFn: () => api.getSubmission(formId, submissionId),
      enabled: enabled && !!formId && !!submissionId,
    }),

  /**
   * Review submission mutation
   */
  useReviewSubmission: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: ({
        formId,
        submissionId,
        data,
      }: {
        formId: string;
        submissionId: string;
        data: ReviewSubmissionRequest;
      }) => api.reviewSubmission(formId, submissionId, data),
      onSuccess: (_, { formId, submissionId }) => {
        queryClient.invalidateQueries({ queryKey: formKeys.submission(formId, submissionId) });
        queryClient.invalidateQueries({ queryKey: formKeys.submissions(formId) });
        queryClient.invalidateQueries({ queryKey: formKeys.detail(formId) });
        queryClient.invalidateQueries({ queryKey: formKeys.statistics() });
      },
    });
  },

  /**
   * List my submissions (user view)
   */
  useMySubmissions: (params?: { page?: number; pageSize?: number; status?: string }) =>
    useQuery({
      queryKey: formKeys.mySubmissions(),
      queryFn: () => api.listMySubmissions(params),
    }),
});

export type FormHooks = ReturnType<typeof createFormHooks>;
