/**
 * LeaseDetailPage - View lease details, amendments, and payments.
 * Epic 19: Lease Management & Tenant Screening
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { PaymentHistoryTable } from '../components/PaymentHistoryTable';
import { RequestSignatureModal } from '../components/RequestSignatureModal';
import { SignatureStatusBadge } from '../components/SignatureStatusBadge';
import type {
  LeaseSignatureStatus,
  LeaseStatus,
  LeaseWithDetails,
  ViolationSummary,
} from '../types';

interface LeaseDetailPageProps {
  lease: LeaseWithDetails;
  violations?: ViolationSummary[];
  isLoading?: boolean;
  onBack: () => void;
  onEdit: (id: string) => void;
  onRenew: (id: string) => void;
  onTerminate: (id: string) => void;
  onAddAmendment: (id: string) => void;
  onRecordPayment: (paymentId: string) => void;
  onViewViolations?: (leaseId: string) => void;
  onReportViolation?: (leaseId: string) => void;
}

type TabType = 'overview' | 'amendments' | 'payments' | 'reminders' | 'violations';

const statusColors: Record<LeaseStatus, string> = {
  draft: 'bg-gray-100 text-gray-800',
  pending_signatures: 'bg-yellow-100 text-yellow-800',
  active: 'bg-green-100 text-green-800',
  expiring_soon: 'bg-orange-100 text-orange-800',
  expired: 'bg-red-100 text-red-800',
  terminated: 'bg-gray-100 text-gray-800',
  renewed: 'bg-blue-100 text-blue-800',
};

export function LeaseDetailPage({
  lease,
  violations = [],
  isLoading,
  onBack,
  onEdit,
  onRenew,
  onTerminate,
  onAddAmendment,
  onRecordPayment,
  onViewViolations,
  onReportViolation,
}: LeaseDetailPageProps) {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<TabType>('overview');
  const [showSignatureModal, setShowSignatureModal] = useState(false);
  const [localSignatureStatus, setLocalSignatureStatus] = useState<
    LeaseSignatureStatus | undefined
  >(lease.signatureStatus);

  // Derived signature state: prefer local override (set after a successful request)
  const signatureStatus = localSignatureStatus ?? lease.signatureStatus;
  const canRequestSignature =
    !!lease.documentId &&
    (!signatureStatus || signatureStatus === 'expired' || signatureStatus === 'cancelled');

  const signerParties = [
    {
      name: lease.tenant.name,
      email: lease.tenant.email,
      role: t('leases.esign.role.tenant', 'Tenant'),
    },
  ];

  const formatDate = (dateString?: string) => {
    if (!dateString) return '-';
    return new Date(dateString).toLocaleDateString();
  };

  const formatCurrency = (amount: number, currency: string) => {
    return new Intl.NumberFormat(undefined, {
      style: 'currency',
      currency,
    }).format(amount);
  };

  const canEdit = lease.status === 'draft' || lease.status === 'pending_signatures';
  const canRenew = lease.status === 'active' || lease.status === 'expiring_soon';
  const canTerminate = lease.status === 'active' || lease.status === 'expiring_soon';

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-12">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
      </div>
    );
  }

  const openViolationsCount = violations.filter((v) => v.status === 'open').length;

  const tabs: { key: TabType; label: string; count?: number }[] = [
    { key: 'overview', label: t('leases.detail.tabs.overview') },
    {
      key: 'amendments',
      label: t('leases.detail.tabs.amendments'),
      count: lease.amendments.length,
    },
    {
      key: 'payments',
      label: t('leases.detail.tabs.payments'),
      count: lease.upcomingPayments.length,
    },
    { key: 'reminders', label: t('leases.detail.tabs.reminders'), count: lease.reminders.length },
    { key: 'violations', label: t('leases.detail.tabs.violations'), count: violations.length },
  ];

  return (
    <div className="max-w-4xl mx-auto px-4 py-8">
      {/* Back Button */}
      <button
        type="button"
        onClick={onBack}
        className="flex items-center gap-2 text-gray-600 hover:text-gray-800 mb-6"
      >
        <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
        </svg>
        {t('leases.detail.backToLeases')}
      </button>

      {/* Header */}
      <div className="bg-white rounded-lg shadow p-6 mb-6">
        <div className="flex items-start justify-between">
          <div>
            <div className="flex items-center gap-3 flex-wrap">
              <h1 className="text-2xl font-bold text-gray-900">
                {lease.unit.buildingName} - {lease.unit.number}
              </h1>
              <span
                className={`px-3 py-1 text-sm font-medium rounded-full ${statusColors[lease.status]}`}
              >
                {t(`leases.status.${lease.status}`)}
              </span>
              {signatureStatus && <SignatureStatusBadge status={signatureStatus} />}
            </div>
            <p className="text-gray-600 mt-1">{lease.tenant.name}</p>
            <p className="text-sm text-gray-500">{lease.tenant.email}</p>
            {lease.tenant.phone && <p className="text-sm text-gray-500">{lease.tenant.phone}</p>}
          </div>
          <div className="flex gap-2 flex-wrap">
            {canEdit && (
              <button
                type="button"
                onClick={() => onEdit(lease.id)}
                className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50"
              >
                {t('common.edit')}
              </button>
            )}
            {canRequestSignature && (
              <button
                type="button"
                onClick={() => setShowSignatureModal(true)}
                className="px-4 py-2 text-sm font-medium text-white bg-indigo-600 hover:bg-indigo-700 rounded-md inline-flex items-center gap-2"
              >
                <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20" aria-hidden="true">
                  <path d="M13.586 3.586a2 2 0 112.828 2.828l-.793.793-2.828-2.828.793-.793zM11.379 5.793L3 14.172V17h2.828l8.38-8.379-2.83-2.828z" />
                </svg>
                {t('leases.esign.requestSignature', 'Request Signature')}
              </button>
            )}
            {canRenew && (
              <button
                type="button"
                onClick={() => onRenew(lease.id)}
                className="px-4 py-2 text-sm font-medium text-white bg-green-600 hover:bg-green-700 rounded-md"
              >
                {t('leases.renew')}
              </button>
            )}
            {canTerminate && (
              <button
                type="button"
                onClick={() => onTerminate(lease.id)}
                className="px-4 py-2 text-sm font-medium text-white bg-red-600 hover:bg-red-700 rounded-md"
              >
                {t('leases.terminate')}
              </button>
            )}
          </div>
        </div>

        {/* Key Details Grid */}
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mt-6 pt-6 border-t">
          <div>
            <p className="text-sm text-gray-500">{t('leases.detail.startDate')}</p>
            <p className="font-medium">{formatDate(lease.startDate)}</p>
          </div>
          <div>
            <p className="text-sm text-gray-500">{t('leases.detail.endDate')}</p>
            <p className="font-medium">{formatDate(lease.endDate)}</p>
          </div>
          <div>
            <p className="text-sm text-gray-500">{t('leases.detail.monthlyRent')}</p>
            <p className="font-medium">{formatCurrency(lease.rentAmount, lease.currency)}</p>
          </div>
          <div>
            <p className="text-sm text-gray-500">{t('leases.detail.deposit')}</p>
            <p className="font-medium">
              {lease.depositAmount ? formatCurrency(lease.depositAmount, lease.currency) : '-'}
            </p>
          </div>
        </div>
      </div>

      {/* E-Signature modal */}
      {showSignatureModal && lease.documentId && (
        <RequestSignatureModal
          documentId={lease.documentId}
          parties={signerParties}
          onSuccess={(newStatus) => {
            setLocalSignatureStatus(newStatus);
            setShowSignatureModal(false);
          }}
          onClose={() => setShowSignatureModal(false)}
        />
      )}

      {/* Tabs */}
      <div className="bg-white rounded-lg shadow">
        <div className="border-b">
          <nav className="flex -mb-px">
            {tabs.map((tab) => (
              <button
                key={tab.key}
                type="button"
                onClick={() => setActiveTab(tab.key)}
                className={`px-6 py-3 text-sm font-medium border-b-2 ${
                  activeTab === tab.key
                    ? 'border-blue-500 text-blue-600'
                    : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
                }`}
              >
                {tab.label}
                {tab.count !== undefined && tab.count > 0 && (
                  <span className="ml-2 px-2 py-0.5 text-xs bg-gray-100 rounded-full">
                    {tab.count}
                  </span>
                )}
              </button>
            ))}
          </nav>
        </div>

        <div className="p-6">
          {/* Overview Tab */}
          {activeTab === 'overview' && (
            <div className="space-y-6">
              <div>
                <h3 className="text-lg font-medium text-gray-900 mb-3">
                  {t('leases.detail.leaseDetails')}
                </h3>
                <div className="grid grid-cols-2 gap-4">
                  <div>
                    <p className="text-sm text-gray-500">{t('leases.detail.paymentDay')}</p>
                    <p className="font-medium">
                      {t('leases.detail.dayOfMonth', { day: lease.paymentDayOfMonth })}
                    </p>
                  </div>
                  <div>
                    <p className="text-sm text-gray-500">{t('leases.detail.signedDate')}</p>
                    <p className="font-medium">{formatDate(lease.signedAt)}</p>
                  </div>
                </div>
              </div>

              {lease.terms && (
                <div>
                  <h3 className="text-lg font-medium text-gray-900 mb-3">
                    {t('leases.detail.terms')}
                  </h3>
                  <div className="bg-gray-50 rounded-lg p-4">
                    <pre className="text-sm text-gray-700 whitespace-pre-wrap">{lease.terms}</pre>
                  </div>
                </div>
              )}

              {lease.notes && (
                <div>
                  <h3 className="text-lg font-medium text-gray-900 mb-3">
                    {t('leases.detail.notes')}
                  </h3>
                  <p className="text-gray-700">{lease.notes}</p>
                </div>
              )}

              {lease.terminatedAt && (
                <div className="bg-red-50 rounded-lg p-4">
                  <h3 className="text-lg font-medium text-red-800 mb-2">
                    {t('leases.detail.termination')}
                  </h3>
                  <p className="text-sm text-red-700">
                    {t('leases.detail.terminatedOn', { date: formatDate(lease.terminatedAt) })}
                  </p>
                  {lease.terminationReason && (
                    <p className="text-sm text-red-700 mt-1">{lease.terminationReason}</p>
                  )}
                </div>
              )}
            </div>
          )}

          {/* Amendments Tab */}
          {activeTab === 'amendments' && (
            <div>
              <div className="flex items-center justify-between mb-4">
                <h3 className="text-lg font-medium text-gray-900">
                  {t('leases.detail.amendments')}
                </h3>
                {canEdit && (
                  <button
                    type="button"
                    onClick={() => onAddAmendment(lease.id)}
                    className="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded-md"
                  >
                    {t('leases.detail.addAmendment')}
                  </button>
                )}
              </div>
              {lease.amendments.length === 0 ? (
                <p className="text-gray-500 text-center py-8">{t('leases.detail.noAmendments')}</p>
              ) : (
                <div className="space-y-4">
                  {lease.amendments.map((amendment) => (
                    <div key={amendment.id} className="border rounded-lg p-4">
                      <div className="flex items-start justify-between">
                        <div>
                          <p className="font-medium text-gray-900">{amendment.description}</p>
                          <p className="text-sm text-gray-500 mt-1">
                            {t('leases.detail.effectiveDate')}:{' '}
                            {formatDate(amendment.effectiveDate)}
                          </p>
                        </div>
                      </div>
                      {(amendment.previousRent || amendment.newRent) && (
                        <div className="mt-3 flex items-center gap-4 text-sm">
                          {amendment.previousRent && (
                            <span className="text-gray-500 line-through">
                              {formatCurrency(amendment.previousRent, lease.currency)}
                            </span>
                          )}
                          {amendment.newRent && (
                            <span className="text-green-600 font-medium">
                              {formatCurrency(amendment.newRent, lease.currency)}
                            </span>
                          )}
                        </div>
                      )}
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}

          {/* Payments Tab */}
          {activeTab === 'payments' && (
            <div>
              <h3 className="text-lg font-medium text-gray-900 mb-4">
                {t('leases.detail.paymentHistory')}
              </h3>
              <PaymentHistoryTable
                payments={lease.upcomingPayments}
                onRecordPayment={onRecordPayment}
              />
            </div>
          )}

          {/* Reminders Tab */}
          {activeTab === 'reminders' && (
            <div>
              <h3 className="text-lg font-medium text-gray-900 mb-4">
                {t('leases.detail.reminders')}
              </h3>
              {lease.reminders.length === 0 ? (
                <p className="text-gray-500 text-center py-8">{t('leases.detail.noReminders')}</p>
              ) : (
                <div className="space-y-3">
                  {lease.reminders.map((reminder) => (
                    <div
                      key={reminder.id}
                      className={`border rounded-lg p-4 ${
                        reminder.isSent ? 'bg-gray-50' : 'bg-white'
                      }`}
                    >
                      <div className="flex items-start justify-between">
                        <div>
                          <p className="font-medium text-gray-900">{reminder.reminderType}</p>
                          {reminder.message && (
                            <p className="text-sm text-gray-600 mt-1">{reminder.message}</p>
                          )}
                          <p className="text-sm text-gray-500 mt-2">
                            {t('leases.detail.scheduledFor')}: {formatDate(reminder.scheduledFor)}
                          </p>
                        </div>
                        <span
                          className={`px-2 py-1 text-xs font-medium rounded ${
                            reminder.isSent
                              ? 'bg-green-100 text-green-800'
                              : 'bg-yellow-100 text-yellow-800'
                          }`}
                        >
                          {reminder.isSent ? t('leases.detail.sent') : t('leases.detail.pending')}
                        </span>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}

          {/* Violations Tab */}
          {activeTab === 'violations' && (
            <div>
              <div className="flex items-center justify-between mb-4">
                <h3 className="text-lg font-medium text-gray-900">
                  {t('leases.detail.violationsSection')}
                </h3>
                <div className="flex gap-2">
                  {onReportViolation && (
                    <button
                      type="button"
                      onClick={() => onReportViolation(lease.id)}
                      className="px-4 py-2 text-sm font-medium text-white bg-red-600 hover:bg-red-700 rounded-md"
                    >
                      {t('leases.detail.reportViolation')}
                    </button>
                  )}
                  {violations.length > 0 && onViewViolations && (
                    <button
                      type="button"
                      onClick={() => onViewViolations(lease.id)}
                      className="px-4 py-2 text-sm font-medium text-blue-600 hover:text-blue-800 border border-blue-600 rounded-md"
                    >
                      {t('leases.detail.viewAllViolations')}
                    </button>
                  )}
                </div>
              </div>

              {/* Violations Summary */}
              {openViolationsCount > 0 && (
                <div className="mb-4 p-4 bg-red-50 border border-red-200 rounded-lg">
                  <div className="flex items-center gap-2">
                    <svg className="w-5 h-5 text-red-600" fill="currentColor" viewBox="0 0 20 20">
                      <path
                        fillRule="evenodd"
                        d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z"
                        clipRule="evenodd"
                      />
                    </svg>
                    <p className="text-sm font-medium text-red-800">
                      {t('leases.detail.openViolationsWarning', { count: openViolationsCount })}
                    </p>
                  </div>
                </div>
              )}

              {violations.length === 0 ? (
                <div className="text-center py-8">
                  <svg
                    className="mx-auto h-12 w-12 text-gray-400"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"
                    />
                  </svg>
                  <p className="mt-2 text-sm text-gray-500">{t('leases.detail.noViolations')}</p>
                </div>
              ) : (
                <div className="space-y-3">
                  {violations.slice(0, 5).map((violation) => (
                    <div key={violation.id} className="border rounded-lg p-4">
                      <div className="flex items-start justify-between">
                        <div>
                          <div className="flex items-center gap-2">
                            <p className="font-medium text-gray-900">
                              {t(`leases.violations.type.${violation.violationType}`)}
                            </p>
                            <span
                              className={`px-2 py-0.5 text-xs font-medium rounded ${
                                violation.status === 'open'
                                  ? 'bg-red-100 text-red-800'
                                  : violation.status === 'resolved'
                                    ? 'bg-green-100 text-green-800'
                                    : violation.status === 'disputed'
                                      ? 'bg-yellow-100 text-yellow-800'
                                      : 'bg-gray-100 text-gray-800'
                              }`}
                            >
                              {t(`leases.violations.status.${violation.status}`)}
                            </span>
                            <span
                              className={`px-2 py-0.5 text-xs font-medium rounded ${
                                violation.severity === 'minor'
                                  ? 'bg-blue-100 text-blue-800'
                                  : violation.severity === 'moderate'
                                    ? 'bg-orange-100 text-orange-800'
                                    : 'bg-red-100 text-red-800'
                              }`}
                            >
                              {t(`leases.violations.severity.${violation.severity}`)}
                            </span>
                          </div>
                          <p className="text-sm text-gray-500 mt-1">
                            {formatDate(violation.violationDate)}
                          </p>
                        </div>
                      </div>
                    </div>
                  ))}
                  {violations.length > 5 && (
                    <p className="text-sm text-gray-500 text-center">
                      {t('leases.detail.moreViolations', { count: violations.length - 5 })}
                    </p>
                  )}
                </div>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
