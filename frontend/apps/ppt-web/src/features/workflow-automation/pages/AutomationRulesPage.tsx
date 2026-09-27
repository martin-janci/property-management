/**
 * AutomationRulesPage
 *
 * List all automation rules with filtering and actions.
 * Part of Story 43.1: Automation Rule Builder.
 */

import {
  type AutomationRule,
  useAutomationRules,
  useDeleteAutomationRule,
  useRunAutomationRule,
  useUpdateAutomationRule,
} from '@ppt/api-client';
import { useState } from 'react';
import { useNavigate } from 'react-router-dom';

import { useToast } from '../../../components';
import { RuleCard } from '../components/RuleCard';

type FilterStatus = 'all' | 'active' | 'paused';
type FilterTrigger = 'all' | 'time_based' | 'event_based' | 'condition_based' | 'manual';

const skeletonKeys = ['skeleton-1', 'skeleton-2', 'skeleton-3'];

export function AutomationRulesPage() {
  const navigate = useNavigate();
  const { showToast } = useToast();
  const [statusFilter, setStatusFilter] = useState<FilterStatus>('all');
  const [triggerFilter, setTriggerFilter] = useState<FilterTrigger>('all');
  const [searchQuery, setSearchQuery] = useState('');
  const [deleteConfirm, setDeleteConfirm] = useState<AutomationRule | null>(null);

  const {
    data: rulesData,
    isLoading,
    error,
  } = useAutomationRules({
    page: 1,
    pageSize: 50,
    ...(statusFilter !== 'all' && { isEnabled: statusFilter === 'active' }),
    ...(triggerFilter !== 'all' && { triggerType: triggerFilter }),
    ...(searchQuery && { search: searchQuery }),
  });

  const updateRule = useUpdateAutomationRule();
  const deleteRule = useDeleteAutomationRule();
  const runRule = useRunAutomationRule();

  const handleEdit = (rule: AutomationRule) => {
    navigate(`/automations/rules/${rule.id}/edit`);
  };

  const handleDelete = (rule: AutomationRule) => {
    setDeleteConfirm(rule);
  };

  const confirmDelete = async () => {
    if (!deleteConfirm) return;
    const rule = deleteConfirm;
    try {
      await deleteRule.mutateAsync(rule.id);
      showToast({
        type: 'success',
        title: 'Rule deleted',
        message: `"${rule.name}" was deleted.`,
      });
      setDeleteConfirm(null);
    } catch (err) {
      showToast({
        type: 'error',
        title: 'Delete failed',
        message: err instanceof Error ? err.message : 'The automation rule could not be deleted.',
      });
    }
  };

  const handleToggle = async (rule: AutomationRule, enabled: boolean) => {
    try {
      await updateRule.mutateAsync({
        id: rule.id,
        data: { isEnabled: enabled },
      });
      showToast({
        type: 'success',
        title: enabled ? 'Rule activated' : 'Rule paused',
        message: `"${rule.name}" was ${enabled ? 'activated' : 'paused'}.`,
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: 'Update failed',
        message: err instanceof Error ? err.message : 'The automation rule could not be updated.',
      });
    }
  };

  const handleRun = async (rule: AutomationRule) => {
    try {
      await runRule.mutateAsync(rule.id);
      showToast({
        type: 'success',
        title: 'Rule triggered',
        message: `"${rule.name}" is now running.`,
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: 'Run failed',
        message: err instanceof Error ? err.message : 'The automation rule could not be run.',
      });
    }
  };

  const rules = rulesData?.data ?? [];
  const totalRules = rulesData?.total ?? 0;

  return (
    <div className="max-w-6xl mx-auto px-4 py-8">
      {/* Header */}
      <div className="flex items-center justify-between mb-8">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Automation Rules</h1>
          <p className="mt-1 text-sm text-gray-500">
            Create and manage automated workflows for your properties.
          </p>
        </div>
        <button
          type="button"
          onClick={() => navigate('/automations/rules/new')}
          className="inline-flex items-center px-4 py-2 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 transition-colors"
        >
          <svg
            className="w-5 h-5 mr-2"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            aria-hidden="true"
          >
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
          </svg>
          Create Rule
        </button>
      </div>

      {/* Filters */}
      <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-4 mb-6">
        <div className="flex flex-wrap gap-4">
          {/* Search */}
          <div className="flex-1 min-w-64">
            <label htmlFor="search" className="sr-only">
              Search rules
            </label>
            <div className="relative">
              <svg
                className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
                aria-hidden="true"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
                />
              </svg>
              <input
                id="search"
                type="text"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder="Search rules..."
                className="w-full pl-10 pr-4 py-2 border border-gray-300 rounded-md text-sm focus:ring-blue-500 focus:border-blue-500"
              />
            </div>
          </div>

          {/* Status Filter */}
          <div>
            <label htmlFor="status-filter" className="sr-only">
              Filter by status
            </label>
            <select
              id="status-filter"
              value={statusFilter}
              onChange={(e) => setStatusFilter(e.target.value as FilterStatus)}
              className="px-3 py-2 border border-gray-300 rounded-md text-sm focus:ring-blue-500 focus:border-blue-500"
            >
              <option value="all">All Status</option>
              <option value="active">Active</option>
              <option value="paused">Paused</option>
            </select>
          </div>

          {/* Trigger Filter */}
          <div>
            <label htmlFor="trigger-filter" className="sr-only">
              Filter by trigger
            </label>
            <select
              id="trigger-filter"
              value={triggerFilter}
              onChange={(e) => setTriggerFilter(e.target.value as FilterTrigger)}
              className="px-3 py-2 border border-gray-300 rounded-md text-sm focus:ring-blue-500 focus:border-blue-500"
            >
              <option value="all">All Triggers</option>
              <option value="time_based">Time Based</option>
              <option value="event_based">Event Based</option>
              <option value="condition_based">Condition Based</option>
              <option value="manual">Manual</option>
            </select>
          </div>
        </div>
      </div>

      {/* Content */}
      {error && (
        <div className="bg-red-50 border border-red-200 rounded-lg p-4 mb-6">
          <p className="text-red-700">Failed to load automation rules. Please try again.</p>
        </div>
      )}

      {isLoading ? (
        <div className="grid gap-4">
          {skeletonKeys.map((key) => (
            <div
              key={key}
              className="bg-white rounded-lg shadow-sm border border-gray-200 p-4 animate-pulse"
            >
              <div className="flex items-start gap-3">
                <div className="w-10 h-10 bg-gray-200 rounded-full" />
                <div className="flex-1">
                  <div className="h-5 bg-gray-200 rounded w-1/3 mb-2" />
                  <div className="h-4 bg-gray-200 rounded w-2/3" />
                </div>
              </div>
            </div>
          ))}
        </div>
      ) : rules.length === 0 ? (
        <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-12 text-center">
          <svg
            className="mx-auto h-16 w-16 text-gray-300"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            aria-hidden="true"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={1.5}
              d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z"
            />
          </svg>
          <h3 className="mt-4 text-lg font-medium text-gray-900">No automation rules yet</h3>
          <p className="mt-2 text-sm text-gray-500">
            Create your first automation to streamline your property management tasks.
          </p>
          <button
            type="button"
            onClick={() => navigate('/automations/rules/new')}
            className="mt-6 inline-flex items-center px-4 py-2 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700"
          >
            <svg
              className="w-5 h-5 mr-2"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              aria-hidden="true"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M12 4v16m8-8H4"
              />
            </svg>
            Create Your First Rule
          </button>
        </div>
      ) : (
        <>
          <p className="text-sm text-gray-500 mb-4">
            Showing {rules.length} of {totalRules} rules
          </p>
          <div className="grid gap-4">
            {rules.map((rule) => (
              <RuleCard
                key={rule.id}
                rule={rule}
                onEdit={handleEdit}
                onDelete={handleDelete}
                onToggle={handleToggle}
                onRun={handleRun}
              />
            ))}
          </div>
        </>
      )}

      {/* Delete Confirmation Modal */}
      {deleteConfirm && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg shadow-xl max-w-md w-full mx-4 p-6">
            <div className="flex items-center gap-3 mb-4">
              <div className="flex-shrink-0 w-10 h-10 bg-red-100 rounded-full flex items-center justify-center">
                <svg
                  className="w-5 h-5 text-red-600"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                  aria-hidden="true"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
                  />
                </svg>
              </div>
              <div>
                <h3 className="text-lg font-semibold text-gray-900">Delete Automation Rule</h3>
                <p className="text-sm text-gray-500">This action cannot be undone.</p>
              </div>
            </div>
            <p className="text-gray-700 mb-6">
              Are you sure you want to delete <strong>{deleteConfirm.name}</strong>?
            </p>
            <div className="flex justify-end gap-3">
              <button
                type="button"
                onClick={() => setDeleteConfirm(null)}
                className="px-4 py-2 text-sm font-medium text-gray-700 bg-gray-100 rounded-md hover:bg-gray-200"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={confirmDelete}
                disabled={deleteRule.isPending}
                className="px-4 py-2 text-sm font-medium text-white bg-red-600 rounded-md hover:bg-red-700 disabled:opacity-50"
              >
                {deleteRule.isPending ? 'Deleting...' : 'Delete'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
