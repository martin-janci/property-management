/**
 * DocumentTemplatesPage (Epic 7B, Story 7B.2 — Document Templates & Generation).
 *
 * Lists the organization's document templates and launches the
 * {@link GenerateDocumentDialog} for the template → document flow. Wired to the
 * hand-written `@ppt/api-client` templates module (`useTemplates`).
 *
 * Route: `/documents/templates` (mounted by `routes/groups/documents.tsx`).
 */

import { type DocumentTemplateSummary, useTemplates } from '@ppt/api-client';
import { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Link } from 'react-router-dom';
import { GenerateDocumentDialog } from '../components/GenerateDocumentDialog';
import '../document-templates.css';

const TEMPLATE_TYPES = ['lease', 'notice', 'invoice', 'report', 'minutes', 'contract', 'custom'];

export function DocumentTemplatesPage() {
  const { t } = useTranslation();
  const [search, setSearch] = useState('');
  const [typeFilter, setTypeFilter] = useState('');
  const [active, setActive] = useState<DocumentTemplateSummary | null>(null);

  const params = useMemo(
    () => ({
      search: search.trim() || undefined,
      template_type: typeFilter || undefined,
    }),
    [search, typeFilter]
  );

  const { data, isLoading, isError, refetch, isFetching } = useTemplates(params);
  const templates = data?.templates ?? [];

  return (
    <div className="dt-page">
      <header className="page-header dt-header">
        <div>
          <h1 className="page-title">{t('documentTemplates.title')}</h1>
          <p className="dt-subtitle">{t('documentTemplates.subtitle')}</p>
        </div>
        <Link to="/documents" className="dt-back-link">
          {t('common.backToDocuments')}
        </Link>
      </header>

      <div className="dt-toolbar">
        <input
          className="dt-search"
          type="search"
          value={search}
          placeholder={t('documentTemplates.searchPlaceholder')}
          onChange={(e) => setSearch(e.target.value)}
          aria-label={t('documentTemplates.searchPlaceholder')}
        />
        <select
          className="dt-type-filter"
          value={typeFilter}
          onChange={(e) => setTypeFilter(e.target.value)}
          aria-label={t('documentTemplates.typeFilter')}
        >
          <option value="">{t('documentTemplates.allTypes')}</option>
          {TEMPLATE_TYPES.map((type) => (
            <option key={type} value={type}>
              {t(`documentTemplates.types.${type}`)}
            </option>
          ))}
        </select>
      </div>

      {isLoading && (
        <ul className="dt-list" aria-hidden="true">
          {[0, 1, 2, 3].map((i) => (
            <li key={i} className="dt-card dt-card--skeleton" />
          ))}
        </ul>
      )}

      {isError && (
        <div className="dt-state dt-state--error" role="alert">
          <p>{t('documentTemplates.loadError')}</p>
          <button type="button" className="dt-btn" onClick={() => refetch()}>
            {t('common.retry')}
          </button>
        </div>
      )}

      {!isLoading && !isError && templates.length === 0 && (
        <div className="dt-state dt-state--empty">
          <h2>{t('documentTemplates.empty.title')}</h2>
          <p>
            {search || typeFilter
              ? t('documentTemplates.empty.noMatches')
              : t('documentTemplates.empty.body')}
          </p>
        </div>
      )}

      {!isLoading && !isError && templates.length > 0 && (
        <ul className="dt-list">
          {templates.map((tpl) => (
            <li key={tpl.id} className="dt-card">
              <div className="dt-card__main">
                <div className="dt-card__heading">
                  <h2 className="dt-card__title">{tpl.name}</h2>
                  <span className="dt-badge">
                    {t(`documentTemplates.types.${tpl.template_type}`)}
                  </span>
                </div>
                {tpl.description && <p className="dt-card__desc">{tpl.description}</p>}
                <p className="dt-card__meta">
                  {t('documentTemplates.card.placeholders', { count: tpl.placeholder_count })}
                  {' · '}
                  {t('documentTemplates.card.usage', { count: tpl.usage_count })}
                </p>
              </div>
              <button
                type="button"
                className="dt-btn dt-btn--primary"
                onClick={() => setActive(tpl)}
              >
                {t('documentTemplates.card.generate')}
              </button>
            </li>
          ))}
        </ul>
      )}

      {isFetching && !isLoading && <div className="dt-fetching">{t('common.loading')}</div>}

      {active && (
        <GenerateDocumentDialog
          templateId={active.id}
          templateName={active.name}
          onClose={() => setActive(null)}
        />
      )}
    </div>
  );
}
