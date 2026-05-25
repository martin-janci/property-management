/**
 * Lease management types.
 * Epic 19: Lease Management & Tenant Screening
 */

// Lease status options
export type LeaseStatus =
  | 'draft'
  | 'pending_signatures'
  | 'active'
  | 'expiring_soon'
  | 'expired'
  | 'terminated'
  | 'renewed';

// Application status options
export type ApplicationStatus =
  | 'draft'
  | 'submitted'
  | 'under_review'
  | 'screening'
  | 'approved'
  | 'rejected'
  | 'withdrawn';

// Screening status options
export type ScreeningStatus = 'pending' | 'in_progress' | 'completed' | 'failed';

// Screening type options
export type ScreeningType =
  | 'credit_check'
  | 'background_check'
  | 'employment_verification'
  | 'rental_history'
  | 'income_verification';

// Payment status options
export type PaymentStatus = 'pending' | 'paid' | 'overdue' | 'partial' | 'waived';

// Payment method options
export type PaymentMethod =
  | 'bank_transfer'
  | 'credit_card'
  | 'debit_card'
  | 'cash'
  | 'check'
  | 'other';

// E-signature status on a lease (mirrors backend SignatureRequestStatus)
export type LeaseSignatureStatus =
  | 'pending'
  | 'in_progress'
  | 'completed'
  | 'declined'
  | 'expired'
  | 'cancelled';

// Lease interface
export interface Lease {
  id: string;
  organizationId: string;
  unitId: string;
  templateId?: string;
  tenantId: string;
  status: LeaseStatus;
  startDate: string;
  endDate: string;
  rentAmount: number;
  currency: string;
  depositAmount?: number;
  paymentDayOfMonth: number;
  terms?: string;
  notes?: string;
  signedAt?: string;
  terminatedAt?: string;
  terminationReason?: string;
  createdAt: string;
  updatedAt: string;
  createdBy: string;
  /** ID of the primary document attached to this lease, used for e-signatures */
  documentId?: string;
  /** Current e-signature request status, if any */
  signatureStatus?: LeaseSignatureStatus;
}

// Lease summary for list views
export interface LeaseSummary {
  id: string;
  unitId: string;
  unitNumber: string;
  buildingName: string;
  tenantId: string;
  tenantName: string;
  tenantEmail: string;
  status: LeaseStatus;
  startDate: string;
  endDate: string;
  rentAmount: number;
  currency: string;
  daysUntilExpiry?: number;
  /** Current e-signature request status, if any */
  signatureStatus?: LeaseSignatureStatus;
  /** ID of the primary document attached to this lease */
  documentId?: string;
}

// Lease with full details
export interface LeaseWithDetails extends Lease {
  unit: {
    id: string;
    number: string;
    buildingId: string;
    buildingName: string;
  };
  tenant: {
    id: string;
    name: string;
    email: string;
    phone?: string;
  };
  amendments: LeaseAmendment[];
  upcomingPayments: LeasePayment[];
  reminders: LeaseReminder[];
}

// Lease amendment
export interface LeaseAmendment {
  id: string;
  leaseId: string;
  effectiveDate: string;
  description: string;
  previousRent?: number;
  newRent?: number;
  previousTerms?: string;
  newTerms?: string;
  createdAt: string;
  createdBy: string;
}

// Lease payment
export interface LeasePayment {
  id: string;
  leaseId: string;
  dueDate: string;
  amount: number;
  currency: string;
  status: PaymentStatus;
  paidAmount?: number;
  paidAt?: string;
  paymentMethod?: PaymentMethod;
  reference?: string;
  notes?: string;
}

// Lease reminder
export interface LeaseReminder {
  id: string;
  leaseId: string;
  reminderType: string;
  scheduledFor: string;
  message?: string;
  isSent: boolean;
  sentAt?: string;
}

// Tenant application
export interface LeaseApplication {
  id: string;
  organizationId: string;
  unitId: string;
  status: ApplicationStatus;
  applicantName: string;
  applicantEmail: string;
  applicantPhone?: string;
  currentAddress?: string;
  employerName?: string;
  employerPhone?: string;
  annualIncome?: number;
  moveInDate?: string;
  numberOfOccupants?: number;
  hasPets?: boolean;
  petDetails?: string;
  references?: string;
  notes?: string;
  submittedAt?: string;
  reviewedAt?: string;
  reviewedBy?: string;
  reviewNotes?: string;
  createdAt: string;
  updatedAt: string;
}

// Application summary for list views
export interface ApplicationSummary {
  id: string;
  unitId: string;
  unitNumber: string;
  buildingName: string;
  applicantName: string;
  applicantEmail: string;
  status: ApplicationStatus;
  submittedAt?: string;
  createdAt: string;
}

// Tenant screening
export interface TenantScreening {
  id: string;
  applicationId: string;
  screeningType: ScreeningType;
  status: ScreeningStatus;
  provider?: string;
  requestedAt: string;
  completedAt?: string;
  result?: 'pass' | 'fail' | 'review_required';
  score?: number;
  details?: string;
  consentGivenAt?: string;
}

// Lease template
export interface LeaseTemplate {
  id: string;
  organizationId: string;
  name: string;
  description?: string;
  content: string;
  defaultRentAmount?: number;
  defaultCurrency?: string;
  defaultDepositMonths?: number;
  defaultLeaseDurationMonths?: number;
  isActive: boolean;
  createdAt: string;
  updatedAt: string;
  createdBy: string;
}

// Dashboard statistics
export interface LeaseStatistics {
  totalLeases: number;
  activeLeases: number;
  expiringLeases: number;
  expiredLeases: number;
  pendingApplications: number;
  occupancyRate: number;
  totalMonthlyRent: number;
  overduePayments: number;
  currency: string;
}

// Expiration overview
export interface ExpirationOverview {
  expiringIn30Days: LeaseSummary[];
  expiringIn60Days: LeaseSummary[];
  expiringIn90Days: LeaseSummary[];
  expired: LeaseSummary[];
}

// Payment summary
export interface PaymentSummary {
  leaseId: string;
  unitNumber: string;
  tenantName: string;
  dueDate: string;
  amount: number;
  currency: string;
  daysOverdue: number;
}

// Form data types
export interface CreateLeaseData {
  unitId: string;
  templateId?: string;
  tenantId: string;
  startDate: string;
  endDate: string;
  rentAmount: number;
  currency: string;
  depositAmount?: number;
  paymentDayOfMonth: number;
  terms?: string;
  notes?: string;
}

export interface CreateApplicationData {
  unitId: string;
  applicantName: string;
  applicantEmail: string;
  applicantPhone?: string;
  currentAddress?: string;
  employerName?: string;
  employerPhone?: string;
  annualIncome?: number;
  moveInDate?: string;
  numberOfOccupants?: number;
  hasPets?: boolean;
  petDetails?: string;
  references?: string;
  notes?: string;
}

export interface CreateTemplateData {
  name: string;
  description?: string;
  content: string;
  defaultRentAmount?: number;
  defaultCurrency?: string;
  defaultDepositMonths?: number;
  defaultLeaseDurationMonths?: number;
  isActive?: boolean;
}

export interface CreateAmendmentData {
  effectiveDate: string;
  description: string;
  newRent?: number;
  newTerms?: string;
}

export interface RecordPaymentData {
  dueDate: string;
  amount: number;
  currency: string;
  paidAmount: number;
  paymentMethod: PaymentMethod;
  reference?: string;
  notes?: string;
}

export interface TerminateLeaseData {
  terminationDate: string;
  reason: string;
  notes?: string;
}

export interface RenewLeaseData {
  newEndDate: string;
  newRentAmount?: number;
  notes?: string;
}

export interface ReviewApplicationData {
  decision: 'approve' | 'reject';
  notes?: string;
}

// ============================================
// Violation Types - UC-34 Violations Tracking
// ============================================

// Violation type enum
export type ViolationType =
  | 'noise'
  | 'damage'
  | 'unauthorized_occupant'
  | 'late_payment'
  | 'pet_policy'
  | 'parking'
  | 'cleanliness'
  | 'illegal_activity'
  | 'lease_terms'
  | 'other';

// Violation severity enum
export type ViolationSeverity = 'minor' | 'moderate' | 'severe';

// Violation status enum
export type ViolationStatus = 'open' | 'resolved' | 'disputed' | 'escalated' | 'dismissed';

// Evidence type
export interface ViolationEvidence {
  id: string;
  violationId: string;
  fileName: string;
  fileUrl: string;
  fileType: string;
  fileSize: number;
  description?: string;
  uploadedAt: string;
  uploadedBy: string;
}

// Timeline event type
export interface ViolationTimelineEvent {
  id: string;
  violationId: string;
  eventType:
    | 'created'
    | 'updated'
    | 'evidence_added'
    | 'status_changed'
    | 'disputed'
    | 'resolved'
    | 'escalated'
    | 'comment_added';
  description: string;
  previousStatus?: ViolationStatus;
  newStatus?: ViolationStatus;
  createdAt: string;
  createdBy: string;
  createdByName: string;
}

// Lease violation
export interface LeaseViolation {
  id: string;
  organizationId: string;
  leaseId: string;
  unitId: string;
  violationType: ViolationType;
  severity: ViolationSeverity;
  status: ViolationStatus;
  description: string;
  violationDate: string;
  reportedAt: string;
  reportedBy: string;
  resolvedAt?: string;
  resolvedBy?: string;
  resolutionNotes?: string;
  disputedAt?: string;
  disputedBy?: string;
  disputeReason?: string;
  createdAt: string;
  updatedAt: string;
}

// Violation summary for list views
export interface ViolationSummary {
  id: string;
  leaseId: string;
  unitId: string;
  unitNumber: string;
  buildingName: string;
  tenantId: string;
  tenantName: string;
  violationType: ViolationType;
  severity: ViolationSeverity;
  status: ViolationStatus;
  violationDate: string;
  reportedAt: string;
}

// Violation with full details
export interface ViolationWithDetails extends LeaseViolation {
  lease: {
    id: string;
    startDate: string;
    endDate: string;
  };
  unit: {
    id: string;
    number: string;
    buildingId: string;
    buildingName: string;
  };
  tenant: {
    id: string;
    name: string;
    email: string;
    phone?: string;
  };
  reportedByUser: {
    id: string;
    name: string;
  };
  evidence: ViolationEvidence[];
  timeline: ViolationTimelineEvent[];
}

// Form data for creating a violation
export interface CreateViolationData {
  leaseId: string;
  violationType: ViolationType;
  severity: ViolationSeverity;
  description: string;
  violationDate: string;
}

// Form data for resolving a violation
export interface ResolveViolationData {
  resolutionNotes: string;
}

// Form data for disputing a violation
export interface DisputeViolationData {
  disputeReason: string;
}

// Violation statistics
export interface ViolationStatistics {
  totalViolations: number;
  openViolations: number;
  resolvedViolations: number;
  disputedViolations: number;
  violationsByType: Record<ViolationType, number>;
  violationsBySeverity: Record<ViolationSeverity, number>;
}
