/**
 * DSA Transparency Reports Page (Epic 67, Story 67.3).
 * Epic 90, Story 90.6: Wire up DSA reports handlers to API.
 *
 * Page for generating and viewing DSA transparency reports.
 */

import {
  useDownloadDsaReportPdf,
  useDsaMetrics,
  useDsaReports,
  useGenerateDsaReport,
  usePublishDsaReport,
} from '@ppt/api-client';
import type React from 'react';
import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useToast } from '../../../components';
import type { DsaTransparencyReport } from '../components/DsaTransparencyReportCard';
import { DsaTransparencyReportCard } from '../components/DsaTransparencyReportCard';

interface DsaMetricsDisplay {
  current_period_start: string;
  current_period_end: string;
  moderation_actions_this_period: number;
  pending_cases: number;
  avg_resolution_time_hours: number;
  sla_compliance_rate: number;
}

export const DsaReportsPage: React.FC = () => {
  const { t } = useTranslation();
  const { showToast } = useToast();

  // Report generation form
  const [showGenerateForm, setShowGenerateForm] = useState(false);
  const [periodStart, setPeriodStart] = useState('');
  const [periodEnd, setPeriodEnd] = useState('');
  const [formError, setFormError] = useState<string | null>(null);

  // Fetch reports from API
  const { data: reportsData, isLoading: reportsLoading, error: reportsError } = useDsaReports();

  // Fetch metrics from API
  const { data: metricsData } = useDsaMetrics();

  // Mutations
  const generateReport = useGenerateDsaReport();
  const publishReport = usePublishDsaReport();
  const downloadPdf = useDownloadDsaReportPdf();

  // Transform API data to component types
  const reports: DsaTransparencyReport[] = (reportsData?.reports ?? []).map((r) => ({
    id: r.id,
    period_start: r.period_start,
    period_end: r.period_end,
    status: r.status,
    summary: r.summary,
    content_type_breakdown: r.content_type_breakdown.map((c) => ({
      content_type: c.type,
      count: c.count,
    })),
    violation_type_breakdown: r.violation_type_breakdown.map((v) => ({
      violation_type: v.type,
      count: v.count,
    })),
    download_url: r.download_url,
    generated_at: r.generated_at,
    published_at: r.published_at,
  }));

  const metrics: DsaMetricsDisplay | null = metricsData?.metrics
    ? {
        current_period_start: metricsData.metrics.current_period_start,
        current_period_end: metricsData.metrics.current_period_end,
        moderation_actions_this_period: metricsData.metrics.moderation_actions_this_period,
        pending_cases: metricsData.metrics.pending_cases,
        avg_resolution_time_hours: metricsData.metrics.avg_resolution_time_hours,
        sla_compliance_rate: metricsData.metrics.sla_compliance_rate,
      }
    : null;

  const handleGenerateReport = useCallback(() => {
    if (!periodStart || !periodEnd) {
      setFormError(t('dsa.generate.dateError'));
      return;
    }

    setFormError(null);
    generateReport.mutate(
      {
        period_start: periodStart,
        period_end: periodEnd,
      },
      {
        onSuccess: () => {
          setShowGenerateForm(false);
          setPeriodStart('');
          setPeriodEnd('');
        },
        onError: (err) => {
          console.error('Failed to generate report:', err);
          setFormError(t('dsa.generate.error'));
        },
      }
    );
  }, [periodStart, periodEnd, generateReport, t]);

  const handlePublish = useCallback(
    (reportId: string) => {
      publishReport.mutate(reportId, {
        onSuccess: () => {
          showToast({
            type: 'success',
            title: t('dsa.publish.successTitle'),
            message: t('dsa.publish.successMessage'),
          });
        },
        onError: (err) => {
          console.error('Failed to publish report:', err);
          showToast({
            type: 'error',
            title: t('dsa.publish.errorTitle'),
            message: t('dsa.publish.errorMessage'),
          });
        },
      });
    },
    [publishReport, showToast, t]
  );

  const handleDownload = useCallback(
    (reportId: string) => {
      downloadPdf.mutate(reportId, {
        onError: (err) => {
          console.error('Failed to download report:', err);
          showToast({
            type: 'error',
            title: t('dsa.download.errorTitle'),
            message: t('dsa.download.errorMessage'),
          });
        },
      });
    },
    [downloadPdf, showToast, t]
  );

  const formatDate = (dateStr: string): string => {
    return new Date(dateStr).toLocaleDateString('en-GB', {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
    });
  };

  // Loading state
  if (reportsLoading) {
    return (
      <div className="dsa-reports-page">
        <div className="dsa-reports-header">
          <h1>{t('dsa.title')}</h1>
          <p>{t('dsa.subtitle')}</p>
        </div>
        <div className="dsa-loading">{t('dsa.loading')}</div>
      </div>
    );
  }

  // Error state
  if (reportsError) {
    return (
      <div className="dsa-reports-page">
        <div className="dsa-reports-header">
          <h1>{t('dsa.title')}</h1>
          <p>{t('dsa.subtitle')}</p>
        </div>
        <div className="dsa-reports-error" role="alert">
          {t('dsa.loadError', { message: reportsError.message })}
          <button
            type="button"
            onClick={() => window.location.reload()}
            className="dsa-retry-button"
          >
            {t('dsa.retry')}
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="dsa-reports-page">
      <div className="dsa-reports-header">
        <h1>{t('dsa.title')}</h1>
        <p>{t('dsa.subtitle')}</p>
      </div>

      {/* Current Metrics */}
      {metrics && (
        <div className="dsa-current-metrics">
          <h2>{t('dsa.metrics.title')}</h2>
          <p className="dsa-period-info">
            {t('dsa.metrics.period', {
              start: formatDate(metrics.current_period_start),
              end: formatDate(metrics.current_period_end),
            })}
          </p>
          <div className="dsa-metrics-grid">
            <div className="dsa-metric-card">
              <div className="dsa-metric-value">{metrics.moderation_actions_this_period}</div>
              <div className="dsa-metric-label">{t('dsa.metrics.moderationActions')}</div>
            </div>
            <div className="dsa-metric-card">
              <div className="dsa-metric-value">{metrics.pending_cases}</div>
              <div className="dsa-metric-label">{t('dsa.metrics.pendingCases')}</div>
            </div>
            <div className="dsa-metric-card">
              <div className="dsa-metric-value">
                {metrics.avg_resolution_time_hours.toFixed(1)}h
              </div>
              <div className="dsa-metric-label">{t('dsa.metrics.avgResolutionTime')}</div>
            </div>
            <div className="dsa-metric-card">
              <div className="dsa-metric-value">{metrics.sla_compliance_rate.toFixed(1)}%</div>
              <div className="dsa-metric-label">{t('dsa.metrics.slaCompliance')}</div>
            </div>
          </div>
        </div>
      )}

      {/* Generate Report */}
      <div className="dsa-generate-section">
        {!showGenerateForm ? (
          <button
            type="button"
            className="dsa-generate-button"
            onClick={() => setShowGenerateForm(true)}
          >
            {t('dsa.generate.newButton')}
          </button>
        ) : (
          <div className="dsa-generate-form">
            <h3>{t('dsa.generate.formTitle')}</h3>
            {formError && (
              <div className="dsa-form-error" role="alert">
                {formError}
              </div>
            )}
            <div className="dsa-form-row">
              <div className="dsa-form-field">
                <label htmlFor="periodStart">{t('dsa.generate.periodStart')}</label>
                <input
                  type="date"
                  id="periodStart"
                  value={periodStart}
                  onChange={(e) => setPeriodStart(e.target.value)}
                />
              </div>
              <div className="dsa-form-field">
                <label htmlFor="periodEnd">{t('dsa.generate.periodEnd')}</label>
                <input
                  type="date"
                  id="periodEnd"
                  value={periodEnd}
                  onChange={(e) => setPeriodEnd(e.target.value)}
                />
              </div>
            </div>
            <div className="dsa-form-actions">
              <button
                type="button"
                className="dsa-form-cancel"
                onClick={() => setShowGenerateForm(false)}
                disabled={generateReport.isPending}
              >
                {t('common.cancel')}
              </button>
              <button
                type="button"
                className="dsa-form-submit"
                onClick={handleGenerateReport}
                disabled={generateReport.isPending}
              >
                {generateReport.isPending ? t('dsa.generate.submitting') : t('dsa.generate.submit')}
              </button>
            </div>
          </div>
        )}
      </div>

      {/* Reports List */}
      <div className="dsa-reports-list-section">
        <h2>{t('dsa.list.title')}</h2>
        {reports.length > 0 ? (
          <div className="dsa-reports-list">
            {reports.map((report) => (
              <DsaTransparencyReportCard
                key={report.id}
                report={report}
                onPublish={handlePublish}
                onDownload={handleDownload}
                showActions={true}
              />
            ))}
          </div>
        ) : (
          <div className="dsa-empty-state">
            <p>{t('dsa.list.emptyTitle')}</p>
            <p>{t('dsa.list.emptySubtitle')}</p>
          </div>
        )}
      </div>

      {/* DSA Requirements Info */}
      <div className="dsa-requirements-info">
        <h2>{t('dsa.requirements.title')}</h2>
        <ul>
          <li>
            <strong>{t('dsa.requirements.moderationLabel')}</strong>{' '}
            {t('dsa.requirements.moderationText')}
          </li>
          <li>
            <strong>{t('dsa.requirements.userReportsLabel')}</strong>{' '}
            {t('dsa.requirements.userReportsText')}
          </li>
          <li>
            <strong>{t('dsa.requirements.automatedLabel')}</strong>{' '}
            {t('dsa.requirements.automatedText')}
          </li>
          <li>
            <strong>{t('dsa.requirements.appealsLabel')}</strong>{' '}
            {t('dsa.requirements.appealsText')}
          </li>
          <li>
            <strong>{t('dsa.requirements.timelinessLabel')}</strong>{' '}
            {t('dsa.requirements.timelinessText')}
          </li>
        </ul>
      </div>
    </div>
  );
};

DsaReportsPage.displayName = 'DsaReportsPage';
