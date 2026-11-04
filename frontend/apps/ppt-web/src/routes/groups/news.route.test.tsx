/// <reference types="vitest/globals" />
/**
 * News route-table tests (nav-fixes).
 *
 * "Create Article" links to `/news/new` and the card's Edit action to
 * `/news/:id/edit`. Neither had a route: `/news/new` was swallowed by
 * `/news/:articleId` (detail page fetching article "new" → error) and the
 * edit URL fell through to the 404. These tests pin the registration order
 * and the manager-only gate on create.
 */
import { render, screen } from '@testing-library/react';
import { MemoryRouter, Routes } from 'react-router-dom';

vi.mock('../../contexts', () => ({
  useAuth: () => ({ user: { id: 'u-1', organizationId: 'org-1' } }),
}));

vi.mock('../../components', () => ({
  AuthRequiredGate: () => <div data-testid="auth-gate" />,
  ProtectedRoute: ({
    children,
    requiredRoles,
  }: {
    children: React.ReactNode;
    requiredRoles?: string[];
  }) => (
    <div data-testid="protected" data-roles={(requiredRoles ?? []).join(',')}>
      {children}
    </div>
  ),
}));

vi.mock('../lazyRoutes', () => ({
  ArticleDetailPage: ({ articleId }: { articleId: string }) => (
    <div data-testid="article-detail">{articleId}</div>
  ),
  CreateArticlePage: () => <div data-testid="article-create" />,
  EditArticlePage: () => <div data-testid="article-edit" />,
  DocumentDetailPage: () => null,
  DocumentSignPage: () => null,
  DocumentsPage: () => null,
  DocumentTemplatesPage: () => null,
  DocumentUploadPage: () => null,
  FolderTreePage: () => null,
  NewsListPage: () => <div data-testid="news-list" />,
}));

import { newsRoutes } from './documents';

function renderAt(path: string) {
  return render(
    <MemoryRouter initialEntries={[path]}>
      <Routes>{newsRoutes()}</Routes>
    </MemoryRouter>
  );
}

describe('news routes', () => {
  it('/news/new renders the create page (not the detail route)', () => {
    renderAt('/news/new');
    expect(screen.getByTestId('article-create')).toBeInTheDocument();
    expect(screen.queryByTestId('article-detail')).not.toBeInTheDocument();
  });

  it('/news/new is gated to manager roles', () => {
    renderAt('/news/new');
    const roles = screen.getByTestId('protected').getAttribute('data-roles') ?? '';
    expect(roles.split(',')).toEqual(expect.arrayContaining(['manager', 'org_admin']));
    expect(roles).not.toContain('resident');
  });

  it('/news/:id/edit renders the edit page', () => {
    renderAt('/news/a-1/edit');
    expect(screen.getByTestId('article-edit')).toBeInTheDocument();
  });

  it('/news/:articleId still renders the detail page', () => {
    renderAt('/news/a-1');
    expect(screen.getByTestId('article-detail')).toHaveTextContent('a-1');
  });
});
