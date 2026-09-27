/**
 * TemplateLibraryPage
 *
 * Browse and use pre-built automation templates.
 * Part of Story 43.2: Template Library.
 */

import type { AutomationTemplate, TemplateCategory } from '@ppt/api-client';
import { useAutomationTemplates, useCreateRuleFromTemplate } from '@ppt/api-client';
import { useState } from 'react';
import { useNavigate } from 'react-router-dom';

import { useToast } from '../../../components';
import { TemplateCard } from '../components/TemplateCard';
import { TemplatePreviewModal } from '../components/TemplatePreviewModal';

const categories: { value: TemplateCategory | 'all'; label: string; icon: string }[] = [
  { value: 'all', label: 'All Templates', icon: '📚' },
  { value: 'faults', label: 'Faults', icon: '🔧' },
  { value: 'payments', label: 'Payments', icon: '💰' },
  { value: 'communications', label: 'Communications', icon: '📢' },
  { value: 'documents', label: 'Documents', icon: '📄' },
  { value: 'maintenance', label: 'Maintenance', icon: '🛠️' },
];

const skeletonKeys = [
  'skeleton-1',
  'skeleton-2',
  'skeleton-3',
  'skeleton-4',
  'skeleton-5',
  'skeleton-6',
];

export function TemplateLibraryPage() {
  const navigate = useNavigate();
  const { showToast } = useToast();
  const [selectedCategory, setSelectedCategory] = useState<TemplateCategory | 'all'>('all');
  const [searchQuery, setSearchQuery] = useState('');
  const [previewTemplate, setPreviewTemplate] = useState<AutomationTemplate | null>(null);

  const {
    data: templatesData,
    isLoading,
    error,
  } = useAutomationTemplates({
    page: 1,
    pageSize: 50,
    ...(selectedCategory !== 'all' && { category: selectedCategory }),
    ...(searchQuery && { search: searchQuery }),
  });

  const createFromTemplate = useCreateRuleFromTemplate();

  const handleUseTemplate = async (template: AutomationTemplate) => {
    try {
      const result = await createFromTemplate.mutateAsync(template.id);
      if (result?.id) {
        navigate(`/automations/rules/${result.id}/edit`);
      } else {
        navigate('/automations/rules/new', { state: { template } });
      }
    } catch (err) {
      showToast({
        type: 'error',
        title: 'Could not use template',
        message:
          err instanceof Error ? err.message : 'A rule could not be created from this template.',
      });
    }
  };

  const handlePreview = (template: AutomationTemplate) => {
    setPreviewTemplate(template);
  };

  const templates = templatesData?.data ?? [];
  const totalTemplates = templatesData?.total ?? 0;

  return (
    <div className="max-w-6xl mx-auto px-4 py-8">
      {/* Header */}
      <div className="flex items-center justify-between mb-8">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Template Library</h1>
          <p className="mt-1 text-sm text-gray-500">
            Start quickly with pre-built automation templates for common workflows.
          </p>
        </div>
        <button
          type="button"
          onClick={() => navigate('/automations/rules/new')}
          className="inline-flex items-center px-4 py-2 text-gray-700 bg-white border border-gray-300 font-medium rounded-lg hover:bg-gray-50 transition-colors"
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
          Create from Scratch
        </button>
      </div>

      {/* Search and Filters */}
      <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-4 mb-6">
        <div className="flex flex-col md:flex-row gap-4">
          {/* Search */}
          <div className="flex-1">
            <label htmlFor="template-search" className="sr-only">
              Search templates
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
                id="template-search"
                type="text"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder="Search templates..."
                className="w-full pl-10 pr-4 py-2 border border-gray-300 rounded-md text-sm focus:ring-blue-500 focus:border-blue-500"
              />
            </div>
          </div>
        </div>

        {/* Category Tabs */}
        <div className="mt-4 flex flex-wrap gap-2">
          {categories.map((category) => (
            <button
              key={category.value}
              type="button"
              onClick={() => setSelectedCategory(category.value)}
              className={`inline-flex items-center px-3 py-1.5 text-sm font-medium rounded-full transition-colors ${
                selectedCategory === category.value
                  ? 'bg-blue-100 text-blue-700'
                  : 'bg-gray-100 text-gray-600 hover:bg-gray-200'
              }`}
            >
              <span className="mr-1.5">{category.icon}</span>
              {category.label}
            </button>
          ))}
        </div>
      </div>

      {/* Content */}
      {error && (
        <div className="bg-red-50 border border-red-200 rounded-lg p-4 mb-6">
          <p className="text-red-700">Failed to load templates. Please try again.</p>
        </div>
      )}

      {isLoading ? (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {skeletonKeys.map((key) => (
            <div
              key={key}
              className="bg-white rounded-lg shadow-sm border border-gray-200 p-5 animate-pulse"
            >
              <div className="flex items-start gap-3 mb-4">
                <div className="w-10 h-10 bg-gray-200 rounded-full" />
                <div className="flex-1">
                  <div className="h-5 bg-gray-200 rounded w-2/3 mb-2" />
                  <div className="h-4 bg-gray-200 rounded w-full" />
                </div>
              </div>
              <div className="h-4 bg-gray-200 rounded w-1/2 mb-4" />
              <div className="flex gap-2">
                <div className="flex-1 h-9 bg-gray-200 rounded" />
                <div className="flex-1 h-9 bg-gray-200 rounded" />
              </div>
            </div>
          ))}
        </div>
      ) : templates.length === 0 ? (
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
              d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"
            />
          </svg>
          <h3 className="mt-4 text-lg font-medium text-gray-900">No templates found</h3>
          <p className="mt-2 text-sm text-gray-500">
            {searchQuery || selectedCategory !== 'all'
              ? 'Try adjusting your filters or search query.'
              : 'Templates will appear here once they are available.'}
          </p>
          {(searchQuery || selectedCategory !== 'all') && (
            <button
              type="button"
              onClick={() => {
                setSearchQuery('');
                setSelectedCategory('all');
              }}
              className="mt-4 text-sm text-blue-600 hover:text-blue-700"
            >
              Clear filters
            </button>
          )}
        </div>
      ) : (
        <>
          <p className="text-sm text-gray-500 mb-4">
            Showing {templates.length} of {totalTemplates} templates
          </p>
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {templates.map((template) => (
              <TemplateCard
                key={template.id}
                template={template}
                onUse={handleUseTemplate}
                onPreview={handlePreview}
              />
            ))}
          </div>
        </>
      )}

      {/* Creating from template loading overlay */}
      {createFromTemplate.isPending && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg p-6 flex items-center gap-3">
            <svg
              className="animate-spin h-5 w-5 text-blue-600"
              fill="none"
              viewBox="0 0 24 24"
              aria-hidden="true"
            >
              <circle
                className="opacity-25"
                cx="12"
                cy="12"
                r="10"
                stroke="currentColor"
                strokeWidth="4"
              />
              <path
                className="opacity-75"
                fill="currentColor"
                d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
              />
            </svg>
            <span className="text-gray-700">Creating automation from template...</span>
          </div>
        </div>
      )}

      {/* Preview Modal */}
      {previewTemplate && (
        <TemplatePreviewModal
          template={previewTemplate}
          onClose={() => setPreviewTemplate(null)}
          onUse={(template) => {
            setPreviewTemplate(null);
            handleUseTemplate(template);
          }}
        />
      )}
    </div>
  );
}
