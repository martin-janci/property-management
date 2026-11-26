/**
 * Unit tests for the guest report submission helpers (Epic 41, Story 41.1).
 *
 * These pin the pure logic extracted out of GuestReportSubmissionPage so the
 * payload shape, report period, date formatting and portal/template filtering
 * are covered independently of the JSX.
 */

/// <reference types="vitest/globals" />
import type { GovernmentPortalConnection, RegulatoryReportTemplate } from '@ppt/api-client';
import {
  buildSubmissionRequest,
  filterPortalConnections,
  filterReportTemplates,
  formatSubmissionDate,
  type GuestRecord,
  getReportPeriod,
} from './guestReportSubmission';

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

function connection(
  overrides: Partial<GovernmentPortalConnection> = {}
): GovernmentPortalConnection {
  return {
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
    ...overrides,
  };
}

function template(overrides: Partial<RegulatoryReportTemplate> = {}): RegulatoryReportTemplate {
  return {
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
    ...overrides,
  };
}

describe('filterPortalConnections', () => {
  it('keeps only active guest-registration portals for the country', () => {
    const sk = connection();
    const cz = connection({ id: 'conn-2', countryCode: 'CZ' });
    const inactive = connection({ id: 'conn-3', isActive: false });
    const wrongType = connection({ id: 'conn-4', portalType: 'tax_authority' });

    const result = filterPortalConnections([sk, cz, inactive, wrongType], 'SK');

    expect(result).toEqual([sk]);
  });

  it('accepts housing_registry as an eligible portal type', () => {
    const housing = connection({ id: 'conn-5', portalType: 'housing_registry' });
    expect(filterPortalConnections([housing], 'SK')).toEqual([housing]);
  });
});

describe('filterReportTemplates', () => {
  it('keeps only active guest-registration templates for the country', () => {
    const sk = template();
    const cz = template({ id: 'tpl-2', countryCode: 'CZ' });
    const inactive = template({ id: 'tpl-3', isActive: false });
    const wrongType = template({ id: 'tpl-4', portalType: 'tax_authority' });

    expect(filterReportTemplates([sk, cz, inactive, wrongType], 'SK')).toEqual([sk]);
  });
});

describe('formatSubmissionDate', () => {
  it('renders a day/short-month/year string', () => {
    expect(formatSubmissionDate('2026-08-01')).toBe('01 Aug 2026');
  });
});

describe('getReportPeriod', () => {
  it('returns the full calendar month containing the reference date', () => {
    const period = getReportPeriod(new Date(2026, 7, 15)); // August 2026
    expect(period.start).toBe('2026-08-01');
    expect(period.end).toBe('2026-08-31');
  });
});

describe('buildSubmissionRequest', () => {
  it('maps guests and metadata into the submission payload', () => {
    const reference = new Date(2026, 7, 15);
    const request = buildSubmissionRequest({
      portalConnectionId: 'conn-1',
      templateId: 'tpl-1',
      country: 'SK',
      guests: [guest],
      reference,
    });

    expect(request.portalConnectionId).toBe('conn-1');
    expect(request.templateId).toBe('tpl-1');
    expect(request.reportType).toBe('guest_registration');
    expect(request.reportPeriodStart).toBe('2026-08-01');
    expect(request.reportPeriodEnd).toBe('2026-08-31');

    expect(request.reportData.countryCode).toBe('SK');
    expect(request.reportData.totalGuests).toBe(1);
    expect(request.reportData.submissionDate).toBe(reference.toISOString());

    const guests = request.reportData.guests as Array<Record<string, unknown>>;
    expect(guests).toHaveLength(1);
    expect(guests[0]).toMatchObject({
      firstName: 'Jane',
      lastName: 'Doe',
      documentNumber: 'X1234567',
      accommodation: { unitNumber: 'A-101', buildingName: 'Central Court' },
    });
  });
});
