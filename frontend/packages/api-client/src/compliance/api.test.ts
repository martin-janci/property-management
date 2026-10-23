/**
 * Contract test for the compliance/AML-DSA client's base paths (#3149).
 *
 * The AML/DSA handlers are mounted on the api-server at `/api/v1/aml-dsa`
 * (`backend/servers/api-server/src/lib.rs` -> `routes::aml_dsa::router()`),
 * while the content-moderation handlers live under `/api/v1/compliance`
 * (`routes::compliance::router()`). The hand-maintained client originally
 * sent every call — moderation AND aml/dsa — to `/api/v1/compliance`, so the
 * whole `aml/*` + `dsa/*` surface resolved to `/api/v1/compliance/aml/...`
 * and 404'd. These assertions pin each function to the router that actually
 * mounts it so the split can't silently regress.
 */

import { beforeEach, describe, expect, it, vi } from 'vitest';

import { assignModerationCase, generateDsaReport } from './api';

const AML_DSA_BASE = '/api/v1/aml-dsa';
const COMPLIANCE_BASE = '/api/v1/compliance';

let fetchMock: ReturnType<typeof vi.fn>;

beforeEach(() => {
  vi.clearAllMocks();
  fetchMock = vi.fn().mockResolvedValue({ ok: true, json: async () => ({}) });
  vi.stubGlobal('fetch', fetchMock);
});

function calledUrl(): string {
  return String(fetchMock.mock.calls[0][0]);
}

describe('compliance client base paths (#3149)', () => {
  it('AML calls target the /api/v1/aml-dsa router', async () => {
    const api = await import('./api');
    await api.listAmlAssessments();
    expect(calledUrl()).toContain(`${AML_DSA_BASE}/aml/assessments`);
    fetchMock.mockClear();

    await api.getAmlThresholds();
    expect(calledUrl()).toContain(`${AML_DSA_BASE}/aml/thresholds`);
    fetchMock.mockClear();

    await api.getCountryRisks();
    expect(calledUrl()).toContain(`${AML_DSA_BASE}/aml/country-risks`);
    fetchMock.mockClear();

    await api.reviewAmlAssessment('a-1', { decision: 'approve', notes: 'n' });
    expect(calledUrl()).toContain(`${AML_DSA_BASE}/aml/assessments/a-1/review`);
  });

  it('DSA calls target the /api/v1/aml-dsa router', async () => {
    const api = await import('./api');
    await api.listDsaReports();
    expect(calledUrl()).toContain(`${AML_DSA_BASE}/dsa/reports`);
    fetchMock.mockClear();

    await api.getDsaMetrics();
    expect(calledUrl()).toContain(`${AML_DSA_BASE}/dsa/metrics`);
    fetchMock.mockClear();

    await generateDsaReport({ report_type: 'quarterly' } as never);
    expect(calledUrl()).toContain(`${AML_DSA_BASE}/dsa/reports/generate`);
    fetchMock.mockClear();

    await api.publishDsaReport('r-1');
    expect(calledUrl()).toContain(`${AML_DSA_BASE}/dsa/reports/r-1/publish`);
    fetchMock.mockClear();

    await api.downloadDsaReportPdf('r-1');
    expect(calledUrl()).toContain(`${AML_DSA_BASE}/dsa/reports/r-1/download`);
  });

  it('moderation calls stay on the /api/v1/compliance router', async () => {
    await assignModerationCase('c-1');
    expect(calledUrl()).toContain(`${COMPLIANCE_BASE}/moderation/cases/c-1/assign`);
    // and must NOT leak onto the aml-dsa base
    expect(calledUrl()).not.toContain(AML_DSA_BASE);
  });
});
