'use client';

import type { ListingDetail } from '@ppt/reality-api-client';
import { useTranslations } from 'next-intl';
import { Component, type ErrorInfo, type ReactNode } from 'react';
import type { ResolvedScreen } from '../../lib/layout';
import type { ListingRegistry } from './sections/registry';

// ---------------------------------------------------------------------------
// Warn-once guard
// ---------------------------------------------------------------------------

const warnedTypes = new Set<string>();

// ---------------------------------------------------------------------------
// Placeholder
// ---------------------------------------------------------------------------

function Placeholder() {
  const t = useTranslations('layout');
  return (
    <div className="layout-placeholder" role="status">
      <p className="layout-placeholder__title">{t('placeholderTitle')}</p>
      <p className="layout-placeholder__body">{t('placeholderBody')}</p>
      <style jsx>{`
        .layout-placeholder {
          min-height: 96px;
          border: 2px dashed #ccc;
          display: flex;
          flex-direction: column;
          align-items: center;
          justify-content: center;
          padding: 16px;
          color: #888;
        }
        .layout-placeholder__title {
          font-weight: 600;
          margin: 0 0 4px;
        }
        .layout-placeholder__body {
          margin: 0;
          font-size: 0.875rem;
        }
      `}</style>
    </div>
  );
}

// ---------------------------------------------------------------------------
// SectionBoundary — minimal class error boundary (reality-web has no reusable one)
// ---------------------------------------------------------------------------

interface BoundaryState {
  hasError: boolean;
}

class SectionBoundary extends Component<{ children: ReactNode }, BoundaryState> {
  constructor(props: { children: ReactNode }) {
    super(props);
    this.state = { hasError: false };
  }

  static getDerivedStateFromError(_error: Error): BoundaryState {
    return { hasError: true };
  }

  componentDidCatch(_error: Error, _info: ErrorInfo) {
    // Intentionally swallowed — console.error suppressed by tests
  }

  render() {
    if (this.state.hasError) {
      return <Placeholder />;
    }
    return this.props.children;
  }
}

// ---------------------------------------------------------------------------
// LayoutSections
// ---------------------------------------------------------------------------

export interface LayoutSectionsProps {
  layout: ResolvedScreen;
  listing: ListingDetail;
  registry: ListingRegistry;
}

/**
 * Renders a resolved listing-detail layout defensively (spec §4):
 * - unknown type → skip + warn once per type
 * - placeholder presentation → <Placeholder />
 * - crashing section → <Placeholder /> via SectionBoundary; siblings unaffected
 * - listing forwarded to every section component
 */
export function LayoutSections({ layout, listing, registry }: LayoutSectionsProps) {
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
          <SectionBoundary key={`${section.type}-${i}`}>
            <Component listing={listing} mode={section.mode} props={section.props} />
          </SectionBoundary>
        );
      })}
      <style jsx>{`
        .layout-sections {
          display: flex;
          flex-direction: column;
          gap: 24px;
        }
      `}</style>
    </div>
  );
}
