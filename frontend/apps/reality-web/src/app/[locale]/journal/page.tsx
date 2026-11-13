'use client';

/**
 * Journal / magazine index — Reality Portal.
 * Screen-map: docs/screens/reality/journal.md
 */

import { useLocale, useTranslations } from 'next-intl';
import { useState } from 'react';
import { Footer, Header } from '@/components/ui';
import { journalEmptyCopy } from './_empty';
import { EDITORS_PICKS, MOCK_ARTICLES, MOST_READ } from './_mock';

// NOTE: MOCK_ARTICLES (titles, excerpts, bylines, dates) are design-review
// scaffolding — the journal has no CMS / reality-server feed yet (screen-map
// docs/screens/reality/journal.md → endpoints: []). They are gated to local
// development (see JournalPage below) so no deployed build ships fabricated
// editorial content as if it were real; deployed builds render the honest
// "coming soon" state from `_empty.ts` instead. When the real feed lands these
// records will carry their own locale field and `articles` becomes a data
// hook. The page chrome (h1, section headings, newsletter form) is localised
// via pages.journal.*; the placeholder empty-state copy is co-located in
// `_empty.ts` so the whole pre-CMS layer is removed together.

// TODO: replace article cards with @ppt/ui-kit/Card once available

function ArticleCard({
  article,
  variant = 'default',
}: {
  article: (typeof MOCK_ARTICLES)[0];
  variant?: 'featured' | 'default' | 'compact';
}) {
  const isFeatured = variant === 'featured';
  const isCompact = variant === 'compact';

  return (
    <article
      style={{
        background: 'var(--ppt-bg-surface)',
        borderRadius: 12,
        overflow: 'hidden',
        boxShadow: '0 1px 4px rgba(0,0,0,.07)',
        display: isCompact ? 'flex' : 'block',
        gap: isCompact ? 16 : 0,
        alignItems: isCompact ? 'center' : 'initial',
      }}
    >
      {!isCompact && (
        // eslint-disable-next-line @next/next/no-img-element
        <img
          src={article.imageUrl}
          alt={article.title}
          style={{ width: '100%', height: isFeatured ? 360 : 200, objectFit: 'cover' }}
        />
      )}
      {isCompact && (
        // eslint-disable-next-line @next/next/no-img-element
        <img
          src={article.imageUrl}
          alt={article.title}
          style={{ width: 80, height: 80, objectFit: 'cover', borderRadius: 8, flexShrink: 0 }}
        />
      )}
      <div style={{ padding: isCompact ? '8px 0' : 20 }}>
        <span
          style={{
            display: 'inline-block',
            fontSize: '0.75rem',
            fontWeight: 600,
            color: 'var(--ppt-color-primary, #2563eb)',
            textTransform: 'uppercase',
            letterSpacing: '.5px',
            marginBottom: 6,
          }}
        >
          {article.category}
        </span>
        <h3
          style={{
            fontSize: isFeatured ? '1.375rem' : isCompact ? '0.9375rem' : '1.0625rem',
            fontWeight: 700,
            color: 'var(--ppt-fg-primary)',
            margin: '0 0 8px',
            lineHeight: 1.35,
          }}
        >
          {article.title}
        </h3>
        {!isCompact && (
          <p
            style={{
              fontSize: '0.875rem',
              color: 'var(--ppt-fg-secondary)',
              lineHeight: 1.65,
              margin: '0 0 12px',
            }}
          >
            {article.excerpt}
          </p>
        )}
        <div style={{ fontSize: '0.8125rem', color: 'var(--ppt-fg-muted, #9ca3af)' }}>
          {article.author} · {article.readMin} min čítania
        </div>
      </div>
    </article>
  );
}

export default function JournalPage() {
  const t = useTranslations('pages.journal');
  const locale = useLocale();
  const [email, setEmail] = useState('');
  const [subscribed, setSubscribed] = useState(false);

  // Gate the placeholder articles to local development (same convention as the
  // DevPanel / MSW mocks in DevPanelMount.tsx): deployed builds get an empty
  // feed and show the honest "coming soon" state below instead of fabricated
  // bylines and dates. Replace with the real data hook when the feed ships.
  const articles = process.env.NODE_ENV === 'development' ? MOCK_ARTICLES : [];
  const hasArticles = articles.length > 0;
  const emptyCopy = journalEmptyCopy(locale);

  const featured = articles.find((a) => a.featured);
  const latest = articles.filter((a) => !a.featured).slice(0, 4);
  const editorsPicks = articles.filter((a) => EDITORS_PICKS.includes(a.id));
  const mostRead = articles.filter((a) => MOST_READ.includes(a.id));

  const handleSubscribe = (e: React.FormEvent) => {
    e.preventDefault();
    if (email.trim()) setSubscribed(true);
  };

  return (
    <div
      data-i18n="pages.journal.root"
      style={{
        minHeight: '100vh',
        display: 'flex',
        flexDirection: 'column',
        background: 'var(--ppt-bg-app)',
      }}
    >
      <Header />

      <main
        style={{
          flex: 1,
          maxWidth: 1200,
          margin: '0 auto',
          padding: '48px 24px',
          width: '100%',
          boxSizing: 'border-box',
        }}
      >
        {/* Page header */}
        <div style={{ marginBottom: 40 }}>
          <h1
            style={{
              fontSize: '2rem',
              fontWeight: 800,
              color: 'var(--ppt-fg-primary)',
              marginBottom: 8,
            }}
          >
            {t('title')}
          </h1>
          <p style={{ color: 'var(--ppt-fg-secondary)' }}>{t('subtitle')}</p>
        </div>

        <div className="journal-grid">
          {/* Main column */}
          <div>
            {/* Honest empty state — shown on deployed builds until a real
                journal feed replaces the dev-only placeholder articles. */}
            {!hasArticles && (
              <section
                style={{
                  background: 'var(--ppt-bg-surface)',
                  border: '1px solid var(--ppt-border-default, #e5e7eb)',
                  borderRadius: 14,
                  padding: '48px 32px',
                  textAlign: 'center',
                  marginBottom: 48,
                }}
              >
                <h2
                  style={{
                    fontSize: '1.375rem',
                    fontWeight: 700,
                    color: 'var(--ppt-fg-primary)',
                    margin: '0 0 8px',
                  }}
                >
                  {emptyCopy.title}
                </h2>
                <p style={{ color: 'var(--ppt-fg-secondary)', margin: 0 }}>{emptyCopy.body}</p>
              </section>
            )}

            {/* Featured */}
            {featured && (
              <section style={{ marginBottom: 48 }}>
                <h2
                  style={{
                    fontSize: '0.875rem',
                    fontWeight: 700,
                    color: 'var(--ppt-fg-muted, #9ca3af)',
                    textTransform: 'uppercase',
                    letterSpacing: '.8px',
                    marginBottom: 16,
                  }}
                >
                  {t('featured')}
                </h2>
                <ArticleCard article={featured} variant="featured" />
              </section>
            )}

            {/* Latest grid — dev-only until the real feed lands */}
            {hasArticles && (
              <section style={{ marginBottom: 48 }}>
                <h2
                  style={{
                    fontSize: '1.25rem',
                    fontWeight: 700,
                    color: 'var(--ppt-fg-primary)',
                    marginBottom: 20,
                  }}
                >
                  {t('latest')}
                </h2>
                <div
                  style={{
                    display: 'grid',
                    gridTemplateColumns: 'repeat(auto-fill, minmax(260px, 1fr))',
                    gap: 20,
                  }}
                >
                  {latest.map((article) => (
                    <ArticleCard key={article.id} article={article} />
                  ))}
                </div>
              </section>
            )}

            {/* Newsletter */}
            <section
              style={{
                background:
                  'linear-gradient(135deg, var(--ppt-brand-700, #1d4ed8), var(--ppt-brand-500, #3b82f6))',
                borderRadius: 14,
                padding: '36px 32px',
                color: '#fff',
                marginBottom: 48,
              }}
            >
              <h2 style={{ fontSize: '1.375rem', fontWeight: 700, marginBottom: 8 }}>
                {t('newsletterTitle')}
              </h2>
              <p style={{ opacity: 0.85, marginBottom: 20 }}>{t('newsletterIntro')}</p>
              {subscribed ? (
                <p style={{ fontWeight: 600, fontSize: '1.0625rem' }}>{t('newsletterSuccess')}</p>
              ) : (
                <form
                  onSubmit={handleSubscribe}
                  style={{ display: 'flex', gap: 10, flexWrap: 'wrap' }}
                >
                  <input
                    type="email"
                    required
                    placeholder={t('newsletterEmailPlaceholder')}
                    value={email}
                    onChange={(e) => setEmail(e.target.value)}
                    style={{
                      flex: 1,
                      minWidth: 200,
                      padding: '11px 16px',
                      borderRadius: 8,
                      border: 'none',
                      fontSize: '0.9375rem',
                      outline: 'none',
                    }}
                  />
                  <button
                    type="submit"
                    style={{
                      padding: '11px 24px',
                      background: '#fff',
                      color: 'var(--ppt-brand-700, #1d4ed8)',
                      border: 'none',
                      borderRadius: 8,
                      fontWeight: 700,
                      cursor: 'pointer',
                      whiteSpace: 'nowrap',
                    }}
                  >
                    {t('newsletterSubmit')}
                  </button>
                </form>
              )}
            </section>

            {/* Editor's picks — dev-only until the real feed lands */}
            {hasArticles && (
              <section>
                <h2
                  style={{
                    fontSize: '1.25rem',
                    fontWeight: 700,
                    color: 'var(--ppt-fg-primary)',
                    marginBottom: 20,
                  }}
                >
                  {t('editorsPicks')}
                </h2>
                <div style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
                  {editorsPicks.map((article) => (
                    <ArticleCard key={article.id} article={article} variant="compact" />
                  ))}
                </div>
              </section>
            )}
          </div>

          {/* Sidebar — Most read (dev-only until the real feed lands) */}
          {hasArticles && (
            <aside style={{ position: 'sticky', top: 24 }}>
              <h2
                style={{
                  fontSize: '1.0625rem',
                  fontWeight: 700,
                  color: 'var(--ppt-fg-primary)',
                  marginBottom: 16,
                }}
              >
                {t('mostRead')}
              </h2>
              <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
                {mostRead.map((article, idx) => (
                  <div
                    key={article.id}
                    style={{ display: 'flex', gap: 12, alignItems: 'flex-start' }}
                  >
                    <span
                      style={{
                        fontWeight: 800,
                        fontSize: '1.5rem',
                        color: 'var(--ppt-border-default, #e5e7eb)',
                        lineHeight: 1,
                        flexShrink: 0,
                        minWidth: 28,
                      }}
                    >
                      {idx + 1}
                    </span>
                    <div>
                      <div
                        style={{
                          fontWeight: 600,
                          fontSize: '0.9rem',
                          color: 'var(--ppt-fg-primary)',
                          lineHeight: 1.4,
                        }}
                      >
                        {article.title}
                      </div>
                      <div
                        style={{
                          fontSize: '0.8rem',
                          color: 'var(--ppt-fg-muted, #9ca3af)',
                          marginTop: 4,
                        }}
                      >
                        {t('readingTime', { min: article.readMin })}
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </aside>
          )}
        </div>
      </main>

      <Footer />

      <style jsx>{`
        /* Two-column layout above 1024 px; stacks to a single column below
           so the 320 px sidebar doesn't force horizontal overflow on
           mobile (was +297 px on 375 px viewport before this rule). */
        .journal-grid {
          display: grid;
          grid-template-columns: 1fr;
          gap: 40px;
          align-items: start;
        }
        @media (min-width: 1024px) {
          .journal-grid {
            grid-template-columns: 1fr 320px;
          }
        }
      `}</style>
    </div>
  );
}
