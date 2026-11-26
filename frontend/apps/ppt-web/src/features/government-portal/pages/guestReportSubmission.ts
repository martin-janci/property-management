/**
 * Pure helpers for the guest report submission wizard.
 * Epic 41: Government Portal UI (Story 41.1)
 *
 * Extracted from GuestReportSubmissionPage so the submission payload,
 * date formatting, report-period and portal/template filtering logic are
 * testable in isolation (and not re-derived inline inside the JSX).
 */

import type {
  CreateRegulatorySubmissionRequest,
  GovernmentPortalConnection,
  RegulatoryReportTemplate,
} from '@ppt/api-client';

/** Guest record for submission. */
export interface GuestRecord {
  id: string;
  firstName: string;
  lastName: string;
  dateOfBirth: string;
  nationality: string;
  documentType: string;
  documentNumber: string;
  checkInDate: string;
  checkOutDate?: string;
  unitNumber: string;
  buildingName: string;
}

/** Country codes the guest-report wizard supports. */
export type SubmissionCountry = 'SK' | 'CZ';

/** Portal types eligible for guest-registration submissions. */
const GUEST_PORTAL_TYPES: readonly string[] = ['police_registry', 'housing_registry'];

/** Active guest-registration portal connections for a country. */
export function filterPortalConnections(
  connections: GovernmentPortalConnection[],
  country: SubmissionCountry
): GovernmentPortalConnection[] {
  return connections.filter(
    (c) => c.countryCode === country && GUEST_PORTAL_TYPES.includes(c.portalType) && c.isActive
  );
}

/** Active guest-registration report templates for a country. */
export function filterReportTemplates(
  templates: RegulatoryReportTemplate[],
  country: SubmissionCountry
): RegulatoryReportTemplate[] {
  return templates.filter(
    (template) =>
      template.countryCode === country &&
      GUEST_PORTAL_TYPES.includes(template.portalType) &&
      template.isActive
  );
}

/** Display format for dates (e.g. "01 Aug 2026"). */
export function formatSubmissionDate(dateString: string): string {
  return new Date(dateString).toLocaleDateString('en-GB', {
    day: '2-digit',
    month: 'short',
    year: 'numeric',
  });
}

/**
 * The report period is the full calendar month containing `reference`,
 * returned as `YYYY-MM-DD` boundary strings.
 */
export function getReportPeriod(reference: Date): { start: string; end: string } {
  const start = new Date(reference.getFullYear(), reference.getMonth(), 1);
  const end = new Date(reference.getFullYear(), reference.getMonth() + 1, 0);
  return {
    start: start.toISOString().split('T')[0],
    end: end.toISOString().split('T')[0],
  };
}

/** Build the portal submission request payload from the selected guests. */
export function buildSubmissionRequest(params: {
  portalConnectionId: string;
  templateId: string;
  country: SubmissionCountry;
  guests: GuestRecord[];
  /** Reference date for submission timestamp + report period (defaults to now). */
  reference?: Date;
}): CreateRegulatorySubmissionRequest {
  const { portalConnectionId, templateId, country, guests } = params;
  const reference = params.reference ?? new Date();
  const period = getReportPeriod(reference);

  const reportData: Record<string, unknown> = {
    guests: guests.map((guest) => ({
      firstName: guest.firstName,
      lastName: guest.lastName,
      dateOfBirth: guest.dateOfBirth,
      nationality: guest.nationality,
      documentType: guest.documentType,
      documentNumber: guest.documentNumber,
      checkInDate: guest.checkInDate,
      checkOutDate: guest.checkOutDate,
      accommodation: {
        unitNumber: guest.unitNumber,
        buildingName: guest.buildingName,
      },
    })),
    submissionDate: reference.toISOString(),
    countryCode: country,
    totalGuests: guests.length,
  };

  return {
    portalConnectionId,
    templateId,
    reportType: 'guest_registration',
    reportPeriodStart: period.start,
    reportPeriodEnd: period.end,
    reportData,
  };
}
