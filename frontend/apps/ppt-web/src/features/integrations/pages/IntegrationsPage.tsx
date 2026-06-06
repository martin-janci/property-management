/**
 * Integrations Page
 *
 * Main page for managing external integrations (Epic 61).
 */

import { useState } from 'react';
import {
  AccountingExportsList,
  AirbnbConnectionPanel,
  BookingChannelPanel,
  CalendarConnectionsList,
  ESignatureWorkflowsList,
  IntegrationsDashboard,
  VideoMeetingsList,
  WebhookSubscriptionsList,
} from '../components';

interface IntegrationsPageProps {
  organizationId: string;
}

type TabValue =
  | 'airbnb'
  | 'calendars'
  | 'accounting'
  | 'esignatures'
  | 'video'
  | 'webhooks'
  | 'booking';

const tabs: { value: TabValue; label: string }[] = [
  { value: 'airbnb', label: 'Airbnb' },
  { value: 'calendars', label: 'Calendars' },
  { value: 'accounting', label: 'Accounting' },
  { value: 'esignatures', label: 'E-Signatures' },
  { value: 'video', label: 'Video' },
  { value: 'webhooks', label: 'Webhooks' },
  { value: 'booking', label: 'Booking.com' },
];

export function IntegrationsPage({ organizationId }: IntegrationsPageProps) {
  const [activeTab, setActiveTab] = useState<TabValue>('airbnb');

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold tracking-tight">Integrations</h1>
        <p className="text-muted-foreground">Connect and manage external services</p>
      </div>

      <IntegrationsDashboard organizationId={organizationId} />

      <div className="space-y-4">
        <div className="flex gap-2 border-b">
          {tabs.map((tab) => (
            <button
              type="button"
              key={tab.value}
              onClick={() => setActiveTab(tab.value)}
              className={`px-4 py-2 text-sm font-medium border-b-2 transition-colors ${
                activeTab === tab.value
                  ? 'border-primary text-primary'
                  : 'border-transparent text-muted-foreground hover:text-foreground'
              }`}
            >
              {tab.label}
            </button>
          ))}
        </div>

        <div className="pt-4">
          {activeTab === 'airbnb' && <AirbnbConnectionPanel organizationId={organizationId} />}

          {activeTab === 'calendars' && <CalendarConnectionsList organizationId={organizationId} />}

          {activeTab === 'accounting' && <AccountingExportsList organizationId={organizationId} />}

          {activeTab === 'esignatures' && (
            <ESignatureWorkflowsList organizationId={organizationId} />
          )}

          {activeTab === 'video' && <VideoMeetingsList organizationId={organizationId} />}

          {activeTab === 'webhooks' && <WebhookSubscriptionsList organizationId={organizationId} />}

          {activeTab === 'booking' && <BookingChannelPanel organizationId={organizationId} />}
        </div>
      </div>
    </div>
  );
}
