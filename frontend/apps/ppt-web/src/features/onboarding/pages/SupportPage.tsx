/**
 * SupportPage - support and bug report page.
 * UC-42: Onboarding/Help Feature
 */

import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { getActiveLocale } from '../../../lib/locale';
import type { BugReportFormData, SupportTicket, SupportTicketPriority, SystemInfo } from '../types';
import { getSystemInfo } from '../types';

export interface SupportPageProps {
  tickets: SupportTicket[];
  isLoading?: boolean;
  isSubmitting?: boolean;
  supportEmail?: string;
  supportPhone?: string;
  onSubmitBugReport: (data: BugReportFormData) => void;
  onViewTicket: (ticketId: string) => void;
  onNavigateBack: () => void;
}

export function SupportPage({
  tickets,
  isLoading,
  isSubmitting,
  supportEmail = 'support@example.com',
  supportPhone = '+1 (555) 123-4567',
  onSubmitBugReport,
  onViewTicket,
  onNavigateBack,
}: SupportPageProps) {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<'chat' | 'report' | 'history'>('report');
  const [systemInfo, setSystemInfo] = useState<SystemInfo | null>(null);
  const [formData, setFormData] = useState<BugReportFormData>({
    subject: '',
    description: '',
    stepsToReproduce: '',
    expectedBehavior: '',
    actualBehavior: '',
    priority: 'medium',
    attachments: undefined,
  });
  const [errors, setErrors] = useState<Partial<Record<keyof BugReportFormData, string>>>({});

  useEffect(() => {
    setSystemInfo(getSystemInfo());
  }, []);

  const priorities: { value: SupportTicketPriority; label: string; color: string }[] = [
    { value: 'low', label: t('help.support.priorityLow'), color: 'text-green-600 bg-green-50' },
    {
      value: 'medium',
      label: t('help.support.priorityMedium'),
      color: 'text-yellow-600 bg-yellow-50',
    },
    { value: 'high', label: t('help.support.priorityHigh'), color: 'text-orange-600 bg-orange-50' },
    { value: 'urgent', label: t('help.support.priorityUrgent'), color: 'text-red-600 bg-red-50' },
  ];

  const validate = (): boolean => {
    const newErrors: Partial<Record<keyof BugReportFormData, string>> = {};

    if (!formData.subject.trim()) {
      newErrors.subject = t('help.support.errors.subjectRequired');
    }

    if (!formData.description.trim()) {
      newErrors.description = t('help.support.errors.descriptionRequired');
    }

    if (!formData.stepsToReproduce.trim()) {
      newErrors.stepsToReproduce = t('help.support.errors.stepsRequired');
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (validate()) {
      onSubmitBugReport(formData);
    }
  };

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const files = e.target.files;
    if (files) {
      setFormData((prev) => ({
        ...prev,
        attachments: Array.from(files),
      }));
    }
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-12">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
      </div>
    );
  }

  return (
    <div className="max-w-4xl mx-auto px-4 py-8">
      {/* Header */}
      <div className="mb-6">
        <button
          type="button"
          onClick={onNavigateBack}
          className="flex items-center text-sm text-gray-600 hover:text-gray-800 mb-4"
        >
          <svg className="w-4 h-4 mr-1" fill="currentColor" viewBox="0 0 20 20">
            <title>{t('common.back')}</title>
            <path
              fillRule="evenodd"
              d="M9.707 16.707a1 1 0 01-1.414 0l-6-6a1 1 0 010-1.414l6-6a1 1 0 011.414 1.414L5.414 9H17a1 1 0 110 2H5.414l4.293 4.293a1 1 0 010 1.414z"
              clipRule="evenodd"
            />
          </svg>
          {t('help.backToHelpCenter')}
        </button>

        <h1 className="text-2xl font-bold text-gray-900">{t('help.support.title')}</h1>
        <p className="mt-1 text-sm text-gray-500">{t('help.support.subtitle')}</p>
      </div>

      {/* Contact info cards */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-6">
        <div className="bg-white rounded-lg border border-gray-200 p-4">
          <div className="flex items-center">
            <div className="p-2 bg-blue-100 text-blue-600 rounded-lg mr-3">
              <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <title>{t('help.support.email')}</title>
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M3 8l7.89 5.26a2 2 0 002.22 0L21 8M5 19h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"
                />
              </svg>
            </div>
            <div>
              <p className="text-sm text-gray-500">{t('help.support.email')}</p>
              <a href={`mailto:${supportEmail}`} className="text-blue-600 hover:underline">
                {supportEmail}
              </a>
            </div>
          </div>
        </div>
        <div className="bg-white rounded-lg border border-gray-200 p-4">
          <div className="flex items-center">
            <div className="p-2 bg-green-100 text-green-600 rounded-lg mr-3">
              <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <title>{t('help.support.phone')}</title>
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M3 5a2 2 0 012-2h3.28a1 1 0 01.948.684l1.498 4.493a1 1 0 01-.502 1.21l-2.257 1.13a11.042 11.042 0 005.516 5.516l1.13-2.257a1 1 0 011.21-.502l4.493 1.498a1 1 0 01.684.949V19a2 2 0 01-2 2h-1C9.716 21 3 14.284 3 6V5z"
                />
              </svg>
            </div>
            <div>
              <p className="text-sm text-gray-500">{t('help.support.phone')}</p>
              <a href={`tel:${supportPhone}`} className="text-green-600 hover:underline">
                {supportPhone}
              </a>
            </div>
          </div>
        </div>
      </div>

      {/* Tab navigation */}
      <div className="border-b border-gray-200 mb-6">
        <nav className="flex gap-6">
          <TabButton
            isActive={activeTab === 'chat'}
            onClick={() => setActiveTab('chat')}
            label={t('help.support.liveChat')}
          />
          <TabButton
            isActive={activeTab === 'report'}
            onClick={() => setActiveTab('report')}
            label={t('help.support.reportBug')}
          />
          <TabButton
            isActive={activeTab === 'history'}
            onClick={() => setActiveTab('history')}
            label={t('help.support.ticketHistory')}
            count={tickets.length}
          />
        </nav>
      </div>

      {/* Tab content */}
      {activeTab === 'chat' && (
        <div className="bg-gray-50 rounded-lg p-8 text-center">
          <svg
            className="mx-auto h-16 w-16 text-gray-400 mb-4"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <title>{t('help.support.liveChat')}</title>
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={1.5}
              d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z"
            />
          </svg>
          <h3 className="text-lg font-medium text-gray-900 mb-2">
            {t('help.support.liveChatTitle')}
          </h3>
          <p className="text-gray-600 mb-4">{t('help.support.liveChatDescription')}</p>
          <button
            type="button"
            className="px-6 py-2 bg-blue-600 text-white font-medium rounded-md hover:bg-blue-700"
          >
            {t('help.support.startChat')}
          </button>
          <p className="text-sm text-gray-500 mt-4">{t('help.support.chatAvailability')}</p>
        </div>
      )}

      {activeTab === 'report' && (
        <div className="bg-white rounded-lg border border-gray-200 p-6">
          <h3 className="text-lg font-medium text-gray-900 mb-4">
            {t('help.support.reportBugTitle')}
          </h3>

          <form onSubmit={handleSubmit} className="space-y-4">
            {/* Subject */}
            <div>
              <label htmlFor="subject" className="block text-sm font-medium text-gray-700 mb-1">
                {t('help.support.subject')}
              </label>
              <input
                type="text"
                id="subject"
                value={formData.subject}
                onChange={(e) => setFormData((prev) => ({ ...prev, subject: e.target.value }))}
                placeholder={t('help.support.subjectPlaceholder')}
                className={`w-full px-4 py-2 border rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 ${
                  errors.subject ? 'border-red-300' : 'border-gray-300'
                }`}
              />
              {errors.subject && <p className="mt-1 text-sm text-red-600">{errors.subject}</p>}
            </div>

            {/* Description */}
            <div>
              <label htmlFor="description" className="block text-sm font-medium text-gray-700 mb-1">
                {t('help.support.description')}
              </label>
              <textarea
                id="description"
                value={formData.description}
                onChange={(e) => setFormData((prev) => ({ ...prev, description: e.target.value }))}
                placeholder={t('help.support.descriptionPlaceholder')}
                rows={3}
                className={`w-full px-4 py-2 border rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 ${
                  errors.description ? 'border-red-300' : 'border-gray-300'
                }`}
              />
              {errors.description && (
                <p className="mt-1 text-sm text-red-600">{errors.description}</p>
              )}
            </div>

            {/* Steps to reproduce */}
            <div>
              <label
                htmlFor="stepsToReproduce"
                className="block text-sm font-medium text-gray-700 mb-1"
              >
                {t('help.support.stepsToReproduce')}
              </label>
              <textarea
                id="stepsToReproduce"
                value={formData.stepsToReproduce}
                onChange={(e) =>
                  setFormData((prev) => ({ ...prev, stepsToReproduce: e.target.value }))
                }
                placeholder={t('help.support.stepsPlaceholder')}
                rows={3}
                className={`w-full px-4 py-2 border rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 ${
                  errors.stepsToReproduce ? 'border-red-300' : 'border-gray-300'
                }`}
              />
              {errors.stepsToReproduce && (
                <p className="mt-1 text-sm text-red-600">{errors.stepsToReproduce}</p>
              )}
            </div>

            {/* Expected vs Actual */}
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div>
                <label
                  htmlFor="expectedBehavior"
                  className="block text-sm font-medium text-gray-700 mb-1"
                >
                  {t('help.support.expectedBehavior')}
                </label>
                <textarea
                  id="expectedBehavior"
                  value={formData.expectedBehavior}
                  onChange={(e) =>
                    setFormData((prev) => ({ ...prev, expectedBehavior: e.target.value }))
                  }
                  placeholder={t('help.support.expectedPlaceholder')}
                  rows={2}
                  className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
              </div>
              <div>
                <label
                  htmlFor="actualBehavior"
                  className="block text-sm font-medium text-gray-700 mb-1"
                >
                  {t('help.support.actualBehavior')}
                </label>
                <textarea
                  id="actualBehavior"
                  value={formData.actualBehavior}
                  onChange={(e) =>
                    setFormData((prev) => ({ ...prev, actualBehavior: e.target.value }))
                  }
                  placeholder={t('help.support.actualPlaceholder')}
                  rows={2}
                  className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
              </div>
            </div>

            {/* Priority */}
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                {t('help.support.priorityLabel')}
              </label>
              <div className="flex flex-wrap gap-2">
                {priorities.map((priority) => (
                  <button
                    key={priority.value}
                    type="button"
                    onClick={() => setFormData((prev) => ({ ...prev, priority: priority.value }))}
                    className={`px-3 py-1.5 text-sm font-medium rounded-md border transition-all ${
                      formData.priority === priority.value
                        ? `${priority.color} border-current`
                        : 'border-gray-300 text-gray-600 hover:border-gray-400'
                    }`}
                  >
                    {priority.label}
                  </button>
                ))}
              </div>
            </div>

            {/* System info */}
            {systemInfo && (
              <div className="bg-gray-50 rounded-lg p-4">
                <h4 className="text-sm font-medium text-gray-700 mb-2">
                  {t('help.support.systemInfo')}
                </h4>
                <div className="grid grid-cols-2 gap-2 text-sm text-gray-600">
                  <div>
                    <span className="text-gray-500">{t('help.support.browser')}:</span>{' '}
                    {systemInfo.browser}
                  </div>
                  <div>
                    <span className="text-gray-500">{t('help.support.os')}:</span> {systemInfo.os}
                  </div>
                  <div>
                    <span className="text-gray-500">{t('help.support.resolution')}:</span>{' '}
                    {systemInfo.screenResolution}
                  </div>
                </div>
              </div>
            )}

            {/* Attachments */}
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                {t('help.support.attachments')} ({t('common.optional')})
              </label>
              <input
                type="file"
                id="attachments"
                accept="image/*,.pdf,.txt,.log"
                multiple
                onChange={handleFileChange}
                className="w-full text-sm text-gray-500 file:mr-4 file:py-2 file:px-4 file:rounded-md file:border-0 file:text-sm file:font-medium file:bg-blue-50 file:text-blue-700 hover:file:bg-blue-100"
              />
              {formData.attachments && formData.attachments.length > 0 && (
                <p className="mt-1 text-sm text-gray-500">
                  {formData.attachments.length} {t('help.feedback.filesSelected')}
                </p>
              )}
            </div>

            {/* Submit */}
            <div className="flex justify-end pt-4">
              <button
                type="submit"
                disabled={isSubmitting}
                className="px-6 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {isSubmitting ? t('common.loading') : t('help.support.submitReport')}
              </button>
            </div>
          </form>
        </div>
      )}

      {activeTab === 'history' && (
        <div>
          {tickets.length === 0 ? (
            <div className="text-center py-12 bg-white rounded-lg border border-gray-200">
              <svg
                className="mx-auto h-12 w-12 text-gray-400"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <title>{t('help.support.noTickets')}</title>
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={1.5}
                  d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2"
                />
              </svg>
              <p className="mt-4 text-gray-500">{t('help.support.noTickets')}</p>
              <p className="mt-1 text-sm text-gray-400">{t('help.support.noTicketsDescription')}</p>
            </div>
          ) : (
            <div className="space-y-3">
              {tickets.map((ticket) => (
                <TicketCard key={ticket.id} ticket={ticket} onView={onViewTicket} />
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

interface TabButtonProps {
  isActive: boolean;
  onClick: () => void;
  label: string;
  count?: number;
}

function TabButton({ isActive, onClick, label, count }: TabButtonProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`pb-3 text-sm font-medium border-b-2 transition-colors ${
        isActive
          ? 'border-blue-500 text-blue-600'
          : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
      }`}
    >
      {label}
      {count !== undefined && count > 0 && (
        <span className="ml-2 px-2 py-0.5 text-xs bg-gray-100 text-gray-600 rounded-full">
          {count}
        </span>
      )}
    </button>
  );
}

interface TicketCardProps {
  ticket: SupportTicket;
  onView: (ticketId: string) => void;
}

function TicketCard({ ticket, onView }: TicketCardProps) {
  const { t } = useTranslation();

  const statusStyles: Record<string, string> = {
    open: 'bg-blue-100 text-blue-800',
    in_progress: 'bg-yellow-100 text-yellow-800',
    waiting_response: 'bg-purple-100 text-purple-800',
    resolved: 'bg-green-100 text-green-800',
    closed: 'bg-gray-100 text-gray-800',
  };

  const priorityStyles: Record<string, string> = {
    low: 'text-green-600',
    medium: 'text-yellow-600',
    high: 'text-orange-600',
    urgent: 'text-red-600',
  };

  return (
    <div className="bg-white rounded-lg border border-gray-200 p-4 hover:shadow-sm transition-shadow">
      <div className="flex items-start justify-between">
        <div className="flex-1">
          <div className="flex items-center gap-2 mb-1">
            <span className="text-sm text-gray-500">#{ticket.ticketNumber}</span>
            <span
              className={`px-2 py-0.5 text-xs font-medium rounded ${statusStyles[ticket.status]}`}
            >
              {t(`help.support.status.${ticket.status}`)}
            </span>
            <span className={`text-xs font-medium ${priorityStyles[ticket.priority]}`}>
              {t(`help.support.priority.${ticket.priority}`)}
            </span>
          </div>
          <h4 className="font-medium text-gray-900">{ticket.subject}</h4>
          <p className="text-sm text-gray-500 mt-1 line-clamp-1">{ticket.description}</p>
          <p className="text-xs text-gray-400 mt-2">
            {t('help.support.created')}:{' '}
            {new Date(ticket.createdAt).toLocaleDateString(getActiveLocale())}
          </p>
        </div>
        <button
          type="button"
          onClick={() => onView(ticket.id)}
          className="ml-4 px-3 py-1 text-sm text-blue-600 hover:bg-blue-50 rounded-md"
        >
          {t('common.view')}
        </button>
      </div>
    </div>
  );
}
