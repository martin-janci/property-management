/**
 * Sentiment Analysis Feature Types (Epic 13, Story 13.2)
 */

export interface SentimentTrend {
  id: string;
  organizationId: string;
  buildingId: string | null;
  date: string;
  avgSentiment: number;
  messageCount: number;
  negativeCount: number;
  neutralCount: number;
  positiveCount: number;
  createdAt: string;
  updatedAt: string;
}

export interface SentimentAlert {
  id: string;
  organizationId: string;
  buildingId: string | null;
  alertType: string;
  thresholdBreached: number;
  currentSentiment: number;
  previousSentiment: number | null;
  sampleMessageIds: string[];
  acknowledged: boolean;
  acknowledgedBy: string | null;
  acknowledgedAt: string | null;
  createdAt: string;
}

export interface SentimentThresholds {
  id: string;
  organizationId: string;
  negativeThreshold: number;
  alertOnSpike: boolean;
  spikeThresholdChange: number;
  minMessagesForAlert: number;
  enabled: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface SentimentDashboard {
  organization_avg: number;
  trends: SentimentTrend[];
  recent_alerts: SentimentAlert[];
}

export interface UpdateSentimentThresholdsRequest {
  negativeThreshold?: number;
  alertOnSpike?: boolean;
  spikeThresholdChange?: number;
  minMessagesForAlert?: number;
  enabled?: boolean;
}

export interface SentimentTrendQuery {
  buildingId?: string;
  fromDate?: string;
  toDate?: string;
  limit?: number;
}
