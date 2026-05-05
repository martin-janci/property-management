/**
 * RegistryRulesPage Component
 *
 * Page for configuring building registry rules (Epic 57, Story 57.7).
 */

import type { UpdateRegistryRulesRequest } from '@ppt/api-client';
import { createRegistryApi, createRegistryHooks } from '@ppt/api-client';
import { useMemo } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { RegistryRulesForm } from '../components/RegistryRulesForm';

// API base URL - prefer environment configuration for different environments (dev/staging/prod)
const API_BASE_URL = import.meta.env.VITE_API_BASE_URL ?? '';

// Create API instance - in production, this would come from a context with auth tokens
const registryApi = createRegistryApi({
  baseUrl: API_BASE_URL,
  // accessToken and tenantId would come from auth context
});

export function RegistryRulesPage() {
  const { buildingId } = useParams<{ buildingId: string }>();
  const navigate = useNavigate();

  // Create hooks instance from the API
  const registryHooks = useMemo(() => createRegistryHooks(registryApi), []);

  // Fetch registry rules for the building
  const {
    data: rules,
    isLoading,
    error,
  } = registryHooks.useRegistryRules(buildingId || '', !!buildingId);

  // Mutation for updating rules
  const updateRulesMutation = registryHooks.useUpdateRegistryRules();

  const handleSubmit = async (data: UpdateRegistryRulesRequest) => {
    if (!buildingId) return;

    try {
      await updateRulesMutation.mutateAsync({
        buildingId,
        data,
      });
      // Success - could show a toast notification here
    } catch (err) {
      // Error handling - could show error toast
      console.error('Failed to update registry rules:', err);
    }
  };

  const handleCancel = () => {
    // Navigate back to registry overview or building settings
    navigate(-1);
  };

  // Loading state
  if (isLoading) {
    return (
      <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        <div className="animate-pulse">
          <div className="h-8 bg-gray-200 rounded w-1/4 mb-4" />
          <div className="h-4 bg-gray-200 rounded w-1/2 mb-8" />
          <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6">
            <div className="space-y-4">
              <div className="h-10 bg-gray-200 rounded" />
              <div className="h-10 bg-gray-200 rounded" />
              <div className="h-10 bg-gray-200 rounded" />
            </div>
          </div>
        </div>
      </div>
    );
  }

  // Error state
  if (error || !buildingId) {
    return (
      <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        <div className="bg-red-50 border border-red-200 rounded-lg p-6 text-center">
          <h2 className="text-lg font-medium text-red-800 mb-2">
            {!buildingId ? 'Building ID Required' : 'Failed to Load Registry Rules'}
          </h2>
          <p className="text-red-600">
            {!buildingId
              ? 'Please select a building to configure registry rules.'
              : 'There was an error loading the registry rules. Please try again.'}
          </p>
          <button
            type="button"
            onClick={() => navigate(-1)}
            className="mt-4 px-4 py-2 bg-red-600 text-white rounded-md hover:bg-red-700"
          >
            Go Back
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      <div className="mb-8">
        <h1 className="text-3xl font-bold text-gray-900">Registry Rules</h1>
        <p className="mt-2 text-gray-600">
          Configure pet and vehicle registration rules for your building
        </p>
      </div>

      <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6">
        <RegistryRulesForm
          initialData={rules}
          onSubmit={handleSubmit}
          onCancel={handleCancel}
          isSubmitting={updateRulesMutation.isPending}
        />
      </div>

      {/* Mutation error display */}
      {updateRulesMutation.isError && (
        <div className="mt-4 p-4 bg-red-50 border border-red-200 rounded-lg">
          <p className="text-sm text-red-700">Failed to save changes. Please try again.</p>
        </div>
      )}

      {/* Success message */}
      {updateRulesMutation.isSuccess && (
        <div className="mt-4 p-4 bg-green-50 border border-green-200 rounded-lg">
          <p className="text-sm text-green-700">Registry rules updated successfully.</p>
        </div>
      )}

      {/* Info Section */}
      <div className="mt-6 bg-blue-50 border border-blue-200 rounded-lg p-4">
        <div className="flex">
          <svg
            className="w-5 h-5 text-blue-400 mt-0.5"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <title>Info</title>
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
            />
          </svg>
          <div className="ml-3">
            <h3 className="text-sm font-medium text-blue-800">About Registry Rules</h3>
            <div className="mt-2 text-sm text-blue-700">
              <ul className="list-disc pl-5 space-y-1">
                <li>Rules apply to all new registrations after they are saved</li>
                <li>Existing approved registrations are not affected by rule changes</li>
                <li>If approval is required, managers will be notified of new registrations</li>
                <li>Residents will be notified if their registration is rejected</li>
              </ul>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
