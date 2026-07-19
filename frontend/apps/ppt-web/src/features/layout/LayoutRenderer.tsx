import { useTranslation } from 'react-i18next';
import type { ResolvedScreen } from '@ppt/api-client';
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
}

/** Renders a resolved layout defensively (spec §4): unknown type → skip +
 *  warn once; placeholder presentation → Placeholder; crashing section →
 *  Placeholder via boundary, siblings unaffected; container owns spacing. */
export function LayoutRenderer({ layout, registry }: LayoutRendererProps) {
  return (
    <div className="layout-sections">
      {layout.sections.map((section, i) => {
        const def = registry[section.type];
        if (!def) {
          if (!warnedTypes.has(section.type)) {
            warnedTypes.add(section.type);
            console.warn(`layout: unknown section type ${section.type} — skipped`);
          }
          return null;
        }
        if (section.presentation === 'placeholder') {
          return <Placeholder key={`${section.type}-${i}`} />;
        }
        const Component = def.component;
        return (
          <ErrorBoundary key={`${section.type}-${i}`} fallback={<Placeholder />}>
            <Component mode={section.mode} props={section.props} />
          </ErrorBoundary>
        );
      })}
    </div>
  );
}
