/**
 * Documents + News route group (Epic 39 / Epic 59).
 *
 * Owns the document and news route-wrapper components and the `<Route>` table
 * fragments. Extracted from App.tsx to isolate document/news work.
 */
import { useTranslation } from 'react-i18next';
import { Link, Route, useParams } from 'react-router-dom';
import { ProtectedRoute } from '../../components';
import { useAuth } from '../../contexts';
import {
  ArticleDetailPage,
  DocumentDetailPage,
  DocumentsPage,
  DocumentUploadPage,
  FolderTreePage,
  NewsListPage,
} from '../lazyRoutes';

/** Route wrapper for documents page */
function DocumentsPageRoute() {
  const { user } = useAuth();
  const organizationId = user?.organizationId ?? 'default-org';
  return <DocumentsPage organizationId={organizationId} />;
}

/** Route wrapper for folder-tree page (gap-7a-2) */
function FolderTreePageRoute() {
  const { user } = useAuth();
  const organizationId = user?.organizationId ?? 'default-org';
  return <FolderTreePage organizationId={organizationId} />;
}

/** Route wrapper for document detail page to extract params */
function DocumentDetailRoute() {
  const { t } = useTranslation();
  const { documentId } = useParams<{ documentId: string }>();
  if (!documentId) {
    return (
      <div className="error-page">
        <h1>{t('errors.documentNotFound')}</h1>
        <p>{t('errors.documentNotFoundDesc')}</p>
        <Link to="/documents">{t('common.backToDocuments')}</Link>
      </div>
    );
  }
  return <DocumentDetailPage documentId={documentId} />;
}

/** Route wrapper for article detail page to extract params */
function ArticleDetailRoute() {
  const { t } = useTranslation();
  const { articleId } = useParams<{ articleId: string }>();
  if (!articleId) return <div>{t('errors.articleNotFound')}</div>;
  return <ArticleDetailPage articleId={articleId} />;
}

/** Document Intelligence routes (Epic 39). */
export function documentRoutes() {
  return (
    <>
      <Route
        path="/documents"
        element={
          <ProtectedRoute>
            <DocumentsPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/documents/folders"
        element={
          <ProtectedRoute>
            <FolderTreePageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/documents/upload"
        element={
          <ProtectedRoute>
            <DocumentUploadPage />
          </ProtectedRoute>
        }
      />
      <Route
        path="/documents/:documentId"
        element={
          <ProtectedRoute>
            <DocumentDetailRoute />
          </ProtectedRoute>
        }
      />
    </>
  );
}

/** News routes (Epic 59). */
export function newsRoutes() {
  return (
    <>
      <Route
        path="/news"
        element={
          <ProtectedRoute>
            <NewsListPage />
          </ProtectedRoute>
        }
      />
      <Route
        path="/news/:articleId"
        element={
          <ProtectedRoute>
            <ArticleDetailRoute />
          </ProtectedRoute>
        }
      />
    </>
  );
}
