/**
 * Onboarding TanStack Query hooks (Epic 10B, Story 10B.6).
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  completeOnboardingStep,
  completeOnboardingTour,
  fetchOnboardingStatus,
  fetchOnboardingTour,
  fetchOnboardingTours,
  resetOnboardingTour,
  skipOnboardingTour,
  startOnboardingTour,
} from './api';

// ============================================
// Query key factory
// ============================================

export const onboardingKeys = {
  all: ['onboarding'] as const,
  status: () => [...onboardingKeys.all, 'status'] as const,
  tours: () => [...onboardingKeys.all, 'tours'] as const,
  tour: (tourId: string) => [...onboardingKeys.tours(), tourId] as const,
};

// ============================================
// Hooks
// ============================================

export function useOnboardingStatus() {
  return useQuery({
    queryKey: onboardingKeys.status(),
    queryFn: ({ signal }) => fetchOnboardingStatus(signal),
    staleTime: 30_000,
  });
}

export function useOnboardingTours() {
  return useQuery({
    queryKey: onboardingKeys.tours(),
    queryFn: ({ signal }) => fetchOnboardingTours(signal),
    staleTime: 30_000,
  });
}

export function useOnboardingTour(tourId: string) {
  return useQuery({
    queryKey: onboardingKeys.tour(tourId),
    queryFn: ({ signal }) => fetchOnboardingTour(tourId, signal),
    staleTime: 60_000,
    enabled: !!tourId,
  });
}

export function useStartOnboardingTour() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (tourId: string) => startOnboardingTour(tourId),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: onboardingKeys.tours() });
      void qc.invalidateQueries({ queryKey: onboardingKeys.status() });
    },
  });
}

export function useCompleteOnboardingStep() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ tourId, stepId }: { tourId: string; stepId: string }) =>
      completeOnboardingStep(tourId, stepId),
    onSuccess: (_data, { tourId }) => {
      void qc.invalidateQueries({ queryKey: onboardingKeys.tours() });
      void qc.invalidateQueries({ queryKey: onboardingKeys.tour(tourId) });
      void qc.invalidateQueries({ queryKey: onboardingKeys.status() });
    },
  });
}

export function useCompleteOnboardingTour() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (tourId: string) => completeOnboardingTour(tourId),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: onboardingKeys.tours() });
      void qc.invalidateQueries({ queryKey: onboardingKeys.status() });
    },
  });
}

export function useSkipOnboardingTour() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (tourId: string) => skipOnboardingTour(tourId),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: onboardingKeys.tours() });
      void qc.invalidateQueries({ queryKey: onboardingKeys.status() });
    },
  });
}

export function useResetOnboardingTour() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (tourId: string) => resetOnboardingTour(tourId),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: onboardingKeys.tours() });
      void qc.invalidateQueries({ queryKey: onboardingKeys.status() });
    },
  });
}
