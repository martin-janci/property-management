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
 * single entity and map it best-effort onto the richer UI prop shapes (some
 * nested data — e.g. tenant user records, screening rows, violation evidence —
 * has no list endpoint yet, so those collections render empty). The create
 * forms' building/unit/tenant selectors reuse what the API exposes today
 * (`useBuildings`, live templates); unit/tenant pickers stay empty until those
 * endpoints land.
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
  Violation as ApiViolation,
  ViolationCategory as ApiViolationCategory,
  ViolationSeverity as ApiViolationSeverity,
  ViolationStatus as ApiViolationStatus,
  ViolationSummary as ApiViolationSummary,
  CreateViolationRequest,
} from '@ppt/api-client';
import {
  useApplication,
  useApplications,
  useBuildings,
  useCreateLease,
  useCreateViolation,
  useExpiringLeases,
  useLease,
  useLeaseStatistics,
  useLeases,
  useLeaseTemplates,
  useViolation,
  useViolations,
} from '@ppt/api-client';
import { lazy } from 'react';
import { useTranslation } from 'react-i18next';
import { Route, useNavigate, useParams } from 'react-router-dom';
import { ProtectedRoute, useToast } from '../../components';
import type {
  CreateLeaseData,
  CreateViolationData,
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
  ViolationSeverity as UiViolationSeverity,
  ViolationStatus as UiViolationStatus,
  ViolationSummary as UiViolationSummary,
  ViolationType as UiViolationType,
  ViolationWithDetails as UiViolationWithDetails,
} from '../../features/leases/types';
import { useOrganization } from '../../hooks/useOrganization';

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

const DEFAULT_CURRENCY = 'EUR';

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
function mapLeaseSummaryToUi(s: ApiLeaseSummary): UiLeaseSummary {
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
    currency: DEFAULT_CURRENCY,
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
function mapStatisticsToUi(s: ApiLeaseStatistics): UiLeaseStatistics {
  return {
    totalLeases: s.total_leases,
    activeLeases: s.active_leases,
    expiringLeases: s.expiring_soon,
    expiredLeases: 0,
    pendingApplications: s.pending_applications,
    occupancyRate: s.occupancy_rate,
    totalMonthlyRent: Number(s.total_monthly_rent),
    overduePayments: 0,
    currency: DEFAULT_CURRENCY,
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
function mapPaymentToUi(p: ApiLeasePayment): UiLeasePayment {
  return {
    id: p.id,
    leaseId: p.lease_id,
    dueDate: p.due_date,
    amount: Number(p.amount),
    currency: DEFAULT_CURRENCY,
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
function mapLeaseToUi(l: ApiLease): UiLease {
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
    currency: DEFAULT_CURRENCY,
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
        currency: DEFAULT_CURRENCY,
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
    />
  );
}

/** Route wrapper: leases list, live across the organization. */
function LeasesListPageRoute() {
  const navigate = useNavigate();
  const { organizationId } = useOrganization();
  const { data, isLoading } = useLeases({ organization_id: organizationId });
  const { data: buildingsData } = useBuildings();

  const leases = (data?.items ?? []).map(mapLeaseSummaryToUi);
  const buildings = (buildingsData?.items ?? []).map((b) => ({ id: b.id, name: b.name }));

  return (
    <LeasesListPage
      leases={leases}
      total={data?.total ?? 0}
      buildings={buildings}
      isLoading={isLoading}
      onNavigateToCreate={() => navigate('/leases/new')}
      onNavigateToView={(id) => navigate(`/leases/${id}`)}
      onRenewLease={(id) => navigate(`/leases/${id}`)}
      onTerminateLease={(id) => navigate(`/leases/${id}`)}
      onFilterChange={() => {}}
    />
  );
}

/** Route wrapper: lease detail (`/leases/:leaseId`), live. */
function LeaseDetailPageRoute() {
  const navigate = useNavigate();
  const { leaseId } = useParams<{ leaseId: string }>();
  const { data, isLoading } = useLease(leaseId);

  if (isLoading || !data) {
    return <LeaseLoadingNotice onBack={() => navigate('/leases/list')} isLoading={isLoading} />;
  }

  const lease = mapLeaseWithDetailsToUi(data);
  return (
    <LeaseDetailPage
      lease={lease}
      onBack={() => navigate('/leases/list')}
      onEdit={(id) => navigate(`/leases/${id}`)}
      onRenew={(id) => navigate(`/leases/${id}`)}
      onTerminate={(id) => navigate(`/leases/${id}`)}
      onAddAmendment={(id) => navigate(`/leases/${id}`)}
      onRecordPayment={() => {}}
    />
  );
}

/** Route wrapper: create lease. Templates selector is live; unit/tenant pending. */
function CreateLeasePageRoute() {
  const navigate = useNavigate();
  const { showToast } = useToast();
  const { organizationId } = useOrganization();
  const { data: templatesData } = useLeaseTemplates(organizationId);
  const createLease = useCreateLease();

  const templates = (templatesData ?? []).map(mapTemplateToUi);

  return (
    <CreateLeasePage
      units={[]}
      tenants={[]}
      templates={templates}
      isSubmitting={createLease.isPending}
      onCancel={() => navigate('/leases/list')}
      onSubmit={async (formData: CreateLeaseData) => {
        try {
          const termMonths = monthsBetween(formData.startDate, formData.endDate);
          await createLease.mutateAsync({
            organization_id: organizationId,
            unit_id: formData.unitId,
            template_id: formData.templateId,
            // The unit/tenant pickers are not yet wired to an endpoint, so the
            // landlord/tenant identity fields are submitted blank for the
            // backend to resolve from the unit/application context.
            landlord_name: '',
            tenant_user_id: formData.tenantId || undefined,
            tenant_name: '',
            tenant_email: '',
            start_date: formData.startDate,
            end_date: formData.endDate,
            term_months: termMonths,
            monthly_rent: formData.rentAmount,
            security_deposit: formData.depositAmount ?? 0,
            rent_due_day: formData.paymentDayOfMonth,
            notes: formData.notes,
          });
          showToast({ type: 'success', title: 'Created', message: 'Lease created' });
          navigate('/leases/list');
        } catch (err) {
          showToast({
            type: 'error',
            title: 'Create failed',
            message: err instanceof Error ? err.message : 'Could not create lease',
          });
        }
      }}
    />
  );
}

/** Route wrapper: lease templates, live list. */
function TemplatesPageRoute() {
  const { organizationId } = useOrganization();
  const { data, isLoading } = useLeaseTemplates(organizationId);

  const templates = (data ?? []).map(mapTemplateToUi);
  return (
    <TemplatesPage
      templates={templates}
      isLoading={isLoading}
      onCreateTemplate={() => {}}
      onUpdateTemplate={() => {}}
      onDeleteTemplate={() => {}}
      onToggleActive={() => {}}
    />
  );
}

/** Route wrapper: applications list, live. Unit selector pending an endpoint. */
function ApplicationsPageRoute() {
  const navigate = useNavigate();
  const { organizationId } = useOrganization();
  const { data, isLoading } = useApplications({ organization_id: organizationId });

  const applications = (data?.items ?? []).map(mapApplicationSummaryToUi);
  return (
    <ApplicationsPage
      applications={applications}
      total={data?.total ?? 0}
      units={[]}
      isLoading={isLoading}
      onNavigateToView={(id) => navigate(`/leases/applications/${id}`)}
      onNavigateToReview={(id) => navigate(`/leases/applications/${id}`)}
      onFilterChange={() => {}}
    />
  );
}

/** Route wrapper: application detail (`/leases/applications/:applicationId`), live. */
function ApplicationDetailPageRoute() {
  const navigate = useNavigate();
  const { applicationId } = useParams<{ applicationId: string }>();
  const { data, isLoading } = useApplication(applicationId);

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
  return (
    <ApplicationDetailPage
      application={application}
      // Screening rows have no list endpoint yet; the screening section renders
      // empty until that lands.
      screenings={[]}
      unitInfo={{ number: '', buildingName: '' }}
      onBack={() => navigate('/leases/applications')}
      onInitiateScreening={() => {}}
      onReview={() => {}}
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
      onExportReport={() => {}}
    />
  );
}

/** Route wrapper: create violation, live. Lease selector is live. */
function CreateViolationPageRoute() {
  const navigate = useNavigate();
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
            title: `${formData.violationType} violation`,
            description: formData.description,
            occurred_at: toDateTime(formData.violationDate),
          };
          await createViolation.mutateAsync(body);
          showToast({ type: 'success', title: 'Created', message: 'Violation recorded' });
          navigate('/leases/violations');
        } catch (err) {
          showToast({
            type: 'error',
            title: 'Create failed',
            message: err instanceof Error ? err.message : 'Could not record violation',
          });
        }
      }}
    />
  );
}

/** Route wrapper: violation detail (`/leases/violations/:violationId`), live. */
function ViolationDetailPageRoute() {
  const navigate = useNavigate();
  const { violationId } = useParams<{ violationId: string }>();
  const { data, isLoading } = useViolation(violationId);

  if (isLoading || !data) {
    return (
      <LeaseLoadingNotice
        onBack={() => navigate('/leases/violations')}
        isLoading={isLoading}
        label="violation"
      />
    );
  }

  const violation = mapViolationToUi(data);
  return (
    <ViolationDetailPage
      violation={violation}
      onBack={() => navigate('/leases/violations')}
      onResolve={() => {}}
      onDispute={() => {}}
      onUploadEvidence={() => {}}
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
  label?: string;
}) {
  const { t } = useTranslation();
  const entity = `${label[0].toUpperCase()}${label.slice(1)}`;
  return (
    <div className="mx-auto max-w-md py-16 text-center">
      <h1 className="text-lg font-semibold text-gray-900">
        {isLoading
          ? `Loading ${label}…`
          : t('errors.entityNotFound', { entity, defaultValue: '{{entity}} not found' })}
      </h1>
      {!isLoading && (
        <p className="mt-2 text-sm text-gray-500">
          This {label} could not be loaded. It may have been removed or you may not have access.
        </p>
      )}
      <button
        type="button"
        onClick={onBack}
        className="mt-6 rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700"
      >
        Back to leases
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
          <ProtectedRoute>
            <LeasesDashboardPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/leases/list"
        element={
          <ProtectedRoute>
            <LeasesListPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/leases/new"
        element={
          <ProtectedRoute>
            <CreateLeasePageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/leases/templates"
        element={
          <ProtectedRoute>
            <TemplatesPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/leases/applications"
        element={
          <ProtectedRoute>
            <ApplicationsPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/leases/applications/:applicationId"
        element={
          <ProtectedRoute>
            <ApplicationDetailPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/leases/violations"
        element={
          <ProtectedRoute>
            <ViolationsPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/leases/violations/new"
        element={
          <ProtectedRoute>
            <CreateViolationPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/leases/violations/:violationId"
        element={
          <ProtectedRoute>
            <ViolationDetailPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/leases/:leaseId"
        element={
          <ProtectedRoute>
            <LeaseDetailPageRoute />
          </ProtectedRoute>
        }
      />
    </>
  );
}
