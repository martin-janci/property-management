/// <reference types="vitest/globals" />
/**
 * DsaReportsPage publish / download feedback tests (#2854).
 *
 * History: after the i18n pass (#2847) moved every literal onto `dsa.*` keys,
 * the publish and download flows still surfaced their feedback through a
 * blocking, unstyled native `window.alert(...)` — the odd one out in the
 * compliance feature after the sibling `ContentModerationPage` (#2841) and
 * `AmlDashboardPage` (#2829) were migrated to the in-app Toast system.
 *
 * These tests lock in the migration: the publish/download handlers must drive
 * feedback through the Toast system (localized `dsa.publish.*` / `dsa.download.*`
 * title+message keys) and must NEVER call `window.alert`, mirroring the
 * "never uses window.prompt or window.alert" assertion added for
 * `ContentModerationPage` in #2841.
 */
import { useDownloadDsaReportPdf, useDsaMetrics, usePublishDsaReport } from '@ppt/api-client';
import { fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ToastProvider } from '../../../components';
import { DsaReportsPage } from './DsaReportsPage';

const publishMutate = vi.fn();
const downloadMutate = vi.fn();

// One "generated" report with a download_url so the card renders BOTH the
// "Publish Report" action (status === 'generated') and the "Download PDF"
// action (download_url present).
const generatedReport = {
  id: 'report-1',
  period_start: '2026-01-01',
  period_end: '2026-03-31',
  status: 'generated',
  summary: {
    total_actions: 10,
    total_reports: 5,
    total_appeals: 2,
    appeals_upheld: 1,
    appeals_overturned: 1,
    automated_decisions: 3,
    human_reviews: 7,
  },
  content_type_breakdown: [{ type: 'listing', count: 4 }],
  violation_type_breakdown: [{ type: 'spam', count: 6 }],
  download_url: 'https://example.test/report-1.pdf',
  generated_at: '2026-04-01T00:00:00Z',
  published_at: null,
};

vi.mock('@ppt/api-client', () => ({
  useDsaReports: vi.fn(() => ({
    data: { reports: [generatedReport] },
    isLoading: false,
    error: null,
  })),
  useDsaMetrics: vi.fn(() => ({ data: undefined })),
  useGenerateDsaReport: vi.fn(() => ({ mutate: vi.fn(), isPending: false })),
  usePublishDsaReport: vi.fn(() => ({ mutate: publishMutate, isPending: false })),
  useDownloadDsaReportPdf: vi.fn(() => ({ mutate: downloadMutate, isPending: false })),
}));

function renderPage() {
  return render(
    <ToastProvider>
      <DsaReportsPage />
    </ToastProvider>
  );
}

function clickPublish() {
  fireEvent.click(screen.getByRole('button', { name: /publish report/i }));
}

function clickDownload() {
  fireEvent.click(screen.getByRole('button', { name: /download pdf/i }));
}

describe('DsaReportsPage feedback', () => {
  const alertSpy = vi.spyOn(window, 'alert').mockImplementation(() => {});

  beforeEach(() => {
    publishMutate.mockReset();
    downloadMutate.mockReset();
    alertSpy.mockClear();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('shows a success toast (not window.alert) when a report is published', () => {
    // Simulate the mutation resolving successfully.
    publishMutate.mockImplementation((_id: string, opts?: { onSuccess?: () => void }) => {
      opts?.onSuccess?.();
    });

    renderPage();
    clickPublish();

    expect(publishMutate).toHaveBeenCalledWith('report-1', expect.any(Object));
    expect(alertSpy).not.toHaveBeenCalled();
    // Localized success toast copy (dsa.publish.successTitle / successMessage).
    expect(screen.getByText('Report published')).toBeInTheDocument();
    expect(screen.getByText('Report published successfully.')).toBeInTheDocument();
  });

  it('shows an error toast (not window.alert) when publishing fails', () => {
    publishMutate.mockImplementation((_id: string, opts?: { onError?: (err: Error) => void }) => {
      opts?.onError?.(new Error('boom'));
    });

    renderPage();
    clickPublish();

    expect(alertSpy).not.toHaveBeenCalled();
    expect(screen.getByText('Failed to publish report')).toBeInTheDocument();
    expect(screen.getByText('Failed to publish report. Please try again.')).toBeInTheDocument();
  });

  it('shows an error toast (not window.alert) when a download fails', () => {
    downloadMutate.mockImplementation((_id: string, opts?: { onError?: (err: Error) => void }) => {
      opts?.onError?.(new Error('boom'));
    });

    renderPage();
    clickDownload();

    expect(downloadMutate).toHaveBeenCalledWith('report-1', expect.any(Object));
    expect(alertSpy).not.toHaveBeenCalled();
    expect(screen.getByText('Failed to download report')).toBeInTheDocument();
    expect(screen.getByText('Failed to download report. Please try again.')).toBeInTheDocument();
  });
});
