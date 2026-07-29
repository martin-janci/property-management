import type { ResolvedScreen } from '@ppt/api-client';
import type React from 'react';
import { useTranslation } from 'react-i18next';
import { ErrorBoundary } from '../../components/ErrorBoundary';
import type { SectionRegistry } from './registry';
import './LayoutRenderer.css';

const warnedTypes = new Set<string>();

function Placeholder() {
  const { t } = useTranslation();
  return (
    <div className="layout-placeholder" role="status">
      <p className="layout-placeholder__title">{t('layout.placeholderTitle')}</p>
      <p className="layout-placeholder__body">{t('layout.placeholderBody')}</p>
    </div>
  );
}

export interface LayoutRendererProps {
  layout: ResolvedScreen;
  registry: SectionRegistry;
  /** When set, every section wrapper intercepts clicks (capture phase),
   *  calls preventDefault + stopPropagation, and invokes this callback
   *  with the section type. When unset: no listener, clicks pass through. */
  onSectionClick?: (type: string) => void;
}

/** Renders a resolved layout defensively (spec §4): unknown type → skip +
 *  warn once; placeholder presentation → Placeholder; crashing section →
 *  Placeholder via boundary, siblings unaffected; container owns spacing. */
export function LayoutRenderer({ layout, registry, onSectionClick }: LayoutRendererProps) {
  return (
    <div className="layout-sections">
      {layout.sections.map((section) => {
        // Element-level defense: a malformed cached/preview layout may carry
        // null / non-object elements or a missing string type — skip them so
        // they can't throw outside the per-section error boundary.
        if (
          section === null ||
          typeof section !== 'object' ||
          typeof (section as { type?: unknown }).type !== 'string'
        ) {
          return null;
        }
        const def = registry[section.type];
        if (!def) {
          if (!warnedTypes.has(section.type)) {
            warnedTypes.add(section.type);
            console.warn(`layout: unknown section type ${section.type} — skipped`);
          }
          return null;
        }

        const handleClickCapture = onSectionClick
          ? (e: React.MouseEvent) => {
              e.preventDefault();
              e.stopPropagation();
              onSectionClick(section.type);
            }
          : undefined;

        if (section.presentation === 'placeholder') {
          return (
            <div
              key={section.type}
              data-layout-section={section.type}
              onClickCapture={handleClickCapture}
            >
              <Placeholder />
            </div>
          );
        }
        const Component = def.component;
        return (
          <div
            key={section.type}
            data-layout-section={section.type}
            onClickCapture={handleClickCapture}
          >
            <ErrorBoundary fallback={<Placeholder />}>
              <Component mode={section.mode} props={section.props} />
            </ErrorBoundary>
          </div>
        );
      })}
    </div>
  );
}
