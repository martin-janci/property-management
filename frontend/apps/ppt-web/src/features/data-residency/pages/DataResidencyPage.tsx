/**
 * Data Residency Dashboard Page (Epic 146).
 *
 * Main dashboard for data residency configuration, compliance verification,
 * and audit trail viewing.
 */

import type React from 'react';
import { useCallback, useState } from 'react';
import type {
  AuditLogEntry,
  ComplianceVerificationResult,
  DataResidencyConfig,
} from '../components';
import { AuditLogCard, ComplianceVerificationCard, DataResidencyConfigCard } from '../components';

// Tab definitions
type TabId = 'overview' | 'verification' | 'audit';

interface Tab {
  id: TabId;
  label: string;
  description: string;
}

const tabs: Tab[] = [
  {
    id: 'overview',
    label: 'Configuration',
    description: 'View and manage data residency settings',
  },
  {
    id: 'verification',
    label: 'Compliance Verification',
    description: 'Verify data residency compliance',
  },
  {
    id: 'audit',
    label: 'Audit Trail',
    description: 'Review data residency changes',
  },
];

// Mock data - In production, this would come from API hooks
const mockConfig: DataResidencyConfig = {
  id: '550e8400-e29b-41d4-a716-446655440000',
  organization_id: '550e8400-e29b-41d4-a716-446655440001',
  primary_region: 'eu_west',
  primary_region_display: 'EU West (Frankfurt)',
  backup_region: 'eu_central',
  backup_region_display: 'EU Central (Paris)',
  status: 'active',
  allow_cross_region_access: false,
  compliance_frameworks: ['GDPR', 'EU Data Residency', 'Schrems II Compliant'],
  compliance_implications: [
    {
      level: 'info',
      title: 'GDPR Compliant Storage',
      description: 'Data stored in EU region satisfies GDPR data residency requirements.',
      regulation: 'GDPR Article 44-49',
    },
    {
      level: 'info',
      title: 'Backup Region Configured',
      description: 'Backup region is within EU, maintaining compliance during failover scenarios.',
    },
  ],
  last_verified_at: new Date().toISOString(),
  created_at: new Date(Date.now() - 30 * 24 * 60 * 60 * 1000).toISOString(),
  updated_at: new Date().toISOString(),
};

const mockVerificationResult: ComplianceVerificationResult = {
  id: '550e8400-e29b-41d4-a716-446655440002',
  organization_id: '550e8400-e29b-41d4-a716-446655440001',
  compliance_status: 'compliant',
  is_compliant: true,
  verified_at: new Date().toISOString(),
  data_locations: [
    {
      data_type: 'personal_data',
      region: 'eu_west',
      configured_region: 'eu_west',
      is_correct_location: true,
      record_count: 15420,
      last_updated: new Date().toISOString(),
    },
    {
      data_type: 'financial_data',
      region: 'eu_west',
      configured_region: 'eu_west',
      is_correct_location: true,
      record_count: 8934,
      last_updated: new Date().toISOString(),
    },
    {
      data_type: 'documents',
      region: 'eu_west',
      configured_region: 'eu_west',
      is_correct_location: true,
      record_count: 3567,
      last_updated: new Date().toISOString(),
    },
    {
      data_type: 'audit_logs',
      region: 'eu_west',
      configured_region: 'eu_west',
      is_correct_location: true,
      record_count: 125890,
      last_updated: new Date().toISOString(),
    },
    {
      data_type: 'communications',
      region: 'eu_west',
      configured_region: 'eu_west',
      is_correct_location: true,
      record_count: 45230,
      last_updated: new Date().toISOString(),
    },
  ],
  access_by_region: [
    {
      region: 'EU West (Frankfurt)',
      read_count: 152340,
      write_count: 23456,
      cross_region_count: 0,
      period: 'Last 24 hours',
    },
    {
      region: 'EU Central (Paris)',
      read_count: 0,
      write_count: 0,
      cross_region_count: 0,
      period: 'Last 24 hours',
    },
  ],
  issues: [],
  report_available: true,
};

const mockAuditEntries: AuditLogEntry[] = [
  {
    id: '550e8400-e29b-41d4-a716-446655440003',
    event_type: 'configuration_created',
    description: 'Data residency configuration created',
    user_id: '550e8400-e29b-41d4-a716-446655440010',
    user_name: 'Admin User',
    changes: [
      { field: 'primary_region', old_value: undefined, new_value: 'eu-west-1' },
      { field: 'backup_region', old_value: undefined, new_value: 'eu-central-1' },
    ],
    ip_address: '10.0.0.1',
    created_at: new Date(Date.now() - 30 * 24 * 60 * 60 * 1000).toISOString(),
    chain_valid: true,
  },
  {
    id: '550e8400-e29b-41d4-a716-446655440004',
    event_type: 'compliance_check_performed',
    description: 'Compliance verification completed - COMPLIANT',
    user_id: '550e8400-e29b-41d4-a716-446655440010',
    user_name: 'Admin User',
    details: {
      status: 'compliant',
      data_types_verified: 5,
      issues_found: 0,
    },
    ip_address: '10.0.0.1',
    created_at: new Date(Date.now() - 7 * 24 * 60 * 60 * 1000).toISOString(),
    chain_valid: true,
  },
  {
    id: '550e8400-e29b-41d4-a716-446655440005',
    event_type: 'compliance_check_performed',
    description: 'Compliance verification completed - COMPLIANT',
    user_id: '550e8400-e29b-41d4-a716-446655440010',
    user_name: 'Admin User',
    details: {
      status: 'compliant',
      data_types_verified: 5,
      issues_found: 0,
    },
    ip_address: '10.0.0.1',
    created_at: new Date().toISOString(),
    chain_valid: true,
  },
];

export const DataResidencyPage: React.FC = () => {
  const [activeTab, setActiveTab] = useState<TabId>('overview');
  const [isConfigModalOpen, setIsConfigModalOpen] = useState(false);
  const [isVerifying, setIsVerifying] = useState(false);

  // In production, these would be API mutations
  const handleEditConfig = useCallback(() => {
    setIsConfigModalOpen(true);
  }, []);

  const handleRunVerification = useCallback(async () => {
    setIsVerifying(true);
    // Simulate API call
    await new Promise((resolve) => setTimeout(resolve, 2000));
    setIsVerifying(false);
    setActiveTab('verification');
  }, []);

  const handleExportReport = useCallback(() => {
    // In production, this would trigger a download
    alert('Downloading compliance report...');
  }, []);

  const handleVerifyChain = useCallback(async () => {
    // In production, this would verify the audit chain
    alert('Audit chain verified successfully!');
  }, []);

  return (
    <div className="min-h-screen bg-gray-50">
      {/* Header */}
      <div className="bg-white border-b border-gray-200">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6">
          <div className="flex items-center justify-between">
            <div>
              <h1 className="text-2xl font-bold text-gray-900">Data Residency</h1>
              <p className="mt-1 text-sm text-gray-500">
                Configure where your data is stored to meet regional compliance requirements
              </p>
            </div>
            <div className="flex items-center space-x-3">
              <span
                className={`px-3 py-1 text-sm font-medium rounded-full ${
                  mockConfig.status === 'active'
                    ? 'bg-green-100 text-green-800'
                    : 'bg-yellow-100 text-yellow-800'
                }`}
              >
                {mockConfig.status === 'active' ? 'Active' : 'Pending'}
              </span>
            </div>
          </div>

          {/* Tabs */}
          <div className="mt-6 border-b border-gray-200">
            <nav className="-mb-px flex space-x-8">
              {tabs.map((tab) => (
                <button
                  key={tab.id}
                  type="button"
                  onClick={() => setActiveTab(tab.id)}
                  className={`whitespace-nowrap py-4 px-1 border-b-2 font-medium text-sm transition-colors ${
                    activeTab === tab.id
                      ? 'border-indigo-500 text-indigo-600'
                      : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
                  }`}
                >
                  {tab.label}
                </button>
              ))}
            </nav>
          </div>
        </div>
      </div>

      {/* Content */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {/* Overview Tab */}
        {activeTab === 'overview' && (
          <div className="space-y-6">
            {/* Quick Stats */}
            <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
              <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6">
                <div className="flex items-center justify-between">
                  <div>
                    <p className="text-sm font-medium text-gray-500">Primary Region</p>
                    <p className="mt-1 text-lg font-semibold text-gray-900">
                      {mockConfig.primary_region_display}
                    </p>
                  </div>
                  <div className="w-10 h-10 bg-indigo-100 rounded-lg flex items-center justify-center">
                    <svg
                      className="w-6 h-6 text-indigo-600"
                      fill="none"
                      stroke="currentColor"
                      viewBox="0 0 24 24"
                    >
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M3.055 11H5a2 2 0 012 2v1a2 2 0 002 2 2 2 0 012 2v2.945M8 3.935V5.5A2.5 2.5 0 0010.5 8h.5a2 2 0 012 2 2 2 0 104 0 2 2 0 012-2h1.064M15 20.488V18a2 2 0 012-2h3.064M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                      />
                    </svg>
                  </div>
                </div>
              </div>

              <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6">
                <div className="flex items-center justify-between">
                  <div>
                    <p className="text-sm font-medium text-gray-500">Compliance Status</p>
                    <p className="mt-1 text-lg font-semibold text-green-600">Compliant</p>
                  </div>
                  <div className="w-10 h-10 bg-green-100 rounded-lg flex items-center justify-center">
                    <svg
                      className="w-6 h-6 text-green-600"
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
                  </div>
                </div>
              </div>

              <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6">
                <div className="flex items-center justify-between">
                  <div>
                    <p className="text-sm font-medium text-gray-500">Cross-Region Access</p>
                    <p className="mt-1 text-lg font-semibold text-gray-900">
                      {mockConfig.allow_cross_region_access ? 'Enabled' : 'Disabled'}
                    </p>
                  </div>
                  <div className="w-10 h-10 bg-gray-100 rounded-lg flex items-center justify-center">
                    <svg
                      className="w-6 h-6 text-gray-600"
                      fill="none"
                      stroke="currentColor"
                      viewBox="0 0 24 24"
                    >
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"
                      />
                    </svg>
                  </div>
                </div>
              </div>
            </div>

            {/* Configuration Card */}
            <DataResidencyConfigCard
              config={mockConfig}
              onEdit={handleEditConfig}
              onVerify={handleRunVerification}
            />
          </div>
        )}

        {/* Verification Tab */}
        {activeTab === 'verification' && (
          <div className="space-y-6">
            {isVerifying ? (
              <div className="bg-white rounded-lg shadow-md border border-gray-200 p-12 text-center">
                <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-indigo-600 mx-auto" />
                <p className="mt-4 text-sm text-gray-500">Running compliance verification...</p>
              </div>
            ) : (
              <ComplianceVerificationCard
                result={mockVerificationResult}
                onExportReport={handleExportReport}
                onRunVerification={handleRunVerification}
              />
            )}
          </div>
        )}

        {/* Audit Tab */}
        {activeTab === 'audit' && (
          <div className="space-y-6">
            <AuditLogCard
              entries={mockAuditEntries}
              totalCount={mockAuditEntries.length}
              chainValid={true}
              onVerifyChain={handleVerifyChain}
              hasMore={false}
            />
          </div>
        )}
      </div>

      {/* Config Modal would go here in production */}
      {isConfigModalOpen && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg shadow-xl max-w-2xl w-full mx-4 p-6">
            <h2 className="text-lg font-semibold text-gray-900">Edit Configuration</h2>
            <p className="mt-2 text-sm text-gray-500">
              Configuration editing would be implemented here.
            </p>
            <div className="mt-6 flex justify-end">
              <button
                type="button"
                onClick={() => setIsConfigModalOpen(false)}
                className="px-4 py-2 text-sm font-medium text-gray-700 bg-gray-100 rounded-lg hover:bg-gray-200"
              >
                Close
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
