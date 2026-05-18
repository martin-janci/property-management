/**
 * Outages Types
 *
 * TypeScript types for Outages API (UC-12).
 */

// ============================================
// Core Types
// ============================================

export type OutageCommodity = 'electricity' | 'water' | 'gas' | 'heating' | 'internet' | 'other';

export type OutageSeverity = 'critical' | 'major' | 'minor' | 'informational';

export type OutageStatus = 'planned' | 'ongoing' | 'resolved' | 'cancelled';

export interface Outage {
  id: string;
  organizationId: string;
  title: string;
  description?: string;
  commodity: OutageCommodity;
  severity: OutageSeverity;
  status: OutageStatus;
  buildingIds: string[];
  scheduledStart: string;
  scheduledEnd?: string;
  actualStart?: string;
  actualEnd?: string;
  externalReference?: string;
  supplierName?: string;
  resolutionNotes?: string;
  cancelReason?: string;
  createdBy: string;
  createdAt: string;
  updatedAt: string;
}

export interface OutageSummary {
  id: string;
  title: string;
  commodity: OutageCommodity;
  severity: OutageSeverity;
  status: OutageStatus;
  scheduledStart: string;
  scheduledEnd?: string;
  actualStart?: string;
  actualEnd?: string;
  affectedBuildingsCount: number;
  createdAt: string;
}

export interface OutageWithDetails extends Outage {
  buildings: OutageBuilding[];
  readStatus?: OutageReadStatus;
  /** Resolved display name of the user who created the outage (joined server-side). */
  creatorName?: string;
}

export interface OutageBuilding {
  id: string;
  name: string;
  address: string;
}

export interface OutageReadStatus {
  readAt?: string;
  isRead: boolean;
}

// ============================================
// Query Types
// ============================================

export interface OutageListQuery {
  status?: OutageStatus | OutageStatus[];
  commodity?: OutageCommodity | OutageCommodity[];
  severity?: OutageSeverity | OutageSeverity[];
  buildingId?: string;
  fromDate?: string;
  toDate?: string;
  activeOnly?: boolean;
  limit?: number;
  offset?: number;
}

export interface PaginatedOutages {
  outages: OutageSummary[];
  count: number;
  total: number;
}

// ============================================
// Create & Update Types
// ============================================

export interface CreateOutageRequest {
  title: string;
  description?: string;
  commodity: OutageCommodity;
  severity: OutageSeverity;
  buildingIds?: string[];
  scheduledStart: string;
  scheduledEnd?: string;
  externalReference?: string;
  supplierName?: string;
}

export interface CreateOutageResponse {
  id: string;
  message: string;
}

export interface UpdateOutageRequest {
  title?: string;
  description?: string;
  commodity?: OutageCommodity;
  severity?: OutageSeverity;
  buildingIds?: string[];
  scheduledStart?: string;
  scheduledEnd?: string;
  externalReference?: string;
  supplierName?: string;
}

export interface OutageActionResponse {
  message: string;
  outage: Outage;
}

// ============================================
// Status Change Types
// ============================================

export interface StartOutageRequest {
  actualStart?: string;
}

export interface ResolveOutageRequest {
  actualEnd?: string;
  resolutionNotes?: string;
}

export interface CancelOutageRequest {
  reason?: string;
}

// ============================================
// Statistics & Dashboard Types
// ============================================

export interface OutageStatistics {
  totalOutages: number;
  activeOutages: number;
  plannedOutages: number;
  resolvedOutages: number;
  cancelledOutages: number;
  outagesByCommodity: Record<OutageCommodity, number>;
  outagesBySeverity: Record<OutageSeverity, number>;
  averageResolutionHours?: number;
}

export interface OutageDashboard {
  currentOutages: OutageSummary[];
  upcomingOutages: OutageSummary[];
  recentlyResolved: OutageSummary[];
  statistics: OutageStatistics;
}

export interface UnreadOutagesCount {
  unreadCount: number;
}
