/// <reference types="vitest/globals" />
/**
 * Integration test for the report execution-history download + retry path
 * (Story 81.2 — Coverage 81-2).
 *
 * This renders the presentational `ScheduleDetailPage` wired through the *real*
 * api-client hooks (`useDownloadReport`, `useRetryReportExecution`) inside a
 * real `QueryClientProvider`, mocking only the network boundary (`fetch`). It
 * confirms the presigned-download + retry flow end-to-end on the surface a
 * manager actually sees:
 *
 *   - Download: GET /api/v1/reports/executions/:id/download returns a presigned
 *     `{ url, fileName }`, and the hook triggers a real anchor click whose
 *     `href`/`download` match the presigned response.
 *   - Retry: POST /api/v1/reports/executions/:id/retry is fired for a failed
 *     execution, the in-row button shows the "Retrying…" state while the
 *     mutation is in flight, and the execution-history query is invalidated on
 *     success.
 *   - Download failure surfaces an error toast (via the route container's
 *     onError wiring is covered separately; here we pin the hook rejecting so
 *     the awaited promise path is exercised without an unhandled rejection).
 */

import type { ReportExecution, ReportSchedule } from '@ppt/api-client';
import { reportKeys, useDownloadReport, useRetryReportExecution } from '@ppt/api-client';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { type ReactNode, useCallback, useState } from 'react';
import { ScheduleDetailPage } from './ScheduleDetailPage';

// ── Fixtures ──

const SCHEDULE_ID = 'sched-1';

const schedule: ReportSchedule = {
  id: SCHEDULE_ID,
  report_id: 'rep-1',
  organization_id: 'org-1',
  name: 'Monthly Building Report',
  frequency: 'monthly',
  time: '08:00',
  timezone: 'UTC',
  format: 'pdf',
  recipients: [],
  is_active: true,
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-01T00:00:00Z',
};

const completedExecution: ReportExecution = {
  id: 'exec-completed',
  scheduleId: SCHEDULE_ID,
  status: 'completed',
  startedAt: '2026-06-01T08:00:00Z',
  completedAt: '2026-06-01T08:00:05Z',
  fileName: 'building-report-2026-06.pdf',
  fileSize: 2_500_000,
  createdAt: '2026-06-01T08:00:00Z',
};

const failedExecution: ReportExecution = {
  id: 'exec-failed',
  scheduleId: SCHEDULE_ID,
  status: 'failed',
  startedAt: '2026-06-02T08:00:00Z',
  completedAt: '2026-06-02T08:00:02Z',
  error: { code: 'GEN_ERROR', message: 'Generation failed' },
  createdAt: '2026-06-02T08:00:00Z',
};

const PRESIGNED_URL = 'https://s3.example.com/reports/building-report-2026-06.pdf?sig=abc';

// ── Helpers ──

function okJson(body: unknown): Response {
  return { ok: true, status: 200, json: async () => body } as Response;
}

function errJson(status: number, body: unknown): Response {
  return { ok: false, status, json: async () => body } as Response;
}

function newClient() {
  return new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
}

function Wrapper({ client, children }: { client: QueryClient; children: ReactNode }) {
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}

/**
 * Container that wires the *real* download/retry hooks into the presentational
 * page exactly like the route does (routes/groups/reports.tsx), minus the
 * Toast/Router shell that isn't relevant to the network path.
 */
function Harness({ executions }: { executions: ReportExecution[] }) {
  const downloadReport = useDownloadReport();
  const retryExecution = useRetryReportExecution();

  const handleDownload = useCallback(
    (executionId: string) => {
      downloadReport.mutate(executionId);
    },
    [downloadReport]
  );

  const [retryError, setRetryError] = useState(false);
  const handleRetry = useCallback(
    async (executionId: string) => {
      // Mirrors the route container's onError handling. We surface the failure
      // via state rather than re-throwing: the page's own `handleRetry` awaits
      // this callback inside a `try/finally` with no catch, so re-throwing here
      // would escape as an unhandled rejection in the test environment (and the
      // surface only needs the error reflected back to the user, which the toast
      // / state does).
      try {
        await retryExecution.mutateAsync(executionId);
      } catch {
        setRetryError(true);
      }
    },
    [retryExecution]
  );

  return (
    <>
      {retryError && <div data-testid="retry-error" />}
      <ScheduleDetailPage
        schedule={schedule}
        executions={executions}
        onDownload={handleDownload}
        onRetry={handleRetry}
      />
    </>
  );
}

describe('ScheduleDetailPage — presigned download + retry path (81.2)', () => {
  let clickSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    // Prevent jsdom "Not implemented: navigation" noise from the real anchor click.
    clickSpy = vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(() => {});
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('downloads via the presigned URL: fetches /download then clicks an anchor with the presigned href + fileName', async () => {
    const fetchMock = vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      okJson({
        url: PRESIGNED_URL,
        fileName: completedExecution.fileName,
        expiresAt: '2026-06-01T09:00:00Z',
        contentType: 'application/pdf',
      })
    );

    render(
      <Wrapper client={newClient()}>
        <Harness executions={[completedExecution]} />
      </Wrapper>
    );

    fireEvent.click(screen.getByRole('button', { name: /download report/i }));

    // The presigned-URL endpoint is hit (GET by default).
    await waitFor(() => expect(fetchMock).toHaveBeenCalled());
    const [calledUrl, calledOpts] = fetchMock.mock.calls[0];
    expect(String(calledUrl)).toBe(`/api/v1/reports/executions/${completedExecution.id}/download`);
    expect((calledOpts as RequestInit | undefined)?.method ?? 'GET').toBe('GET');

    // The anchor is created with the presigned href + the server-provided filename
    // and is actually clicked (download triggered).
    await waitFor(() => expect(clickSpy).toHaveBeenCalledTimes(1));
    const anchor = clickSpy.mock.instances[0] as HTMLAnchorElement;
    expect(anchor.href).toBe(PRESIGNED_URL);
    expect(anchor.download).toBe(completedExecution.fileName);

    // Anchor is cleaned up out of the DOM after the click.
    expect(document.querySelector(`a[download="${completedExecution.fileName}"]`)).toBeNull();
  });

  it('does not trigger an anchor click (and leaves no leaked node) when the presigned URL is empty', async () => {
    // Backend hands back a 200 with a blank `url` (e.g. storage key not yet
    // materialised). The hook must treat this as a failure rather than clicking
    // an anchor with href="" — which would navigate to the current page.
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      okJson({
        url: '',
        fileName: completedExecution.fileName,
        expiresAt: '2026-06-01T09:00:00Z',
        contentType: 'application/pdf',
      })
    );

    render(
      <Wrapper client={newClient()}>
        <Harness executions={[completedExecution]} />
      </Wrapper>
    );

    fireEvent.click(screen.getByRole('button', { name: /download report/i }));

    await waitFor(() => expect(globalThis.fetch).toHaveBeenCalled());
    expect(clickSpy).not.toHaveBeenCalled();
    // No detached anchor leaked into the DOM.
    expect(document.querySelector(`a[download="${completedExecution.fileName}"]`)).toBeNull();
  });

  it('cleans up the temporary anchor even when the click() throws', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      okJson({
        url: PRESIGNED_URL,
        fileName: completedExecution.fileName,
        expiresAt: '2026-06-01T09:00:00Z',
        contentType: 'application/pdf',
      })
    );
    // Simulate a browser/jsdom click that throws — the hook's try/finally must
    // still remove the appended anchor from <body>.
    clickSpy.mockImplementation(() => {
      throw new Error('click boom');
    });

    render(
      <Wrapper client={newClient()}>
        <Harness executions={[completedExecution]} />
      </Wrapper>
    );

    fireEvent.click(screen.getByRole('button', { name: /download report/i }));

    await waitFor(() => expect(clickSpy).toHaveBeenCalledTimes(1));
    await waitFor(() =>
      expect(document.querySelector(`a[download="${completedExecution.fileName}"]`)).toBeNull()
    );
  });

  it('does not trigger an anchor click when the presigned-URL request fails', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(errJson(403, { message: 'Forbidden' }));

    render(
      <Wrapper client={newClient()}>
        <Harness executions={[completedExecution]} />
      </Wrapper>
    );

    fireEvent.click(screen.getByRole('button', { name: /download report/i }));

    // Give the rejected mutation a tick to settle.
    await waitFor(() => {
      // fetch was attempted but no download anchor was clicked.
      expect(globalThis.fetch).toHaveBeenCalled();
    });
    expect(clickSpy).not.toHaveBeenCalled();
  });

  it('retries a failed execution: POSTs /retry, shows the in-flight state, and invalidates the history query', async () => {
    const client = newClient();
    const invalidateSpy = vi.spyOn(client, 'invalidateQueries');

    let resolveRetry: (value: ReportExecution) => void = () => {};
    const retryResponse = new Promise<Response>((resolve) => {
      resolveRetry = (value) => resolve(okJson(value));
    });
    const fetchMock = vi.spyOn(globalThis, 'fetch').mockReturnValue(retryResponse);

    render(
      <Wrapper client={client}>
        <Harness executions={[failedExecution]} />
      </Wrapper>
    );

    const retryBtn = screen.getByRole('button', { name: /retry execution/i });
    fireEvent.click(retryBtn);

    // In-flight: button disabled + "Retrying…" copy while the POST is pending.
    await waitFor(() => expect(retryBtn).toBeDisabled());
    expect(screen.getByText(/retrying/i)).toBeInTheDocument();

    // The retry endpoint is hit with POST.
    const [calledUrl, calledOpts] = fetchMock.mock.calls[0];
    expect(String(calledUrl)).toBe(`/api/v1/reports/executions/${failedExecution.id}/retry`);
    expect((calledOpts as RequestInit | undefined)?.method).toBe('POST');

    // Resolve with a freshly-queued execution; the hook must invalidate the
    // execution-history query for this schedule.
    resolveRetry({
      id: 'exec-retried',
      scheduleId: SCHEDULE_ID,
      status: 'pending',
      startedAt: '2026-06-03T08:00:00Z',
      createdAt: '2026-06-03T08:00:00Z',
    });

    await waitFor(() =>
      expect(invalidateSpy).toHaveBeenCalledWith(
        expect.objectContaining({
          queryKey: reportKeys.executions(SCHEDULE_ID),
        })
      )
    );

    // Button returns to its idle, enabled state once the retry settles.
    await waitFor(() => expect(retryBtn).not.toBeDisabled());
    expect(screen.queryByTestId('retry-error')).toBeNull();
  });

  it('surfaces a retry failure to the caller (rejected mutateAsync) without crashing the surface', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(errJson(500, { message: 'boom' }));

    render(
      <Wrapper client={newClient()}>
        <Harness executions={[failedExecution]} />
      </Wrapper>
    );

    fireEvent.click(screen.getByRole('button', { name: /retry execution/i }));

    await waitFor(() => expect(screen.getByTestId('retry-error')).toBeInTheDocument());
    // The failed row is still rendered (surface didn't unmount on the rejection).
    expect(screen.getByRole('button', { name: /retry execution/i })).toBeInTheDocument();
  });
});
