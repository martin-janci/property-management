/**
 * TanStack Query hooks for the budgets API (Epic 24).
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import * as api from './api';
import type {
  CreateBudgetCategoryInput,
  CreateBudgetInput,
  CreateBudgetItemInput,
  ListBudgetsParams,
} from './types';

export const budgetKeys = {
  all: ['budgets'] as const,
  lists: () => [...budgetKeys.all, 'list'] as const,
  list: (params: ListBudgetsParams) => [...budgetKeys.lists(), params] as const,
  details: () => [...budgetKeys.all, 'detail'] as const,
  detail: (id: string) => [...budgetKeys.details(), id] as const,
  summary: (id: string) => [...budgetKeys.all, 'summary', id] as const,
  variance: (id: string) => [...budgetKeys.all, 'variance', id] as const,
  items: (id: string) => [...budgetKeys.all, 'items', id] as const,
  categories: (organizationId: string) =>
    [...budgetKeys.all, 'categories', organizationId] as const,
};

/** List budgets for an organization (disabled until an organization id is known). */
export function useBudgets(params: ListBudgetsParams | undefined) {
  return useQuery({
    queryKey: budgetKeys.list(params ?? { organizationId: '' }),
    queryFn: () => api.listBudgets(params as ListBudgetsParams),
    enabled: !!params?.organizationId,
  });
}

export function useBudget(organizationId: string | undefined, id: string | undefined) {
  return useQuery({
    queryKey: budgetKeys.detail(id ?? ''),
    queryFn: () => api.getBudget(organizationId as string, id as string),
    enabled: !!organizationId && !!id,
  });
}

export function useBudgetSummary(id: string | undefined) {
  return useQuery({
    queryKey: budgetKeys.summary(id ?? ''),
    queryFn: () => api.getBudgetSummary(id as string),
    enabled: !!id,
  });
}

export function useBudgetItems(id: string | undefined) {
  return useQuery({
    queryKey: budgetKeys.items(id ?? ''),
    queryFn: () => api.listBudgetItems(id as string),
    enabled: !!id,
  });
}

export function useBudgetCategories(organizationId: string | undefined) {
  return useQuery({
    queryKey: budgetKeys.categories(organizationId ?? ''),
    queryFn: () => api.listBudgetCategories(organizationId as string),
    enabled: !!organizationId,
  });
}

export function useCreateBudget() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateBudgetInput) => api.createBudget(input),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: budgetKeys.all }),
  });
}

export function useAddBudgetItem(budgetId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateBudgetItemInput) => api.addBudgetItem(budgetId, input),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: budgetKeys.all }),
  });
}

export function useCreateBudgetCategory() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateBudgetCategoryInput) => api.createBudgetCategory(input),
    onSuccess: (_data, input) =>
      queryClient.invalidateQueries({ queryKey: budgetKeys.categories(input.organizationId) }),
  });
}
