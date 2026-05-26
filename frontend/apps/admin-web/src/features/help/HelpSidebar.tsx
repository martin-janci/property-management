/**
 * HelpSidebar — slide-over panel showing the contextual help article for the
 * current admin page. Opened by the "?" button in the AdminLayout topbar.
 *
 * The panel renders the article body as pre-formatted text (no external
 * markdown parser required — keeps the bundle lean). Section headings
 * (`**bold**`) and bullet lists (lines starting with `-`) are styled inline.
 */

import React, { useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import type { HelpArticle } from '../../help/articles';

// ---------------------------------------------------------------------------
// Styles (injected once)
// ---------------------------------------------------------------------------

const STYLE_ID = 'ppt-help-sidebar-styles';

function ensureStyles() {
  if (typeof document === 'undefined') return;
  if (document.getElementById(STYLE_ID)) return;
  const el = document.createElement('style');
  el.id = STYLE_ID;
  el.textContent = `
    /* Backdrop */
    .ppt-help-backdrop {
      position: fixed;
      inset: 0;
      background: rgba(0,0,0,0.25);
      z-index: 200;
      animation: ppt-help-fade-in 120ms ease;
    }
    @keyframes ppt-help-fade-in { from { opacity: 0; } to { opacity: 1; } }

    /* Panel */
    .ppt-help-panel {
      position: fixed;
      top: 0;
      right: 0;
      bottom: 0;
      width: min(400px, 90vw);
      background: var(--ppt-bg-surface, #ffffff);
      border-left: 1px solid var(--ppt-border-default, #e5e7eb);
      box-shadow: -4px 0 24px rgba(0,0,0,0.12);
      display: flex;
      flex-direction: column;
      z-index: 201;
      animation: ppt-help-slide-in 160ms ease;
    }
    @keyframes ppt-help-slide-in {
      from { transform: translateX(100%); }
      to   { transform: translateX(0); }
    }

    /* Header */
    .ppt-help-header {
      display: flex;
      align-items: center;
      gap: 10px;
      padding: 14px 18px;
      border-bottom: 1px solid var(--ppt-border-default, #e5e7eb);
      flex-shrink: 0;
    }
    .ppt-help-header__icon {
      width: 28px;
      height: 28px;
      border-radius: 50%;
      background: var(--ppt-brand-100, #dbeafe);
      color: var(--ppt-brand-700, #1d4ed8);
      display: flex;
      align-items: center;
      justify-content: center;
      font-size: 14px;
      font-weight: 700;
      flex-shrink: 0;
    }
    .ppt-help-header__title {
      flex: 1;
      font-size: 14px;
      font-weight: 600;
      color: var(--ppt-fg-primary, #111827);
      margin: 0;
    }
    .ppt-help-header__close {
      display: flex;
      align-items: center;
      justify-content: center;
      width: 28px;
      height: 28px;
      border: 0;
      background: transparent;
      border-radius: var(--ppt-radius-md, 6px);
      cursor: pointer;
      color: var(--ppt-fg-muted, #6b7280);
      font-size: 18px;
      line-height: 1;
      padding: 0;
    }
    .ppt-help-header__close:hover {
      background: var(--ppt-bg-subtle, #f3f4f6);
      color: var(--ppt-fg-primary, #111827);
    }

    /* Body */
    .ppt-help-body {
      flex: 1;
      overflow-y: auto;
      padding: 18px;
    }

    /* Article content */
    .ppt-help-article {
      font-size: 13.5px;
      line-height: 1.65;
      color: var(--ppt-fg-secondary, #374151);
    }
    .ppt-help-article p {
      margin: 0 0 12px;
    }
    .ppt-help-article strong {
      color: var(--ppt-fg-primary, #111827);
      font-weight: 600;
    }
    .ppt-help-article ul {
      margin: 0 0 12px;
      padding-left: 18px;
    }
    .ppt-help-article li {
      margin-bottom: 4px;
    }
    .ppt-help-article code {
      font-family: var(--ppt-font-mono, ui-monospace, monospace);
      font-size: 12px;
      background: var(--ppt-bg-subtle, #f3f4f6);
      border-radius: 3px;
      padding: 1px 5px;
      color: var(--ppt-fg-primary, #111827);
    }
    .ppt-help-article table {
      width: 100%;
      border-collapse: collapse;
      margin-bottom: 12px;
      font-size: 12.5px;
    }
    .ppt-help-article th,
    .ppt-help-article td {
      text-align: left;
      padding: 5px 8px;
      border: 1px solid var(--ppt-border-default, #e5e7eb);
    }
    .ppt-help-article th {
      background: var(--ppt-bg-subtle, #f3f4f6);
      font-weight: 600;
      color: var(--ppt-fg-primary, #111827);
    }

    /* Footer */
    .ppt-help-footer {
      padding: 12px 18px;
      border-top: 1px solid var(--ppt-border-default, #e5e7eb);
      flex-shrink: 0;
    }
    .ppt-help-footer__link {
      display: inline-flex;
      align-items: center;
      gap: 5px;
      font-size: 12.5px;
      color: var(--ppt-brand-600, #2563eb);
      text-decoration: none;
    }
    .ppt-help-footer__link:hover {
      text-decoration: underline;
    }
  `;
  document.head.appendChild(el);
}

// ---------------------------------------------------------------------------
// Simple inline "markdown" renderer
// ---------------------------------------------------------------------------

/**
 * Renders the article body string into React nodes without any external
 * markdown library. Handles: paragraphs, **bold**, `code`, bullet lists,
 * | table | rows |.
 */
function renderArticleBody(body: string): React.ReactNode {
  const lines = body.split('\n');
  const nodes: React.ReactNode[] = [];
  let listItems: string[] = [];
  let tableLines: string[] = [];
  let key = 0;

  function flushList() {
    if (listItems.length === 0) return;
    nodes.push(
      <ul key={key++}>
        {listItems.map((item, i) => (
          // biome-ignore lint/suspicious/noArrayIndexKey: stable list
          <li key={i}>{renderInline(item)}</li>
        ))}
      </ul>
    );
    listItems = [];
  }

  function flushTable() {
    if (tableLines.length < 2) {
      // Not a real table — emit as paragraphs
      for (const l of tableLines) {
        nodes.push(<p key={key++}>{renderInline(l)}</p>);
      }
      tableLines = [];
      return;
    }
    const rows = tableLines.map((l) =>
      l
        .replace(/^\|/, '')
        .replace(/\|$/, '')
        .split('|')
        .map((c) => c.trim())
    );
    // Row 1 = header, Row 2 = separator (---, skip), Rows 3+ = data
    const headers = rows[0];
    const dataRows = rows.slice(2);
    nodes.push(
      <table key={key++}>
        <thead>
          <tr>
            {headers.map((h, i) => (
              // biome-ignore lint/suspicious/noArrayIndexKey: stable
              <th key={i}>{renderInline(h)}</th>
            ))}
          </tr>
        </thead>
        <tbody>
          {dataRows.map((row, ri) => (
            // biome-ignore lint/suspicious/noArrayIndexKey: stable
            <tr key={ri}>
              {row.map((cell, ci) => (
                // biome-ignore lint/suspicious/noArrayIndexKey: stable
                <td key={ci}>{renderInline(cell)}</td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    );
    tableLines = [];
  }

  for (const line of lines) {
    if (line.startsWith('- ')) {
      flushTable();
      listItems.push(line.slice(2));
    } else if (line.startsWith('|')) {
      flushList();
      tableLines.push(line);
    } else {
      flushList();
      flushTable();
      if (line.trim() === '') continue;
      nodes.push(<p key={key++}>{renderInline(line)}</p>);
    }
  }

  flushList();
  flushTable();

  return <>{nodes}</>;
}

/** Renders inline **bold** and `code` spans. */
function renderInline(text: string): React.ReactNode {
  // Split on **...** and `...`
  const parts: React.ReactNode[] = [];
  const re = /(\*\*[^*]+\*\*|`[^`]+`)/g;
  let last = 0;
  let m: RegExpExecArray | null;
  let idx = 0;

  // biome-ignore lint/suspicious/noAssignInExpressions: intentional
  while ((m = re.exec(text)) !== null) {
    if (m.index > last) {
      parts.push(<React.Fragment key={idx++}>{text.slice(last, m.index)}</React.Fragment>);
    }
    const token = m[0];
    if (token.startsWith('**')) {
      parts.push(<strong key={idx++}>{token.slice(2, -2)}</strong>);
    } else {
      parts.push(<code key={idx++}>{token.slice(1, -1)}</code>);
    }
    last = m.index + token.length;
  }
  if (last < text.length) {
    parts.push(<React.Fragment key={idx++}>{text.slice(last)}</React.Fragment>);
  }
  return <>{parts}</>;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

interface HelpSidebarProps {
  article: HelpArticle;
  onClose: () => void;
}

export function HelpSidebar({ article, onClose }: HelpSidebarProps) {
  ensureStyles();
  const { t } = useTranslation();
  const closeRef = useRef<HTMLButtonElement>(null);

  // Focus the close button on mount for keyboard accessibility
  useEffect(() => {
    closeRef.current?.focus();
  }, []);

  // Close on Escape
  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === 'Escape') onClose();
    }
    document.addEventListener('keydown', onKeyDown);
    return () => document.removeEventListener('keydown', onKeyDown);
  }, [onClose]);

  return (
    <>
      {/* Backdrop */}
      <div
        className="ppt-help-backdrop"
        onClick={onClose}
        aria-hidden="true"
      />

      {/* Panel */}
      <aside
        className="ppt-help-panel"
        role="complementary"
        aria-label={t('admin.help.panelLabel', 'Contextual help')}
      >
        {/* Header */}
        <div className="ppt-help-header">
          <div className="ppt-help-header__icon" aria-hidden="true">?</div>
          <h2 className="ppt-help-header__title">{article.title}</h2>
          <button
            ref={closeRef}
            type="button"
            className="ppt-help-header__close"
            onClick={onClose}
            aria-label={t('admin.help.close', 'Close help')}
          >
            ×
          </button>
        </div>

        {/* Body */}
        <div className="ppt-help-body">
          <div className="ppt-help-article">
            {renderArticleBody(article.body)}
          </div>
        </div>

        {/* Footer — link to external docs if available */}
        {article.docsUrl && (
          <div className="ppt-help-footer">
            <a
              href={article.docsUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="ppt-help-footer__link"
            >
              {t('admin.help.openDocs', 'Open full documentation')} ↗
            </a>
          </div>
        )}
      </aside>
    </>
  );
}
