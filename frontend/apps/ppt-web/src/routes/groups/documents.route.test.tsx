/// <reference types="vitest/globals" />
/**
 * Documents route-wrapper gating tests (code-review:
 * ppt-web-core-documents-default-org-sentinel).
 *
 * The document route wrappers (`DocumentsPageRoute`, `FolderTreePageRoute`,
 * `DocumentUploadPageRoute`) used to fabricate a `'default-org'` sentinel
 * (`user?.organizationId ?? 'default-org'`) when the authenticated user had no
 * `organizationId`, silently firing RLS-scoped document queries against a
 * non-existent organization. The sibling route groups (disputes, outages,
 * faults, announcements) instead gate with `<AuthRequiredGate />`.
 *
 * These tests pin the corrected contract so it can't regress back to a
 * sentinel:
 *   - no org → render AuthRequiredGate, never mount the page
 *   - with org → mount the page with the REAL organizationId, never 'default-org'
 */
import { render, screen } from '@testing-library/react';

// ── per-test mutable auth user ──
const auth = vi.hoisted(() => ({
  user: undefined as { id: string; organizationId?: string } | undefined,
}));

vi.mock('../../contexts', () => ({
  useAuth: () => ({ user: auth.user }),
}));

// ── AuthRequiredGate + ProtectedRoute stubs (both imported by documents.tsx) ──
vi.mock('../../components', () => ({
  AuthRequiredGate: () => <div data-testid="auth-gate">auth required</div>,
  ProtectedRoute: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

// ── lazy page stubs: capture the props each wrapper passes down ──
const captured = vi.hoisted(() => ({
  documents: undefined as Record<string, unknown> | undefined,
  folders: undefined as Record<string, unknown> | undefined,
  upload: undefined as Record<string, unknown> | undefined,
}));

vi.mock('../lazyRoutes', () => ({
  ArticleDetailPage: () => <div>article</div>,
  DocumentDetailPage: () => <div>detail</div>,
  DocumentSignPage: () => <div>sign</div>,
  DocumentsPage: (props: Record<string, unknown>) => {
    captured.documents = props;
    return <div data-testid="documents-page" />;
  },
  DocumentTemplatesPage: () => <div>templates</div>,
  DocumentUploadPage: (props: Record<string, unknown>) => {
    captured.upload = props;
    return <div data-testid="upload-page" />;
  },
  FolderTreePage: (props: Record<string, unknown>) => {
    captured.folders = props;
    return <div data-testid="folders-page" />;
  },
  NewsListPage: () => <div>news</div>,
}));

import { DocumentsPageRoute, DocumentUploadPageRoute, FolderTreePageRoute } from './documents';

beforeEach(() => {
  auth.user = undefined;
  captured.documents = undefined;
  captured.folders = undefined;
  captured.upload = undefined;
});

const wrappers = [
  ['DocumentsPageRoute', DocumentsPageRoute, 'documents-page', 'documents'],
  ['FolderTreePageRoute', FolderTreePageRoute, 'folders-page', 'folders'],
  ['DocumentUploadPageRoute', DocumentUploadPageRoute, 'upload-page', 'upload'],
] as const;

describe('documents route wrappers — org gating (no default-org sentinel)', () => {
  for (const [name, Wrapper, pageTestId, capKey] of wrappers) {
    it(`${name}: renders AuthRequiredGate when the user has no organizationId`, () => {
      auth.user = { id: 'u-1' }; // no organizationId
      render(<Wrapper />);

      expect(screen.getByTestId('auth-gate')).toBeInTheDocument();
      expect(screen.queryByTestId(pageTestId)).not.toBeInTheDocument();
      // The page must never be mounted with a fabricated org.
      expect(captured[capKey]).toBeUndefined();
    });

    it(`${name}: passes the real organizationId through when present`, () => {
      auth.user = { id: 'u-1', organizationId: 'org-42' };
      render(<Wrapper />);

      expect(screen.getByTestId(pageTestId)).toBeInTheDocument();
      expect(screen.queryByTestId('auth-gate')).not.toBeInTheDocument();
      expect(captured[capKey]?.organizationId).toBe('org-42');
      expect(captured[capKey]?.organizationId).not.toBe('default-org');
    });
  }
});
