/**
 * Predictive Maintenance & Equipment Types (Epic 13, Story 13.3)
 */

export interface Equipment {
  id: string;
  organizationId: string;
  buildingId: string;
  facilityId: string | null;
  name: string;
  category: string;
  manufacturer: string | null;
  model: string | null;
  serialNumber: string | null;
  installationDate: string | null;
  warrantyExpires: string | null;
  expectedLifespanYears: number | null;
  maintenanceIntervalDays: number | null;
  lastMaintenanceDate: string | null;
  nextMaintenanceDue: string | null;
  status: string;
  notes: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface MaintenancePrediction {
  id: string;
  equipmentId: string;
  riskScore: number;
  predictedFailureDate: string | null;
  confidence: number;
  recommendation: string;
  factors: Record<string, unknown>;
  acknowledged: boolean;
  acknowledgedBy: string | null;
  actionTaken: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface EquipmentQuery {
  buildingId?: string;
  facilityId?: string;
  category?: string;
  status?: string;
  needsMaintenance?: boolean;
  limit?: number;
  offset?: number;
}

export interface PredictionsQuery {
  equipmentId?: string;
  limit?: number;
}
