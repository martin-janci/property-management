/**
 * ListingDetailContent Component
 *
 * Client-side presentation component for listing detail (Epic 44, Story 44.3).
 */

'use client';

import type { ListingDetail } from '@ppt/reality-api-client';
import { useTranslations } from 'next-intl';
import { Footer, Header } from '@/components/ui';
import type { ResolvedScreen } from '@/lib/layout';
import { DEFAULT_LISTING_DETAIL_LAYOUT } from '@/lib/layout';
import { ContactForm } from './ContactForm';
import { LayoutSections, Placeholder } from './LayoutSections';
import { listingRegistry } from './sections/registry';

interface ListingDetailContentProps {
  listing: ListingDetail | null;
  jsonLd?: object;
  layout?: ResolvedScreen;
}

export function ListingNotFound() {
  const t = useTranslations('listing');
  return (
    <div className="page-container">
      <Header />
      <main className="main">
        <div className="not-found">
          <h1>{t('notFound')}</h1>
          <p>{t('notFoundDesc')}</p>
          <a href="/listings" className="back-link">
            {t('browseAll')}
          </a>
        </div>
      </main>
      <Footer />
      <style jsx>{`
        .page-container {
          min-height: 100vh;
          display: flex;
          flex-direction: column;
        }
        .main {
          flex: 1;
          display: flex;
          align-items: center;
          justify-content: center;
        }
        .not-found {
          text-align: center;
          padding: 64px 16px;
        }
        .not-found h1 {
          font-size: 2rem;
          color: var(--ppt-fg-primary);
          margin: 0 0 16px;
        }
        .not-found p {
          color: var(--ppt-fg-muted);
          margin: 0 0 24px;
        }
        .back-link {
          color: var(--ppt-color-primary);
          text-decoration: none;
        }
        .back-link:hover {
          text-decoration: underline;
        }
      `}</style>
    </div>
  );
}

export function ListingDetailContent({ listing, jsonLd, layout }: ListingDetailContentProps) {
  const t = useTranslations('listing');
  const tNav = useTranslations('nav');

  if (!listing) {
    return <ListingNotFound />;
  }

  const resolvedLayout = layout ?? DEFAULT_LISTING_DETAIL_LAYOUT;

  // Determine agent-contact section presentation from layout
  const agentContactSection = resolvedLayout.sections.find((s) => s.type === 'agent-contact.v1');

  // Main layout excludes agent-contact (sidebar handles it)
  const mainLayout: ResolvedScreen = {
    ...resolvedLayout,
    sections: resolvedLayout.sections.filter((s) => s.type !== 'agent-contact.v1'),
  };

  return (
    <div className="page-container">
      {/* JSON-LD structured data for SEO */}
      {jsonLd && (
        <script
          type="application/ld+json"
          // biome-ignore lint/security/noDangerouslySetInnerHtml: JSON-LD for SEO requires dangerouslySetInnerHTML
          dangerouslySetInnerHTML={{ __html: JSON.stringify(jsonLd) }}
        />
      )}
      <Header />
      <main className="main">
        <div className="container">
          {/* Breadcrumb */}
          <nav className="breadcrumb" aria-label="Breadcrumb">
            <a href="/">{tNav('home')}</a>
            <span className="separator">/</span>
            <a href="/listings">{tNav('allListings')}</a>
            <span className="separator">/</span>
            <span className="current">{listing.address?.city}</span>
          </nav>

          <div className="content-grid">
            {/* Main Content */}
            <div className="main-content">
              <LayoutSections layout={mainLayout} listing={listing} registry={listingRegistry} />
            </div>

            {/* Sidebar */}
            <div className="sidebar">
              {agentContactSection?.presentation === 'placeholder' ? (
                <Placeholder />
              ) : agentContactSection?.presentation === 'visible' && listing.agent ? (
                <ContactForm listingId={listing.id} agent={listing.agent} />
              ) : null}
            </div>
          </div>
        </div>
      </main>
      <Footer />

      <style jsx>{`
        .page-container {
          min-height: 100vh;
          display: flex;
          flex-direction: column;
          background: var(--ppt-bg-app);
        }

        .main {
          flex: 1;
          padding: 24px 0;
        }

        .container {
          max-width: 1280px;
          margin: 0 auto;
          padding: 0 16px;
        }

        .breadcrumb {
          font-size: 14px;
          color: var(--ppt-fg-muted);
          margin-bottom: 24px;
        }

        .breadcrumb a {
          color: var(--ppt-fg-muted);
          text-decoration: none;
        }

        .breadcrumb a:hover {
          color: var(--ppt-color-primary);
        }

        .separator {
          margin: 0 8px;
        }

        .current {
          color: var(--ppt-fg-primary);
        }

        .content-grid {
          display: grid;
          gap: 32px;
        }

        @media (min-width: 1024px) {
          .content-grid {
            grid-template-columns: 1fr 380px;
          }
        }

        .main-content {
          min-width: 0;
        }

        .sidebar {
          position: sticky;
          top: 88px;
          height: fit-content;
        }
      `}</style>
    </div>
  );
}
