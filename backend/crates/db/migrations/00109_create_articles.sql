-- Reality Portal: journal/news reality_articles and comments (UC-13).
--
-- Articles are authored by portal team users; comments are posted by
-- authenticated portal users.

CREATE TABLE IF NOT EXISTS reality_articles (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    -- URL-safe slug, globally unique.
    slug            TEXT        NOT NULL UNIQUE,
    title           TEXT        NOT NULL,
    -- Full HTML/Markdown body.
    body            TEXT        NOT NULL,
    excerpt         TEXT,
    category        TEXT        NOT NULL DEFAULT 'news',
    -- NULL = editorial team / anonymous author.
    author_user_id  UUID        REFERENCES portal_users(id) ON DELETE SET NULL,
    cover_image_url TEXT,
    -- JSON array of tag strings.
    tags            JSONB,
    -- NULL = draft (not visible on public API).
    published_at    TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_reality_articles_slug
    ON reality_articles(slug);

CREATE INDEX IF NOT EXISTS idx_reality_articles_published
    ON reality_articles(published_at DESC)
    WHERE published_at IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_reality_articles_category
    ON reality_articles(category);

-- Article comments (one level deep, no threading).
CREATE TABLE IF NOT EXISTS reality_article_comments (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    article_id      UUID        NOT NULL REFERENCES reality_articles(id) ON DELETE CASCADE,
    author_user_id  UUID        NOT NULL REFERENCES portal_users(id) ON DELETE CASCADE,
    body            TEXT        NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_reality_article_comments_article
    ON reality_article_comments(article_id);

CREATE INDEX IF NOT EXISTS idx_reality_article_comments_author
    ON reality_article_comments(author_user_id);

-- Auto-update reality_articles.updated_at.
CREATE OR REPLACE FUNCTION update_reality_articles_updated_at()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$;

CREATE TRIGGER trg_reality_articles_updated_at
    BEFORE UPDATE ON reality_articles
    FOR EACH ROW EXECUTE FUNCTION update_reality_articles_updated_at();
