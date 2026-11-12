/**
 * Leases route group (Epic 19 — Lease Management & Tenant Screening, plus
 * Epic 142 violation tracking).
 *
 * Mounts the already-built `features/leases/` pages into ppt-web, wired to the
 * `@ppt/api-client` leases module (PAP-59, mirroring the meters group from
 * PAP-30). This group owns the API↔UI mappers (backend wire shapes are
 * snake_case with Decimal-as-string) and the thin route-wrapper components.
 *
 * The dashboard / list / templates / applications / violations routes render
 * live backend data. The lease/application/violation detail routes fetch their
 * single entity and map it best-effort onto the richer UI prop shapes.
 *
 * Wired actions (all with toast feedback + query invalidation):
 *  - create lease: unit picker = units of every org building
 *    (`GET /buildings/{id}/units`), tenant picker = org members
 *    (`GET /organizations/{id}/members`) excluding manager roles;
 *  - record payment (`POST /leases/{payment_id}/payments`, see api-client note);
 *  - template create / edit / (de)activate (`POST|PUT /leases/templates…`) —
 *    the API has no template delete, so that control is hidden;
 *  - application screening (`POST …/applications/{id}/screening`) and review
 *    (`POST …/applications/{id}/review`) — screening rows have no list
 *    endpoint, so only the rows returned by this session's initiate call show;
 *  - violation resolve (`PUT /violations/{id}`), dispute (`POST
 *    /violations/{id}/appeals`) and evidence upload (presigned
 *    `documents/upload-url` → `POST /violations/{id}/evidence`).
 */
import type {
  ApplicationSummary as ApiApplicationSummary,
  ExpirationOverview as ApiExpirationOverview,
  Lease as ApiLease,
  LeaseAmendment as ApiLeaseAmendment,
  LeasePayment as ApiLeasePayment,
  LeaseReminder as ApiLeaseReminder,
  LeaseStatistics as ApiLeaseStatistics,
  LeaseSummary as ApiLeaseSummary,
  LeaseTemplate as ApiLeaseTemplate,
  TenantApplication as ApiTenantApplication,
  TenantScreening as ApiTenantScreening,
  Violation as ApiViolation,
  ViolationCategory as ApiViolationCategory,
  ViolationEvidence as ApiViolationEvidence,
  ViolationSeverity as ApiViolationSeverity,
  ViolationStatus as ApiViolationStatus,
  ViolationSummary as ApiViolationSummary,
  CreateViolationRequest,
  OrganizationMember,
  SharedSupportedCurrency,
} from '@ppt/api-client';
import {
  createUploadUrl,
  listBuildingUnits,
  uploadFileToPresignedUrl,
  useAddViolationEvidence,
  useApplication,
  useApplications,
  useBuildings,
  useCreateLease,
  useCreateLeaseAmendment,
  useCreateLeaseTemplate,
  useCreateViolation,
  useCreateViolationAppeal,
  useExpiringLeases,
  useInitiateScreening,
  useLease,
  useLeaseStatistics,
  useLeases,
  useLeaseTemplates,
  useOrganizationMembers,
  useRecordLeasePayment,
  useRenewLease,
  useReviewApplication,
  useTerminateLease,
  useUpdateLease,
  useUpdateLeaseTemplate,
  useUpdateViolation,
  useViolation,
  useViolationEvidence,
  useViolations,
} from '@ppt/api-client';
import { useQueries } from '@tanstack/react-query';
import { lazy, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Route, useNavigate, useParams, useSearchParams } from 'react-router-dom';
import { ProtectedRoute, useToast } from '../../components';
import { ConfirmationDialog } from '../../components/ConfirmationDialog';
import { useAuth } from '../../contexts';
import {
  AddAmendmentModal,
  type AmendmentFormData,
  type EditLeaseFormData,
  EditLeaseModal,
  type RenewLeaseFormData,
  RenewLeaseModal,
  type TerminateLeaseFormData,
  TerminateLeaseModal,
} from '../../features/leases/components/LeaseActionModals';
import {
  type RecordPaymentFormData,
  RecordPaymentModal,
} from '../../features/leases/components/RecordPaymentModal';
import type { ApplicationListParams } from '../../features/leases/pages/ApplicationsPage';
import type { LeaseListParams } from '../../features/leases/pages/LeasesListPage';
import type {
  CreateLeaseData,
  CreateTemplateData,
  CreateViolationData,
  ReviewApplicationData,
  ApplicationStatus as UiApplicationStatus,
  ApplicationSummary as UiApplicationSummary,
  ExpirationOverview as UiExpirationOverview,
  Lease as UiLease,
  LeaseAmendment as UiLeaseAmendment,
  LeaseApplication as UiLeaseApplication,
  LeasePayment as UiLeasePayment,
  LeaseReminder as UiLeaseReminder,
  LeaseStatistics as UiLeaseStatistics,
  LeaseStatus as UiLeaseStatus,
  LeaseSummary as UiLeaseSummary,
  LeaseTemplate as UiLeaseTemplate,
  LeaseWithDetails as UiLeaseWithDetails,
  PaymentStatus as UiPaymentStatus,
  ScreeningStatus as UiScreeningStatus,
  ScreeningType as UiScreeningType,
  TenantScreening as UiTenantScreening,
  ViolationEvidence as UiViolationEvidence,
  ViolationSeverity as UiViolationSeverity,
  ViolationStatus as UiViolationStatus,
  ViolationSummary as UiViolationSummary,
  ViolationType as UiViolationType,
  ViolationWithDetails as UiViolationWithDetails,
} from '../../features/leases/types';
import { useOrganization } from '../../hooks/useOrganization';
import { queryKeys } from '../../lib/queryKeys';
import { isManagerRole, MANAGER_ROLES } from '../shared';

// Leases feature pages (Epic 19) — lazy-loaded for code splitting.
const LeasesDashboardPage = lazy(() =>
  import('../../features/leases').then((m) => ({ default: m.LeasesDashboardPage }))
);
const LeasesListPage = lazy(() =>
  import('../../features/leases').then((m) => ({ default: m.LeasesListPage }))
);
const CreateLeasePage = lazy(() =>
  import('../../features/leases').then((m) => ({ default: m.CreateLeasePage }))
);
const TemplatesPage = lazy(() =>
  import('../../features/leases').then((m) => ({ default: m.TemplatesPage }))
);
const ApplicationsPage = lazy(() =>
  import('../../features/leases').then((m) => ({ default: m.ApplicationsPage }))
);
const ApplicationDetailPage = lazy(() =>
  import('../../features/leases').then((m) => ({ default: m.ApplicationDetailPage }))
);
const LeaseDetailPage = lazy(() =>
  import('../../features/leases').then((m) => ({ default: m.LeaseDetailPage }))
);
const ViolationsPage = lazy(() =>
  import('../../features/leases').then((m) => ({ default: m.ViolationsPage }))
);
const CreateViolationPage = lazy(() =>
  import('../../features/leases').then((m) => ({ default: m.CreateViolationPage }))
);
const ViolationDetailPage = lazy(() =>
  import('../../features/leases').then((m) => ({ default: m.ViolationDetailPage }))
);

/**
 * Supported ISO 4217 codes the UI can label amounts with (mirrors
 * `SharedSupportedCurrency` from the generated API client).
 */
const SUPPORTED_CURRENCIES: ReadonlySet<SharedSupportedCurrency> = new Set([
  'EUR',
  'CZK',
  'CHF',
  'GBP',
  'PLN',
  'USD',
  'HUF',
  'RON',
  'BGN',
  'HRK',
  'SEK',
  'DKK',
  'NOK',
]);

const FALLBACK_CURRENCY: SharedSupportedCurrency = 'EUR';

/**
 * Resolve the currency used to label lease monetary values.
 *
 * The backend lease wire shapes (`Lease`, `LeaseSummary`, `LeasePayment`,
 * `LeaseStatistics`, ...) carry no currency field, so the UI cannot derive a
 * per-lease currency from the payload. Until the API models currency per
 * entity, a single deployment-level default is used: `VITE_DEFAULT_CURRENCY`
 * (an ISO 4217 code) when set to a supported value, otherwise EUR. This lets
 * PLN/HUF/CZK market deployments label amounts correctly instead of baking in
 * EUR for every market.
 */
export function resolveDefaultCurrency(): SharedSupportedCurrency {
  const configured = import.meta.env.VITE_DEFAULT_CURRENCY;
  if (typeof configured === 'string') {
    const code = configured.trim().toUpperCase();
    if ((SUPPORTED_CURRENCIES as ReadonlySet<string>).has(code)) {
      return code as SharedSupportedCurrency;
    }
  }
  return FALLBACK_CURRENCY;
}

// ============================================================================
// API → UI mappers (snake_case wire shapes, Decimal-as-string → camelCase)
// ============================================================================

/** Map the backend lease status string onto the UI `LeaseStatus` union. */
function mapLeaseStatus(status: string): UiLeaseStatus {
  switch (status) {
    case 'pending_signature':
      return 'pending_signatures';
    case 'active':
    case 'renewed':
    case 'terminated':
    case 'expired':
    case 'draft':
      return status;
    case 'cancelled':
      return 'terminated';
    default:
      return 'draft';
  }
}

/** Map the backend application status string onto the UI `ApplicationStatus`. */
function mapApplicationStatus(status: string): UiApplicationStatus {
  switch (status) {
    case 'submitted':
    case 'under_review':
    case 'approved':
    case 'rejected':
    case 'withdrawn':
    case 'draft':
      return status;
    case 'screening_pending':
    case 'screening_complete':
      return 'screening';
    default:
      return 'submitted';
  }
}

/** Map the backend violation category onto the UI `ViolationType`. */
function mapViolationType(category: ApiViolationCategory): UiViolationType {
  switch (category) {
    case 'noise':
      return 'noise';
    case 'parking':
      return 'parking';
    case 'pet':
      return 'pet_policy';
    case 'payment':
      return 'late_payment';
    case 'lease':
      return 'lease_terms';
    case 'maintenance':
    case 'architectural':
    case 'common_area':
      return 'cleanliness';
    case 'safety':
      return 'illegal_activity';
    default:
      return 'other';
  }
}

/** Reverse of {@link mapViolationType}: UI type → backend category. */
function mapViolationCategory(type: UiViolationType): ApiViolationCategory {
  switch (type) {
    case 'noise':
      return 'noise';
    case 'parking':
      return 'parking';
    case 'pet_policy':
      return 'pet';
    case 'late_payment':
      return 'payment';
    case 'lease_terms':
      return 'lease';
    case 'damage':
    case 'cleanliness':
      return 'maintenance';
    case 'illegal_activity':
      return 'safety';
    default:
      return 'other';
  }
}

/** Map the backend violation severity (`major|critical` → `severe`). */
function mapViolationSeverity(severity: ApiViolationSeverity): UiViolationSeverity {
  switch (severity) {
    case 'minor':
      return 'minor';
    case 'moderate':
      return 'moderate';
    default:
      return 'severe';
  }
}

/** Reverse of {@link mapViolationSeverity}: UI severity → backend severity. */
function mapSeverityToApi(severity: UiViolationSeverity): ApiViolationSeverity {
  switch (severity) {
    case 'minor':
      return 'minor';
    case 'moderate':
      return 'moderate';
    default:
      return 'major';
  }
}

/** Map the backend violation status onto the UI `ViolationStatus`. */
function mapViolationStatus(status: ApiViolationStatus): UiViolationStatus {
  switch (status) {
    case 'disputed':
      return 'disputed';
    case 'resolved':
      return 'resolved';
    case 'dismissed':
      return 'dismissed';
    case 'escalated':
      return 'escalated';
    default:
      return 'open';
  }
}

/** Transform an API lease summary → UI `LeaseSummary`. */
export function mapLeaseSummaryToUi(s: ApiLeaseSummary): UiLeaseSummary {
  return {
    id: s.id,
    unitId: s.unit_id,
    unitNumber: s.unit_name,
    buildingName: s.building_name,
    tenantId: '',
    tenantName: s.tenant_name,
    tenantEmail: s.tenant_email,
    status: mapLeaseStatus(s.status),
    startDate: s.start_date,
    endDate: s.end_date,
    rentAmount: Number(s.monthly_rent),
    currency: resolveDefaultCurrency(),
    daysUntilExpiry: s.days_until_expiry,
  };
}

/** Transform an API application summary → UI `ApplicationSummary`. */
function mapApplicationSummaryToUi(a: ApiApplicationSummary): UiApplicationSummary {
  return {
    id: a.id,
    unitId: a.unit_id,
    unitNumber: a.unit_name,
    buildingName: a.building_name,
    applicantName: a.applicant_name,
    applicantEmail: a.applicant_email,
    status: mapApplicationStatus(a.status),
    submittedAt: a.submitted_at ?? undefined,
    createdAt: a.submitted_at ?? '',
  };
}

/** Transform an API lease template → UI `LeaseTemplate`. */
function mapTemplateToUi(t: ApiLeaseTemplate): UiLeaseTemplate {
  return {
    id: t.id,
    organizationId: t.organization_id,
    name: t.name,
    description: t.description ?? undefined,
    content: t.content_html,
    defaultDepositMonths:
      t.default_security_deposit_months != null
        ? Number(t.default_security_deposit_months)
        : undefined,
    defaultLeaseDurationMonths: t.default_term_months ?? undefined,
    isActive: t.is_active,
    createdAt: t.created_at,
    updatedAt: t.updated_at,
    createdBy: t.created_by ?? '',
  };
}

/** Transform an API statistics payload → UI `LeaseStatistics`. */
export function mapStatisticsToUi(s: ApiLeaseStatistics): UiLeaseStatistics {
  return {
    totalLeases: s.total_leases,
    activeLeases: s.active_leases,
    expiringLeases: s.expiring_soon,
    expiredLeases: 0,
    pendingApplications: s.pending_applications,
    occupancyRate: s.occupancy_rate,
    totalMonthlyRent: Number(s.total_monthly_rent),
    overduePayments: 0,
    currency: resolveDefaultCurrency(),
  };
}

/** Transform an API expiration overview → UI `ExpirationOverview`. */
function mapExpirationToUi(o: ApiExpirationOverview): UiExpirationOverview {
  return {
    expiringIn30Days: o.expiring_30_days.map(mapLeaseSummaryToUi),
    expiringIn60Days: o.expiring_60_days.map(mapLeaseSummaryToUi),
    expiringIn90Days: o.expiring_90_days.map(mapLeaseSummaryToUi),
    expired: [],
  };
}

/** Map a backend lease payment onto a UI payment status. */
function mapPaymentStatus(p: ApiLeasePayment): UiPaymentStatus {
  if (p.paid_at) {
    return p.paid_amount != null && Number(p.paid_amount) < Number(p.amount) ? 'partial' : 'paid';
  }
  return p.is_late ? 'overdue' : 'pending';
}

/** Transform an API lease amendment → UI `LeaseAmendment`. */
function mapAmendmentToUi(a: ApiLeaseAmendment): UiLeaseAmendment {
  return {
    id: a.id,
    leaseId: a.lease_id,
    effectiveDate: a.effective_date,
    description: a.description ?? a.title,
    createdAt: a.created_at,
    createdBy: a.created_by ?? '',
  };
}

/** Transform an API lease payment → UI `LeasePayment`. */
export function mapPaymentToUi(p: ApiLeasePayment): UiLeasePayment {
  return {
    id: p.id,
    leaseId: p.lease_id,
    dueDate: p.due_date,
    amount: Number(p.amount),
    currency: resolveDefaultCurrency(),
    status: mapPaymentStatus(p),
    paidAmount: p.paid_amount != null ? Number(p.paid_amount) : undefined,
    paidAt: p.paid_at ?? undefined,
    reference: p.payment_reference ?? undefined,
    notes: p.description ?? undefined,
  };
}

/** Transform an API lease reminder → UI `LeaseReminder`. */
function mapReminderToUi(r: ApiLeaseReminder): UiLeaseReminder {
  return {
    id: r.id,
    leaseId: r.lease_id,
    reminderType: r.reminder_type,
    scheduledFor: r.trigger_date,
    message: r.message,
    isSent: r.sent_at != null,
    sentAt: r.sent_at ?? undefined,
  };
}

/** Transform the core API lease entity → UI `Lease`. */
export function mapLeaseToUi(l: ApiLease): UiLease {
  return {
    id: l.id,
    organizationId: l.organization_id,
    unitId: l.unit_id,
    templateId: l.template_id ?? undefined,
    tenantId: l.tenant_user_id ?? '',
    status: mapLeaseStatus(l.status),
    startDate: l.start_date,
    endDate: l.end_date,
    rentAmount: Number(l.monthly_rent),
    currency: resolveDefaultCurrency(),
    depositAmount: Number(l.security_deposit),
    paymentDayOfMonth: l.rent_due_day,
    notes: l.notes ?? undefined,
    signedAt: l.tenant_signed_at ?? l.landlord_signed_at ?? undefined,
    terminatedAt: l.terminated_at ?? undefined,
    terminationReason: l.termination_reason ?? undefined,
    createdAt: l.created_at,
    updatedAt: l.updated_at,
    createdBy: l.created_by ?? '',
  };
}

/** Transform `LeaseWithDetails` (`GET /leases/{id}`) → UI `LeaseWithDetails`. */
function mapLeaseWithDetailsToUi(d: {
  lease: ApiLease;
  unit_name: string;
  building_name: string;
  amendments: ApiLeaseAmendment[];
  upcoming_payments: ApiLeasePayment[];
  reminders: ApiLeaseReminder[];
}): UiLeaseWithDetails {
  return {
    ...mapLeaseToUi(d.lease),
    unit: {
      id: d.lease.unit_id,
      number: d.unit_name,
      buildingId: '',
      buildingName: d.building_name,
    },
    tenant: {
      id: d.lease.tenant_user_id ?? '',
      name: d.lease.tenant_name,
      email: d.lease.tenant_email,
      phone: d.lease.tenant_phone ?? undefined,
    },
    amendments: d.amendments.map(mapAmendmentToUi),
    upcomingPayments: d.upcoming_payments.map(mapPaymentToUi),
    reminders: d.reminders.map(mapReminderToUi),
  };
}

/** Transform an API tenant application → UI `LeaseApplication`. */
function mapApplicationToUi(a: ApiTenantApplication): UiLeaseApplication {
  return {
    id: a.id,
    organizationId: a.organization_id,
    unitId: a.unit_id,
    status: mapApplicationStatus(a.status),
    applicantName: a.applicant_name,
    applicantEmail: a.applicant_email,
    applicantPhone: a.applicant_phone ?? undefined,
    currentAddress: a.current_address ?? undefined,
    employerName: a.employer_name ?? undefined,
    employerPhone: a.employer_phone ?? undefined,
    // Backend stores monthly income; the UI surfaces an annual figure.
    annualIncome: a.monthly_income != null ? Number(a.monthly_income) * 12 : undefined,
    moveInDate: a.desired_move_in ?? undefined,
    notes: a.decision_notes ?? undefined,
    submittedAt: a.submitted_at ?? undefined,
    reviewedAt: a.reviewed_at ?? undefined,
    reviewedBy: a.reviewed_by ?? undefined,
    reviewNotes: a.decision_notes ?? undefined,
    createdAt: a.created_at,
    updatedAt: a.updated_at,
  };
}

/** Transform an API violation summary → UI `ViolationSummary`. */
function mapViolationSummaryToUi(v: ApiViolationSummary): UiViolationSummary {
  return {
    id: v.id,
    leaseId: '',
    unitId: '',
    unitNumber: v.violator_unit ?? '',
    buildingName: '',
    tenantId: '',
    tenantName: v.violator_name ?? '',
    violationType: mapViolationType(v.category),
    severity: mapViolationSeverity(v.severity),
    status: mapViolationStatus(v.status),
    violationDate: v.occurred_at,
    reportedAt: v.occurred_at,
  };
}

/** Transform a single API violation → UI `ViolationWithDetails` (best-effort). */
function mapViolationToUi(v: ApiViolation): UiViolationWithDetails {
  return {
    id: v.id,
    organizationId: v.organization_id,
    leaseId: '',
    unitId: v.unit_id ?? '',
    violationType: mapViolationType(v.category),
    severity: mapViolationSeverity(v.severity),
    status: mapViolationStatus(v.status),
    description: v.description,
    violationDate: v.occurred_at,
    reportedAt: v.reported_at,
    reportedBy: v.reporter_id ?? '',
    resolvedAt: v.resolved_at ?? undefined,
    resolutionNotes: v.resolution_notes ?? undefined,
    createdAt: v.created_at,
    updatedAt: v.updated_at,
    lease: { id: '', startDate: '', endDate: '' },
    unit: {
      id: v.unit_id ?? '',
      number: v.violator_unit ?? '',
      buildingId: v.building_id ?? '',
      buildingName: '',
    },
    tenant: { id: v.violator_id ?? '', name: v.violator_name ?? '', email: '' },
    reportedByUser: { id: v.reporter_id ?? '', name: '' },
    evidence: [],
    timeline: [],
  };
}

/** Map a backend screening status onto the UI `ScreeningStatus`. */
function mapScreeningStatus(status: string): UiScreeningStatus {
  switch (status) {
    case 'in_progress':
      return 'in_progress';
    case 'completed':
      return 'completed';
    case 'failed':
    case 'expired':
      return 'failed';
    default:
      // pending_consent / consent_received
      return 'pending';
  }
}

/** Transform an API tenant screening → UI `TenantScreening`. */
export function mapScreeningToUi(s: ApiTenantScreening): UiTenantScreening {
  return {
    id: s.id,
    applicationId: s.application_id,
    screeningType: s.screening_type as UiScreeningType,
    status: mapScreeningStatus(s.status),
    provider: s.provider ?? undefined,
    requestedAt: s.consent_requested_at ?? s.created_at,
    completedAt: s.completed_at ?? undefined,
    result: s.passed == null ? undefined : s.passed ? 'pass' : 'fail',
    score: s.risk_score ?? undefined,
    details: s.result_summary ?? undefined,
    consentGivenAt: s.consent_received_at ?? undefined,
  };
}

/** Transform an API violation evidence row → UI `ViolationEvidence`. */
export function mapEvidenceToUi(e: ApiViolationEvidence): UiViolationEvidence {
  return {
    id: e.id,
    violationId: e.violation_id,
    fileName: e.file_name,
    // The API stores a storage key, not a downloadable URL, and exposes no
    // download-by-key route — so no link is rendered for evidence files.
    fileUrl: '',
    fileType: e.file_type,
    fileSize: e.file_size ?? 0,
    description: e.description ?? undefined,
    uploadedAt: e.created_at,
    uploadedBy: e.uploaded_by ?? '',
  };
}

/** UI template form → API template fields (rent/currency are not persisted). */
function mapTemplateDataToApi(data: Partial<CreateTemplateData>) {
  return {
    name: data.name,
    description: data.description,
    content_html: data.content,
    default_term_months: data.defaultLeaseDurationMonths,
    default_security_deposit_months: data.defaultDepositMonths,
  };
}

/**
 * Org members eligible as lease tenants: everyone except manager-level roles
 * (managers / admins sign as the landlord side, never as tenants).
 */
export function selectTenantCandidates(members: OrganizationMember[]) {
  const managerRoles: readonly string[] = MANAGER_ROLES;
  return members
    .filter((m) => !managerRoles.includes(m.role_type))
    .map((m) => ({ id: m.user_id, name: m.name, email: m.email }));
}

/**
 * Units of every building in the organization, for the lease unit picker.
 * There is no org-wide units endpoint, so this fans out one
 * `GET /buildings/{id}/units` per building (max page size).
 */
function useOrganizationUnits() {
  const { data: buildingsData, isLoading: buildingsLoading } = useBuildings();
  const buildings = buildingsData?.items ?? [];
  const results = useQueries({
    queries: buildings.map((b) => ({
      queryKey: [...queryKeys.buildings.units(b.id), 'lease-picker'] as const,
      queryFn: ({ signal }: { signal: AbortSignal }) =>
        listBuildingUnits(b.id, { limit: 100 }, signal),
    })),
  });
  const units = results.flatMap((r, i) =>
    (r.data?.units ?? []).map((u) => ({
      id: u.id,
      number: u.designation,
      buildingName: buildings[i]?.name ?? '',
    }))
  );
  const isLoading = buildingsLoading || results.some((r) => r.isLoading);
  return { units, isLoading };
}

/** Error message for a failed mutation, falling back to a translated default. */
function errorMessage(err: unknown, fallback: string): string {
  return err instanceof Error && err.message ? err.message : fallback;
}

// ============================================================================
// Route wrappers
// ============================================================================

/** Route wrapper: leases dashboard, live statistics + expiration overview. */
function LeasesDashboardPageRoute() {
  const navigate = useNavigate();
  const { organizationId } = useOrganization();
  const statsQuery = useLeaseStatistics(organizationId);
  const expiringQuery = useExpiringLeases({ organization_id: organizationId });

  const statistics: UiLeaseStatistics = statsQuery.data
    ? mapStatisticsToUi(statsQuery.data)
    : {
        totalLeases: 0,
        activeLeases: 0,
        expiringLeases: 0,
        expiredLeases: 0,
        pendingApplications: 0,
        occupancyRate: 0,
        totalMonthlyRent: 0,
        overduePayments: 0,
        currency: resolveDefaultCurrency(),
      };
  const expirationOverview: UiExpirationOverview = expiringQuery.data
    ? mapExpirationToUi(expiringQuery.data)
    : { expiringIn30Days: [], expiringIn60Days: [], expiringIn90Days: [], expired: [] };

  return (
    <LeasesDashboardPage
      statistics={statistics}
      expirationOverview={expirationOverview}
      isLoading={statsQuery.isLoading || expiringQuery.isLoading}
      onNavigateToLease={(id) => navigate(`/leases/${id}`)}
      onNavigateToLeases={() => navigate('/leases/list')}
      onNavigateToApplications={() => navigate('/leases/applications')}
      onNavigateToCreateLease={() => navigate('/leases/new')}
      onRenewLease={(id) => navigate(`/leases/${id}?action=renew`)}
      onNavigateToTemplates={() => navigate('/leases/templates')}
      onNavigateToViolations={() => navigate('/leases/violations')}
    />
  );
}

/** Page size of the paginated lease / application lists (server `limit`). */
export const LIST_PAGE_SIZE = 12;

/** Lease statuses the list filter offers (UI values). */
const UI_LEASE_STATUSES: readonly UiLeaseStatus[] = [
  'active',
  'pending_signatures',
  'expiring_soon',
  'expired',
  'terminated',
  'renewed',
  'draft',
];

/** Days window used for the synthetic `expiring_soon` status filter. */
const EXPIRING_SOON_DAYS = 90;

/** Positive integer from a URL search param, else `undefined`. */
function positiveInt(value: string | null): number | undefined {
  if (!value) return undefined;
  const n = Number(value);
  return Number.isInteger(n) && n > 0 ? n : undefined;
}

/**
 * Lease list filters persisted in the URL (`?status=&expiring=&page=`). Only
 * filters the backend `GET /leases` query supports are kept (`status`,
 * `expiring_within_days`, `limit`/`offset`); it has no building or text
 * search, so those controls are hidden.
 */
export function readLeaseListParams(params: URLSearchParams): {
  filters: Omit<LeaseListParams, 'page' | 'pageSize'>;
  page: number;
} {
  const rawStatus = params.get('status');
  const status = UI_LEASE_STATUSES.find((s) => s === rawStatus);
  return {
    filters: { status, expiringWithinDays: positiveInt(params.get('expiring')) },
    page: positiveInt(params.get('page')) ?? 1,
  };
}

/** Serialize lease list filters back into URL search params (empty values dropped). */
export function writeLeaseListParams(p: LeaseListParams): URLSearchParams {
  const out = new URLSearchParams();
  if (p.status) out.set('status', p.status);
  if (p.expiringWithinDays) out.set('expiring', String(p.expiringWithinDays));
  if (p.page > 1) out.set('page', String(p.page));
  return out;
}

/** UI lease list filters → backend `ListLeasesQuery` fields. */
export function toLeaseListQuery(
  filters: Omit<LeaseListParams, 'page' | 'pageSize'>,
  page: number
): { status?: string; expiring_within_days?: number; limit: number; offset: number } {
  let status: string | undefined;
  let expiring = filters.expiringWithinDays;
  switch (filters.status) {
    case undefined:
      break;
    case 'pending_signatures':
      status = 'pending_signature';
      break;
    case 'expiring_soon':
      // Not a stored status: active leases ending within the window.
      status = 'active';
      expiring = expiring ?? EXPIRING_SOON_DAYS;
      break;
    default:
      status = filters.status;
  }
  return {
    status,
    expiring_within_days: expiring,
    limit: LIST_PAGE_SIZE,
    offset: (page - 1) * LIST_PAGE_SIZE,
  };
}

/** Route wrapper: leases list, live across the organization, URL-persisted filters. */
function LeasesListPageRoute() {
  const navigate = useNavigate();
  const { organizationId } = useOrganization();
  const [searchParams, setSearchParams] = useSearchParams();
  const { filters, page } = readLeaseListParams(searchParams);
  const { data, isLoading, isError, refetch } = useLeases({
    organization_id: organizationId,
    ...toLeaseListQuery(filters, page),
  });

  const leases = (data?.items ?? []).map(mapLeaseSummaryToUi);

  return (
    <LeasesListPage
      leases={leases}
      total={data?.total ?? 0}
      buildings={[]}
      isLoading={isLoading}
      isError={isError}
      onRetry={() => {
        void refetch();
      }}
      initialFilters={filters}
      initialPage={page}
      pageSize={LIST_PAGE_SIZE}
      showSearch={false}
      showBuildingFilter={false}
      onNavigateToCreate={() => navigate('/leases/new')}
      onNavigateToView={(id) => navigate(`/leases/${id}`)}
      onRenewLease={(id) => navigate(`/leases/${id}?action=renew`)}
      onTerminateLease={(id) => navigate(`/leases/${id}?action=terminate`)}
      onFilterChange={(next) => setSearchParams(writeLeaseListParams(next), { replace: true })}
    />
  );
}

type LeaseDialog = 'edit' | 'renew' | 'terminate' | 'terminate-confirm' | 'amendment';

/** `?action=` deep links from the list / dashboard cards (renew, terminate). */
function initialDialog(action: string | null): LeaseDialog | null {
  return action === 'renew' || action === 'terminate' ? action : null;
}

/**
 * Route wrapper: lease detail (`/leases/:leaseId`), live. Wires the lifecycle
 * actions to their endpoints:
 *  - edit → `PUT /leases/{id}` (ancillary fields only — `UpdateLease`),
 *  - renew → `POST /leases/{id}/renew` (navigates to the new draft lease),
 *  - terminate → `POST /leases/{id}/terminate` (form + confirmation),
 *  - add amendment → `POST /leases/{id}/amendments`,
 *  - record payment → `POST /leases/{payment_id}/payments`.
 */
function LeaseDetailPageRoute() {
  const navigate = useNavigate();
  const { t } = useTranslation();
  const { showToast } = useToast();
  const { leaseId } = useParams<{ leaseId: string }>();
  const [searchParams, setSearchParams] = useSearchParams();
  const { data, isLoading } = useLease(leaseId);
  const recordPayment = useRecordLeasePayment();
  const updateLease = useUpdateLease();
  const renewLease = useRenewLease();
  const terminateLease = useTerminateLease();
  const createAmendment = useCreateLeaseAmendment();
  const [paymentId, setPaymentId] = useState<string | null>(null);
  const [dialog, setDialogState] = useState<LeaseDialog | null>(() =>
    initialDialog(searchParams.get('action'))
  );
  const [termination, setTermination] = useState<TerminateLeaseFormData | null>(null);

  if (isLoading || !data) {
    return <LeaseLoadingNotice onBack={() => navigate('/leases/list')} isLoading={isLoading} />;
  }

  const lease = mapLeaseWithDetailsToUi(data);
  const payment = paymentId ? lease.upcomingPayments.find((p) => p.id === paymentId) : undefined;
  const isOpenLease =
    lease.status === 'active' ||
    lease.status === 'expiring_soon' ||
    lease.status === 'draft' ||
    lease.status === 'pending_signatures';
  const canRenewOrTerminate = lease.status === 'active' || lease.status === 'expiring_soon';
  // A deep-linked action that no longer applies (e.g. lease already terminated)
  // is ignored rather than opening a dialog the page would not offer.
  const activeDialog =
    dialog === 'edit' || dialog === 'amendment'
      ? isOpenLease
        ? dialog
        : null
      : dialog && canRenewOrTerminate
        ? dialog
        : null;

  const setDialog = (next: LeaseDialog | null) => {
    setDialogState(next);
    if (searchParams.has('action')) {
      const params = new URLSearchParams(searchParams);
      params.delete('action');
      setSearchParams(params, { replace: true });
    }
  };

  const fail = (err: unknown, titleKey: string, titleDefault: string, msgKey: string) =>
    showToast({
      type: 'error',
      title: t(titleKey, { defaultValue: titleDefault }),
      message: errorMessage(err, t(msgKey)),
    });

  const handleRecordPayment = async (form: RecordPaymentFormData) => {
    if (!payment) return;
    try {
      await recordPayment.mutateAsync({
        paymentId: payment.id,
        leaseId: lease.id,
        body: {
          paid_amount: form.paidAmount,
          payment_method: form.paymentMethod,
          payment_reference: form.paymentReference,
          paid_at: toDateTime(form.paidAt),
        },
      });
      setPaymentId(null);
      showToast({
        type: 'success',
        title: t('common.saved', { defaultValue: 'Saved' }),
        message: t('leases.payments.recorded', { defaultValue: 'Payment recorded' }),
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: t('common.saveFailed', { defaultValue: 'Save failed' }),
        message: errorMessage(
          err,
          t('leases.payments.recordFailed', { defaultValue: 'Could not record payment' })
        ),
      });
    }
  };

  const handleEdit = async (form: EditLeaseFormData) => {
    try {
      await updateLease.mutateAsync({
        id: lease.id,
        body: {
          tenant_phone: form.tenantPhone,
          parking_spaces: form.parkingSpaces,
          storage_units: form.storageUnits,
          max_occupants: form.maxOccupants,
          pets_allowed: form.petsAllowed,
          smoking_allowed: form.smokingAllowed,
          notes: form.notes,
        },
      });
      setDialog(null);
      showToast({
        type: 'success',
        title: t('common.saved', { defaultValue: 'Saved' }),
        message: t('leases.actions.edit.success'),
      });
    } catch (err) {
      fail(err, 'common.saveFailed', 'Save failed', 'leases.actions.edit.failed');
    }
  };

  const handleRenew = async (form: RenewLeaseFormData) => {
    try {
      const renewed = await renewLease.mutateAsync({
        id: lease.id,
        body: {
          new_end_date: form.newEndDate,
          new_monthly_rent: form.newMonthlyRent,
          new_security_deposit: form.newSecurityDeposit,
          term_months: form.termMonths,
          notes: form.notes,
        },
      });
      setDialog(null);
      showToast({
        type: 'success',
        title: t('common.saved', { defaultValue: 'Saved' }),
        message: t('leases.actions.renew.success'),
      });
      if (renewed?.id) navigate(`/leases/${renewed.id}`);
    } catch (err) {
      fail(err, 'common.saveFailed', 'Save failed', 'leases.actions.renew.failed');
    }
  };

  const handleTerminate = async () => {
    if (!termination) return;
    try {
      await terminateLease.mutateAsync({
        id: lease.id,
        body: {
          termination_reason: termination.reason,
          termination_notes: termination.notes,
          effective_date: termination.effectiveDate,
        },
      });
      setDialog(null);
      setTermination(null);
      showToast({
        type: 'success',
        title: t('common.updated', { defaultValue: 'Updated' }),
        message: t('leases.actions.terminate.success'),
      });
    } catch (err) {
      fail(err, 'common.updateFailed', 'Update failed', 'leases.actions.terminate.failed');
    }
  };

  const handleAmendment = async (form: AmendmentFormData) => {
    try {
      await createAmendment.mutateAsync({
        id: lease.id,
        body: {
          title: form.title,
          description: form.description,
          effective_date: form.effectiveDate,
          changes: form.newMonthlyRent != null ? { monthly_rent: String(form.newMonthlyRent) } : {},
        },
      });
      setDialog(null);
      showToast({
        type: 'success',
        title: t('common.created', { defaultValue: 'Created' }),
        message: t('leases.actions.amendment.success'),
      });
    } catch (err) {
      fail(err, 'common.createFailed', 'Create failed', 'leases.actions.amendment.failed');
    }
  };

  const unitLabel = `${lease.unit.buildingName} - ${lease.unit.number}`;

  return (
    <>
      <LeaseDetailPage
        lease={lease}
        onBack={() => navigate('/leases/list')}
        onEdit={() => setDialog('edit')}
        onRenew={() => setDialog('renew')}
        onTerminate={() => setDialog('terminate')}
        onAddAmendment={() => setDialog('amendment')}
        onRecordPayment={(id) => setPaymentId(id)}
      />
      {payment && (
        <RecordPaymentModal
          payment={payment}
          isSubmitting={recordPayment.isPending}
          onSubmit={handleRecordPayment}
          onClose={() => setPaymentId(null)}
        />
      )}
      {activeDialog === 'edit' && (
        <EditLeaseModal
          initial={{
            tenantPhone: data.lease.tenant_phone ?? undefined,
            parkingSpaces: data.lease.parking_spaces ?? 0,
            storageUnits: data.lease.storage_units ?? 0,
            maxOccupants: data.lease.max_occupants ?? undefined,
            petsAllowed: data.lease.pets_allowed ?? false,
            smokingAllowed: data.lease.smoking_allowed ?? false,
            notes: data.lease.notes ?? undefined,
          }}
          isSubmitting={updateLease.isPending}
          onSubmit={handleEdit}
          onClose={() => setDialog(null)}
        />
      )}
      {activeDialog === 'renew' && (
        <RenewLeaseModal
          currentEndDate={lease.endDate}
          currentRent={lease.rentAmount}
          currency={lease.currency}
          isSubmitting={renewLease.isPending}
          onSubmit={handleRenew}
          onClose={() => setDialog(null)}
        />
      )}
      {activeDialog === 'terminate' && (
        <TerminateLeaseModal
          initial={termination ?? undefined}
          onSubmit={(form) => {
            setTermination(form);
            setDialog('terminate-confirm');
          }}
          onClose={() => {
            setTermination(null);
            setDialog(null);
          }}
        />
      )}
      <ConfirmationDialog
        isOpen={activeDialog === 'terminate-confirm' && termination != null}
        variant="danger"
        title={t('leases.actions.terminate.confirmTitle')}
        message={t('leases.actions.terminate.confirmMessage', {
          unit: unitLabel,
          tenant: lease.tenant.name,
          date: termination
            ? new Date(`${termination.effectiveDate}T00:00:00Z`).toLocaleDateString(undefined, {
                timeZone: 'UTC',
              })
            : '',
        })}
        confirmLabel={t('leases.actions.terminate.confirm')}
        cancelLabel={t('common.back')}
        isLoading={terminateLease.isPending}
        onConfirm={handleTerminate}
        onCancel={() => setDialog('terminate')}
      />
      {activeDialog === 'amendment' && (
        <AddAmendmentModal
          currency={lease.currency}
          isSubmitting={createAmendment.isPending}
          onSubmit={handleAmendment}
          onClose={() => setDialog(null)}
        />
      )}
    </>
  );
}

/**
 * Route wrapper: create lease. Unit, tenant and template pickers are live; the
 * tenant's name/e-mail and the landlord (organization) name are filled from the
 * selected records so the backend gets a complete lease party block.
 */
function CreateLeasePageRoute() {
  const navigate = useNavigate();
  const { t } = useTranslation();
  const { showToast } = useToast();
  const { organizationId, organizationName } = useOrganization();
  const { data: templatesData, isLoading: templatesLoading } = useLeaseTemplates(organizationId);
  const { units, isLoading: unitsLoading } = useOrganizationUnits();
  const { data: membersData, isLoading: membersLoading } = useOrganizationMembers(organizationId);
  const createLease = useCreateLease();

  const templates = (templatesData ?? []).map(mapTemplateToUi).filter((tpl) => tpl.isActive);
  const tenants = selectTenantCandidates(membersData?.members ?? []);

  return (
    <CreateLeasePage
      units={units}
      tenants={tenants}
      templates={templates}
      isLoading={templatesLoading || unitsLoading || membersLoading}
      isSubmitting={createLease.isPending}
      onCancel={() => navigate('/leases/list')}
      onSubmit={async (formData: CreateLeaseData) => {
        const tenant = tenants.find((x) => x.id === formData.tenantId);
        if (!formData.unitId || !tenant) {
          showToast({
            type: 'error',
            title: t('common.createFailed', { defaultValue: 'Create failed' }),
            message: formData.unitId
              ? t('leases.form.errors.tenantRequired')
              : t('leases.form.errors.unitRequired'),
          });
          return;
        }
        try {
          const termMonths = monthsBetween(formData.startDate, formData.endDate);
          await createLease.mutateAsync({
            organization_id: organizationId,
            unit_id: formData.unitId,
            template_id: formData.templateId || undefined,
            landlord_name: organizationName ?? '',
            tenant_user_id: tenant.id,
            tenant_name: tenant.name,
            tenant_email: tenant.email,
            start_date: formData.startDate,
            end_date: formData.endDate,
            term_months: termMonths,
            monthly_rent: formData.rentAmount,
            security_deposit: formData.depositAmount ?? 0,
            rent_due_day: formData.paymentDayOfMonth,
            notes: formData.notes,
          });
          showToast({
            type: 'success',
            title: t('common.created', { defaultValue: 'Created' }),
            message: t('leases.created', { defaultValue: 'Lease created' }),
          });
          navigate('/leases/list');
        } catch (err) {
          showToast({
            type: 'error',
            title: t('common.createFailed', { defaultValue: 'Create failed' }),
            message: errorMessage(
              err,
              t('leases.createFailed', { defaultValue: 'Could not create lease' })
            ),
          });
        }
      }}
    />
  );
}

/** Route wrapper: lease templates — live list with create / edit / (de)activate. */
function TemplatesPageRoute() {
  const { t } = useTranslation();
  const { showToast } = useToast();
  const { organizationId } = useOrganization();
  const { data, isLoading } = useLeaseTemplates(organizationId);
  const createTemplate = useCreateLeaseTemplate();
  const updateTemplate = useUpdateLeaseTemplate();

  const templates = (data ?? []).map(mapTemplateToUi);

  const failToast = (err: unknown, fallbackKey: string, fallback: string) =>
    showToast({
      type: 'error',
      title: t('common.saveFailed', { defaultValue: 'Save failed' }),
      message: errorMessage(err, t(fallbackKey, { defaultValue: fallback })),
    });

  const update = async (id: string, body: Record<string, unknown>, successKey: string) => {
    try {
      await updateTemplate.mutateAsync({
        id,
        body: { organization_id: organizationId, ...body },
      });
      showToast({
        type: 'success',
        title: t('common.updated', { defaultValue: 'Updated' }),
        message: t(successKey),
      });
    } catch (err) {
      failToast(err, 'leases.templates.updateFailed', 'Could not update template');
    }
  };

  return (
    <TemplatesPage
      templates={templates}
      isLoading={isLoading}
      isSubmitting={createTemplate.isPending || updateTemplate.isPending}
      showRentDefaults={false}
      onCreateTemplate={async (formData) => {
        try {
          const fields = mapTemplateDataToApi(formData);
          await createTemplate.mutateAsync({
            organization_id: organizationId,
            ...fields,
            name: formData.name,
            content_html: formData.content,
          });
          showToast({
            type: 'success',
            title: t('common.created', { defaultValue: 'Created' }),
            message: t('leases.templates.created', { defaultValue: 'Template created' }),
          });
        } catch (err) {
          failToast(err, 'leases.templates.createFailed', 'Could not create template');
        }
      }}
      onUpdateTemplate={(id, formData) =>
        update(
          id,
          { ...mapTemplateDataToApi(formData), is_active: formData.isActive },
          'leases.templates.updated'
        )
      }
      onToggleActive={(id, isActive) =>
        update(
          id,
          { is_active: isActive },
          isActive ? 'leases.templates.activated' : 'leases.templates.deactivated'
        )
      }
    />
  );
}

/** Application statuses the list filter offers (UI values). */
const UI_APPLICATION_STATUSES: readonly UiApplicationStatus[] = [
  'submitted',
  'under_review',
  'screening',
  'approved',
  'rejected',
  'withdrawn',
  'draft',
];

/**
 * Application list filters persisted in the URL (`?status=&unit=&page=`). The
 * backend `GET /leases/applications` filters by `status` + `unit_id` only.
 */
export function readApplicationListParams(params: URLSearchParams): {
  filters: Omit<ApplicationListParams, 'page' | 'pageSize'>;
  page: number;
} {
  const rawStatus = params.get('status');
  return {
    filters: {
      status: UI_APPLICATION_STATUSES.find((s) => s === rawStatus),
      unitId: params.get('unit') || undefined,
    },
    page: positiveInt(params.get('page')) ?? 1,
  };
}

/** Serialize application list filters back into URL search params. */
export function writeApplicationListParams(p: ApplicationListParams): URLSearchParams {
  const out = new URLSearchParams();
  if (p.status) out.set('status', p.status);
  if (p.unitId) out.set('unit', p.unitId);
  if (p.page > 1) out.set('page', String(p.page));
  return out;
}

/**
 * UI application status → backend `tenant_application_status`. The UI folds
 * `screening_pending` + `screening_complete` into one `screening` status; the
 * server filters by a single value, so the filter targets the pending one
 * (applications still waiting on a screening result).
 */
function toApiApplicationStatus(status: UiApplicationStatus | undefined): string | undefined {
  return status === 'screening' ? 'screening_pending' : status;
}

/** Route wrapper: applications list, live, URL-persisted status + unit filters. */
function ApplicationsPageRoute() {
  const navigate = useNavigate();
  const { organizationId } = useOrganization();
  const [searchParams, setSearchParams] = useSearchParams();
  const { filters, page } = readApplicationListParams(searchParams);
  const { units } = useOrganizationUnits();
  const { data, isLoading } = useApplications({
    organization_id: organizationId,
    status: toApiApplicationStatus(filters.status),
    unit_id: filters.unitId,
    limit: LIST_PAGE_SIZE,
    offset: (page - 1) * LIST_PAGE_SIZE,
  });

  const applications = (data?.items ?? []).map(mapApplicationSummaryToUi);
  return (
    <ApplicationsPage
      applications={applications}
      total={data?.total ?? 0}
      units={units}
      isLoading={isLoading}
      initialFilters={filters}
      initialPage={page}
      pageSize={LIST_PAGE_SIZE}
      showSearch={false}
      onNavigateToView={(id) => navigate(`/leases/applications/${id}`)}
      onNavigateToReview={(id) => navigate(`/leases/applications/${id}`)}
      onFilterChange={(next) =>
        setSearchParams(writeApplicationListParams(next), { replace: true })
      }
    />
  );
}

/** Route wrapper: application detail (`/leases/applications/:applicationId`), live. */
function ApplicationDetailPageRoute() {
  const navigate = useNavigate();
  const { t } = useTranslation();
  const { showToast } = useToast();
  const { organizationId } = useOrganization();
  const { applicationId } = useParams<{ applicationId: string }>();
  const { data, isLoading } = useApplication(applicationId);
  // The single-application payload carries only `unit_id`; the list summary
  // has the unit/building labels, so reuse the (usually cached) list query.
  const { data: listData } = useApplications({ organization_id: organizationId });
  const initiateScreening = useInitiateScreening();
  const reviewApplication = useReviewApplication();
  // Screening rows have no list endpoint — show the rows this session created.
  const [screenings, setScreenings] = useState<UiTenantScreening[]>([]);

  if (isLoading || !data) {
    return (
      <LeaseLoadingNotice
        onBack={() => navigate('/leases/applications')}
        isLoading={isLoading}
        label="application"
      />
    );
  }

  const application = mapApplicationToUi(data);
  const summary = listData?.items.find((a) => a.id === data.id);

  const handleInitiateScreening = async (types: UiScreeningType[]) => {
    try {
      const rows = await initiateScreening.mutateAsync({
        id: data.id,
        body: { organization_id: organizationId, screening_types: types },
      });
      setScreenings((prev) => [...prev, ...rows.map(mapScreeningToUi)]);
      showToast({
        type: 'success',
        title: t('common.submitted', { defaultValue: 'Submitted' }),
        message: t('leases.applications.screeningStarted', {
          defaultValue: 'Screening requested',
        }),
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: t('common.submitFailed', { defaultValue: 'Submit failed' }),
        message: errorMessage(
          err,
          t('leases.applications.screeningFailed', {
            defaultValue: 'Could not start screening',
          })
        ),
      });
    }
  };

  const handleReview = async (review: ReviewApplicationData) => {
    try {
      await reviewApplication.mutateAsync({
        id: data.id,
        body: {
          status: review.decision === 'approve' ? 'approved' : 'rejected',
          decision_notes: review.notes,
        },
      });
      showToast({
        type: 'success',
        title: t('common.saved', { defaultValue: 'Saved' }),
        message:
          review.decision === 'approve'
            ? t('leases.applications.approved', { defaultValue: 'Application approved' })
            : t('leases.applications.rejected', { defaultValue: 'Application rejected' }),
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: t('common.saveFailed', { defaultValue: 'Save failed' }),
        message: errorMessage(
          err,
          t('leases.applications.reviewFailed', { defaultValue: 'Could not save the review' })
        ),
      });
    }
  };

  return (
    <ApplicationDetailPage
      application={application}
      screenings={screenings}
      unitInfo={{
        number: summary?.unit_name ?? '',
        buildingName: summary?.building_name ?? '',
      }}
      isSubmitting={initiateScreening.isPending || reviewApplication.isPending}
      onBack={() => navigate('/leases/applications')}
      onInitiateScreening={handleInitiateScreening}
      onReview={handleReview}
      onCreateLease={() => navigate('/leases/new')}
    />
  );
}

/** Route wrapper: violations list, live. */
function ViolationsPageRoute() {
  const navigate = useNavigate();
  const { organizationId } = useOrganization();
  const { data, isLoading } = useViolations();
  const leasesQuery = useLeases({ organization_id: organizationId });

  const violations = (data ?? []).map(mapViolationSummaryToUi);
  const leases = (leasesQuery.data?.items ?? []).map(mapLeaseSummaryToUi);

  return (
    <ViolationsPage
      violations={violations}
      leases={leases}
      isLoading={isLoading}
      onViewViolation={(id) => navigate(`/leases/violations/${id}`)}
      onResolveViolation={(id) => navigate(`/leases/violations/${id}`)}
      onDisputeViolation={(id) => navigate(`/leases/violations/${id}`)}
      onCreateViolation={() => navigate('/leases/violations/new')}
      // No violations export endpoint exists (`routes/violations.rs`), so the
      // export button is hidden by omitting `onExportReport`.
    />
  );
}

/** Route wrapper: create violation, live. Lease selector is live. */
function CreateViolationPageRoute() {
  const navigate = useNavigate();
  const { t } = useTranslation();
  const { showToast } = useToast();
  const { organizationId } = useOrganization();
  const leasesQuery = useLeases({ organization_id: organizationId });
  const createViolation = useCreateViolation();

  const leases = (leasesQuery.data?.items ?? []).map(mapLeaseSummaryToUi);

  return (
    <CreateViolationPage
      leases={leases}
      isSubmitting={createViolation.isPending}
      onBack={() => navigate('/leases/violations')}
      onSubmit={async (formData: CreateViolationData) => {
        try {
          const body: CreateViolationRequest = {
            category: mapViolationCategory(formData.violationType),
            severity: mapSeverityToApi(formData.severity),
            title: t(`leases.violations.type.${formData.violationType}`),
            description: formData.description,
            occurred_at: toDateTime(formData.violationDate),
          };
          await createViolation.mutateAsync(body);
          showToast({
            type: 'success',
            title: t('common.created', { defaultValue: 'Created' }),
            message: t('leases.violationRecorded', { defaultValue: 'Violation recorded' }),
          });
          navigate('/leases/violations');
        } catch (err) {
          showToast({
            type: 'error',
            title: t('common.createFailed', { defaultValue: 'Create failed' }),
            message:
              err instanceof Error
                ? err.message
                : t('leases.violationCreateFailed', {
                    defaultValue: 'Could not record violation',
                  }),
          });
        }
      }}
    />
  );
}

/**
 * Route wrapper: violation detail (`/leases/violations/:violationId`), live.
 * Managers can resolve; other members (tenants/residents) can dispute (files an
 * appeal); anyone can attach evidence.
 */
function ViolationDetailPageRoute() {
  const navigate = useNavigate();
  const { t } = useTranslation();
  const { showToast } = useToast();
  const { user } = useAuth();
  const isManager = isManagerRole(user?.role);
  const { violationId } = useParams<{ violationId: string }>();
  const { data, isLoading } = useViolation(violationId);
  const { data: evidenceData } = useViolationEvidence(violationId);
  const updateViolation = useUpdateViolation();
  const createAppeal = useCreateViolationAppeal();
  const addEvidence = useAddViolationEvidence();
  const [isUploading, setIsUploading] = useState(false);

  if (isLoading || !data) {
    return (
      <LeaseLoadingNotice
        onBack={() => navigate('/leases/violations')}
        isLoading={isLoading}
        label="violation"
      />
    );
  }

  const violation: UiViolationWithDetails = {
    ...mapViolationToUi(data),
    evidence: (evidenceData ?? []).map(mapEvidenceToUi),
  };

  const handleResolve = async (id: string, notes: string) => {
    try {
      await updateViolation.mutateAsync({
        id,
        body: { status: 'resolved', resolution_notes: notes },
      });
      showToast({
        type: 'success',
        title: t('common.updated', { defaultValue: 'Updated' }),
        message: t('leases.violations.resolved', { defaultValue: 'Violation resolved' }),
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: t('common.updateFailed', { defaultValue: 'Update failed' }),
        message: errorMessage(
          err,
          t('leases.violations.resolveFailed', { defaultValue: 'Could not resolve violation' })
        ),
      });
    }
  };

  const handleDispute = async (id: string, reason: string) => {
    try {
      await createAppeal.mutateAsync({ id, body: { reason } });
      showToast({
        type: 'success',
        title: t('common.submitted', { defaultValue: 'Submitted' }),
        message: t('leases.violations.disputed', { defaultValue: 'Dispute submitted' }),
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: t('common.submitFailed', { defaultValue: 'Submit failed' }),
        message: errorMessage(
          err,
          t('leases.violations.disputeFailed', { defaultValue: 'Could not submit dispute' })
        ),
      });
    }
  };

  const handleUploadEvidence = async (id: string, files: FileList) => {
    setIsUploading(true);
    try {
      for (const file of Array.from(files)) {
        const contentType = file.type || 'application/octet-stream';
        const target = await createUploadUrl({
          file_name: file.name,
          mime_type: contentType,
          size_bytes: file.size,
        });
        await uploadFileToPresignedUrl(target.url, file, target.content_type || contentType);
        await addEvidence.mutateAsync({
          id,
          body: {
            file_name: file.name,
            file_type: contentType,
            file_size: file.size,
            storage_path: target.file_key,
          },
        });
      }
      showToast({
        type: 'success',
        title: t('common.saved', { defaultValue: 'Saved' }),
        message: t('leases.violations.evidenceUploaded', { defaultValue: 'Evidence uploaded' }),
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: t('common.saveFailed', { defaultValue: 'Save failed' }),
        message: errorMessage(
          err,
          t('leases.violations.evidenceUploadFailed', {
            defaultValue: 'Could not upload evidence',
          })
        ),
      });
    } finally {
      setIsUploading(false);
    }
  };

  return (
    <ViolationDetailPage
      violation={violation}
      isManager={isManager}
      isTenant={!isManager}
      isUploadingEvidence={isUploading}
      onBack={() => navigate('/leases/violations')}
      onResolve={handleResolve}
      onDispute={handleDispute}
      onUploadEvidence={handleUploadEvidence}
    />
  );
}

// ============================================================================
// Helpers
// ============================================================================

/** Whole months between two ISO date strings (min 1). */
function monthsBetween(start: string, end: string): number {
  const s = new Date(start);
  const e = new Date(end);
  if (Number.isNaN(s.getTime()) || Number.isNaN(e.getTime())) {
    return 12;
  }
  const months = (e.getFullYear() - s.getFullYear()) * 12 + (e.getMonth() - s.getMonth());
  return Math.max(1, months);
}

/** Coerce a date-only string into an RFC3339 timestamp the backend accepts. */
function toDateTime(date: string): string {
  return date.includes('T') ? date : `${date}T00:00:00Z`;
}

type LeaseEntity = 'lease' | 'application' | 'violation';

/**
 * Lightweight loading / not-found notice for single-entity routes while their
 * fetch is in flight (or when the entity is missing).
 */
function LeaseLoadingNotice({
  onBack,
  isLoading,
  label = 'lease',
}: {
  onBack: () => void;
  isLoading?: boolean;
  label?: LeaseEntity;
}) {
  const { t } = useTranslation();
  return (
    <div className="mx-auto max-w-md py-16 text-center">
      <h1 className="text-lg font-semibold text-gray-900">
        {isLoading ? t(`leases.notice.loading.${label}`) : t(`leases.notice.notFound.${label}`)}
      </h1>
      {!isLoading && (
        <p className="mt-2 text-sm text-gray-500">{t(`leases.notice.notFoundBody.${label}`)}</p>
      )}
      <button
        type="button"
        onClick={onBack}
        className="mt-6 rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700"
      >
        {t(`leases.notice.back.${label}`)}
      </button>
    </div>
  );
}

/** Leases routes (Epic 19). */
export function leaseRoutes() {
  return (
    <>
      <Route
        path="/leases"
        element={
          <ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>
            <LeasesDashboardPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/leases/list"
        element={
          <ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>
            <LeasesListPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/leases/new"
        element={
          <ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>
            <CreateLeasePageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/leases/templates"
        element={
          <ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>
            <TemplatesPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/leases/applications"
        element={
          <ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>
            <ApplicationsPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/leases/applications/:applicationId"
        element={
          <ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>
            <ApplicationDetailPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/leases/violations"
        element={
          <ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>
            <ViolationsPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/leases/violations/new"
        element={
          <ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>
            <CreateViolationPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/leases/violations/:violationId"
        element={
          <ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>
            <ViolationDetailPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/leases/:leaseId"
        element={
          <ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>
            <LeaseDetailPageRoute />
          </ProtectedRoute>
        }
      />
    </>
  );
}
