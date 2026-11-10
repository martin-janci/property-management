/**
 * GuestReportSubmissionPage tests (Epic 41: Government Portal UI, Story 41.1).
 *
 * The submission preview is rendered INLINE by this page (step "preview"),
 * which is why the standalone SubmissionPreviewModal component was removed as
 * dead code. This test pins that behavior: advancing through the wizard shows
 * the inline preview without any separate modal component.
 */

/// <reference types="vitest/globals" />
import type { GovernmentPortalConnection, RegulatoryReportTemplate } from '@ppt/api-client';
import { fireEvent, render, screen } from '@testing-library/react';
import { type GuestRecord, GuestReportSubmissionPage } from './GuestReportSubmissionPage';

const noop = () => {};
const asyncNoop = async () => {};

const connection: GovernmentPortalConnection = {
  id: 'conn-1',
  organizationId: 'org-1',
  portalType: 'police_registry',
  portalName: 'SK Police Registry',
  portalCode: 'SK-POL',
  countryCode: 'SK',
  isActive: true,
  autoSubmit: false,
  testMode: true,
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-01T00:00:00Z',
};

const template: RegulatoryReportTemplate = {
  id: 'tpl-1',
  templateCode: 'SK-GUEST',
  templateName: 'SK Guest Registration',
  portalType: 'police_registry',
  countryCode: 'SK',
  schemaVersion: '1.0',
  fieldMappings: {},
  validationRules: {},
  isActive: true,
  effectiveFrom: '2026-01-01',
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-01T00:00:00Z',
};

const guest: GuestRecord = {
  id: 'guest-1',
  firstName: 'Jane',
  lastName: 'Doe',
  dateOfBirth: '1990-05-01',
  nationality: 'DE',
  documentType: 'passport',
  documentNumber: 'X1234567',
  checkInDate: '2026-08-01',
  checkOutDate: '2026-08-05',
  unitNumber: 'A-101',
  buildingName: 'Central Court',
};

const baseProps = {
  connections: [connection],
  templates: [template],
  guests: [guest],
  onSubmit: asyncNoop,
  onBack: noop,
};

describe('GuestReportSubmissionPage', () => {
  it('renders the submission preview inline after advancing the wizard', () => {
    render(<GuestReportSubmissionPage {...baseProps} />);

    // Step 1: select the portal connection and report template.
    fireEvent.click(screen.getByRole('radio'));
    fireEvent.change(screen.getByRole('combobox'), { target: { value: 'tpl-1' } });

    // Advance to the preview step.
    fireEvent.click(screen.getByRole('button', { name: /preview submission/i }));

    // The preview is rendered inline on the page (no separate modal).
    expect(screen.getByText('Submission Preview')).toBeInTheDocument();
    expect(screen.getByText('SK Guest Registration')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /submit to portal/i })).toBeInTheDocument();
  });
});
