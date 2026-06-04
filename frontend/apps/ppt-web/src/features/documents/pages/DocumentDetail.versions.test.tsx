/// <reference types="vitest/globals" />
/**
 * Regression test for PR #990 (frontend gap-sweep) — Story 7B.1 document
 * version-history timeline.
 *
 * PR #990 shipped 34 files without a regression test. This pins the single
 * highest-value, clearly-contractual behavior change from that sweep: the
 * version-row normalization in DocumentDetail. The component:
 *
 *   1. tolerates both camelCase and snake_case field names coming off the
 *      (generated) list-versions response, plus a nested `file.sizeBytes`;
 *   2. sorts the timeline newest-version-first regardless of input order;
 *   3. flags the single highest version number as the current version
 *      ("Current" badge);
 *   4. renders the empty / loading / error states.
 *
 * The version logic lives inline in the component (not exported), so this is a
 * render test that mocks only the network boundary (`@ppt/api-client` hooks)
 * and the leaf presentational children.
 */

import { render, screen, within } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

// ─── network boundary: @ppt/api-client hooks ────────────────────────────────
const useDocumentMock = vi.fn();
const useDocumentClassificationMock = vi.fn();
const useDocumentVersionsMock = vi.fn();
const useReprocessOcrMock = vi.fn();

vi.mock('@ppt/api-client', () => ({
  useDocument: (id: string) => useDocumentMock(id),
  useDocumentClassification: (id: string) => useDocumentClassificationMock(id),
  useDocumentVersions: (id: string) => useDocumentVersionsMock(id),
  useReprocessOcr: () => useReprocessOcrMock(),
}));

// ─── leaf presentational children (irrelevant to the version logic) ──────────
vi.mock('../components/ClassificationBadge', () => ({
  ClassificationUI: () => null,
}));
vi.mock('../components/DocumentSharePanel', () => ({
  DocumentSharePanel: () => null,
}));
vi.mock('../components/DocumentSummary', () => ({
  DocumentSummary: () => null,
}));
vi.mock('../components/OcrStatusBadge', () => ({
  OcrProcessingStatus: () => null,
}));

// Minimal i18next-compatible `t`: honours `defaultValue` and the second-arg
// fallback string, and interpolates `{{name}}` placeholders — matching the
// real react-i18next fallback path the component relies on.
function fakeT(
  _key: string,
  defaultOrOpts?: string | Record<string, unknown>,
  maybeOpts?: Record<string, unknown>
): string {
  let template: string;
  let opts: Record<string, unknown> | undefined;
  if (typeof defaultOrOpts === 'string') {
    template = defaultOrOpts;
    opts = maybeOpts;
  } else {
    opts = defaultOrOpts;
    template = (opts?.defaultValue as string) ?? _key;
  }
  return template.replace(/\{\{(\w+)\}\}/g, (_m, name: string) => String(opts?.[name] ?? ''));
}

vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: fakeT }),
}));

import { DocumentDetail } from './DocumentDetail';

// ─── helpers ─────────────────────────────────────────────────────────────────

/** A loaded document so the component renders past its loading/error gates. */
function loadedDocument() {
  return {
    data: {
      document: {
        title: 'Faktúra.pdf',
        category: 'invoice',
        file_name: 'faktura.pdf',
        size_bytes: 2048,
        created_at: '2025-01-01T00:00:00Z',
        updated_at: '2025-01-02T00:00:00Z',
      },
    },
    isLoading: false,
    error: null,
    refetch: vi.fn(),
  };
}

function setVersions(value: { data?: unknown; isLoading?: boolean; error?: unknown }) {
  useDocumentVersionsMock.mockReturnValue({
    data: value.data,
    isLoading: value.isLoading ?? false,
    error: value.error ?? null,
  });
}

beforeEach(() => {
  useDocumentMock.mockReturnValue(loadedDocument());
  useDocumentClassificationMock.mockReturnValue({ data: undefined, isLoading: false, error: null });
  useReprocessOcrMock.mockReturnValue({ mutate: vi.fn(), isPending: false });
  setVersions({ data: [] });
});

afterEach(() => {
  vi.clearAllMocks();
});

// ─── tests ───────────────────────────────────────────────────────────────────

describe('DocumentDetail version history (PR #990)', () => {
  it('sorts versions newest-first and flags the highest version as current', () => {
    // Deliberately out of order, mixed camelCase/snake_case field names.
    setVersions({
      data: [
        { id: 'v1', version: 1, created_at: '2025-01-01T00:00:00Z' },
        { id: 'v3', versionNumber: 3, createdAt: '2025-03-01T00:00:00Z' },
        { id: 'v2', version_number: 2, created_at: '2025-02-01T00:00:00Z' },
      ],
    });

    render(<DocumentDetail documentId="doc-1" />);

    const rows = screen.getAllByText(/^Version \d+$/).map((el) => el.textContent);
    // Newest version first regardless of input order.
    expect(rows).toEqual(['Version 3', 'Version 2', 'Version 1']);

    // Exactly one "current" badge, attached to the highest version number.
    const badges = screen.getAllByText('Current');
    expect(badges).toHaveLength(1);

    const currentRow = badges[0].closest('li');
    expect(currentRow).not.toBeNull();
    expect(within(currentRow as HTMLElement).getByText('Version 3')).toBeInTheDocument();
    expect(currentRow?.className).toContain('version-row--current');
  });

  it('reads size from a nested file object and tolerates snake_case', () => {
    setVersions({
      data: [
        {
          id: 'v1',
          version: 1,
          created_at: '2025-01-01T00:00:00Z',
          file: { size_bytes: 1536 },
        },
      ],
    });

    render(<DocumentDetail documentId="doc-1" />);

    // 1536 bytes → "1.5 KB" via the component's formatSize.
    expect(screen.getByText('1.5 KB')).toBeInTheDocument();
  });

  it('shows the empty state when there are no versions', () => {
    setVersions({ data: [] });

    render(<DocumentDetail documentId="doc-1" />);

    expect(screen.getByText('No versions are available for this document.')).toBeInTheDocument();
    expect(screen.queryByText('Current')).not.toBeInTheDocument();
  });

  it('shows the error state when the versions query fails', () => {
    setVersions({ data: undefined, error: new Error('boom') });

    render(<DocumentDetail documentId="doc-1" />);

    expect(screen.getByText('Failed to load version history.')).toBeInTheDocument();
  });
});
