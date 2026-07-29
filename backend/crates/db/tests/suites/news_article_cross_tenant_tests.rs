//! Regression tests for issue #2314 — cross-tenant IDOR + data loss in the
//! news-article repository.
//!
//! The api-server handlers used to extract `TenantExtractor` and then discard
//! it, running every op through `NewsArticleRepository` methods that queried by
//! path UUID alone (no `organization_id` predicate). `news_articles` only
//! ENABLEs (not FORCEs) RLS and api-server connects with a BYPASSRLS role, so
//! the tenant-isolation policy is inert on this path — meaning a caller in org A
//! could read, mutate, or permanently delete an org-B article/media/comment by
//! guessing its UUID.
//!
//! The fix threads the caller's `organization_id` into every by-UUID method and
//! adds an explicit `organization_id` predicate (directly on `news_articles`, or
//! via an `EXISTS` on the parent article for the child tables). CI runs these
//! repo tests against a superuser pool that bypasses FORCE RLS, so the SQL
//! predicate — not RLS — is exactly what these tests exercise: pre-fix the
//! cross-tenant op succeeds; post-fix it resolves to `None` / `false` and the
//! victim row is untouched.

use crate::common::seed_org;
use db::models::news_article::{CreateArticle, CreateArticleComment, CreateArticleMedia};
use db::repositories::NewsArticleRepository;
use sqlx::PgPool;
use uuid::Uuid;

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'News User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

fn sample_article(title: &str) -> CreateArticle {
    CreateArticle {
        title: title.to_string(),
        content: "Body content".to_string(),
        excerpt: None,
        cover_image_url: None,
        building_ids: Vec::new(),
        status: Some("published".to_string()),
        published_at: None,
        comments_enabled: true,
        reactions_enabled: true,
    }
}

fn sample_media() -> CreateArticleMedia {
    CreateArticleMedia {
        media_type: "image".to_string(),
        file_key: Some("k/1".to_string()),
        file_name: Some("photo.jpg".to_string()),
        file_size: Some(1234),
        mime_type: Some("image/jpeg".to_string()),
        embed_url: None,
        embed_html: None,
        width: Some(640),
        height: Some(480),
        alt_text: None,
        caption: None,
        display_order: Some(0),
    }
}

/// The core regression: an org-A caller must not be able to read, mutate, or
/// delete an org-B article by its UUID.
#[sqlx::test]
async fn org_a_cannot_read_mutate_or_delete_org_b_article(pool: PgPool) {
    let org_a = seed_org(&pool, "news-a").await;
    let org_b = seed_org(&pool, "news-b").await;
    let author_b = seed_user(&pool, "author-b@news.test").await;

    let repo = NewsArticleRepository::new(pool.clone());

    // Org B owns an article.
    let article_b = repo
        .create(org_b, author_b, sample_article("Org B secret"))
        .await
        .expect("create org B article");

    // --- Reads are org-scoped ---
    assert!(
        repo.find_by_id(article_b.id, org_a)
            .await
            .expect("find_by_id query")
            .is_none(),
        "org A must not read org B's article via find_by_id"
    );
    assert!(
        repo.find_by_id_with_details(article_b.id, org_a)
            .await
            .expect("find_by_id_with_details query")
            .is_none(),
        "org A must not read org B's article via find_by_id_with_details"
    );

    // --- Mutations are org-scoped (return None, leave the row untouched) ---
    let update = db::models::news_article::UpdateArticle {
        title: Some("HACKED".to_string()),
        content: None,
        excerpt: None,
        cover_image_url: None,
        building_ids: None,
        status: None,
        comments_enabled: None,
        reactions_enabled: None,
    };
    assert!(
        repo.update(article_b.id, org_a, update)
            .await
            .expect("update query")
            .is_none(),
        "org A update of org B article must affect no row"
    );
    assert!(
        repo.publish(article_b.id, org_a, None)
            .await
            .expect("publish query")
            .is_none(),
        "org A publish of org B article must affect no row"
    );
    assert!(
        repo.archive(article_b.id, org_a)
            .await
            .expect("archive query")
            .is_none(),
        "org A archive of org B article must affect no row"
    );
    assert!(
        repo.restore(article_b.id, org_a)
            .await
            .expect("restore query")
            .is_none(),
        "org A restore of org B article must affect no row"
    );
    assert!(
        repo.set_pinned(article_b.id, org_a, true, None)
            .await
            .expect("set_pinned query")
            .is_none(),
        "org A pin of org B article must affect no row"
    );

    // --- The data-loss vector: delete must NOT touch a foreign article ---
    assert!(
        !repo
            .delete(article_b.id, org_a)
            .await
            .expect("delete query"),
        "org A delete of org B article must report no rows affected"
    );

    // The victim article survived every cross-tenant attempt, unchanged.
    let survivor = repo
        .find_by_id(article_b.id, org_b)
        .await
        .expect("find survivor")
        .expect("org B article must still exist");
    assert_eq!(survivor.title, "Org B secret", "title must be untouched");
    assert!(!survivor.pinned, "pin state must be untouched");
}

/// The owning org still gets full access — the fix scopes, it does not break.
#[sqlx::test]
async fn owning_org_retains_full_access(pool: PgPool) {
    let org_a = seed_org(&pool, "news-owner").await;
    let author = seed_user(&pool, "author@news-owner.test").await;
    let repo = NewsArticleRepository::new(pool.clone());

    let article = repo
        .create(org_a, author, sample_article("Mine"))
        .await
        .expect("create");

    assert!(repo
        .find_by_id(article.id, org_a)
        .await
        .expect("find")
        .is_some());

    let pinned = repo
        .set_pinned(article.id, org_a, true, Some(author))
        .await
        .expect("pin query")
        .expect("owning org can pin its own article");
    assert!(pinned.pinned);

    assert!(
        repo.delete(article.id, org_a).await.expect("delete query"),
        "owning org can delete its own article"
    );
}

/// Media items carry no `organization_id` of their own; the fix guards them via
/// an `EXISTS` on the parent article, so cross-tenant add/delete/list are inert.
#[sqlx::test]
async fn media_is_org_scoped_via_parent_article(pool: PgPool) {
    let org_a = seed_org(&pool, "news-media-a").await;
    let org_b = seed_org(&pool, "news-media-b").await;
    let author_b = seed_user(&pool, "author-b@news-media.test").await;
    let repo = NewsArticleRepository::new(pool.clone());

    let article_b = repo
        .create(org_b, author_b, sample_article("B with media"))
        .await
        .expect("create org B article");

    // Org A cannot attach media to org B's article.
    assert!(
        repo.add_media(article_b.id, org_a, sample_media())
            .await
            .expect("add_media query")
            .is_none(),
        "org A must not attach media to org B's article"
    );

    // Org B attaches its own media.
    let media_b = repo
        .add_media(article_b.id, org_b, sample_media())
        .await
        .expect("add_media query")
        .expect("org B can attach media to its own article");

    // Org A cannot see or delete org B's media.
    assert!(
        repo.list_media(article_b.id, org_a)
            .await
            .expect("list_media query")
            .is_empty(),
        "org A must not list org B's media"
    );
    assert!(
        !repo
            .delete_media(media_b.id, org_a)
            .await
            .expect("delete_media query"),
        "org A must not delete org B's media"
    );

    // Org B still sees and can delete its own media.
    assert_eq!(
        repo.list_media(article_b.id, org_b)
            .await
            .expect("list_media query")
            .len(),
        1
    );
    assert!(
        repo.delete_media(media_b.id, org_b)
            .await
            .expect("delete_media query"),
        "org B can delete its own media"
    );
}

/// Comment moderation is manager-only but was keyed on `comment_id` alone; the
/// fix scopes it to the parent article's org.
#[sqlx::test]
async fn moderate_comment_is_org_scoped(pool: PgPool) {
    let org_a = seed_org(&pool, "news-mod-a").await;
    let org_b = seed_org(&pool, "news-mod-b").await;
    let author_b = seed_user(&pool, "author-b@news-mod.test").await;
    let commenter_b = seed_user(&pool, "commenter-b@news-mod.test").await;
    let manager_a = seed_user(&pool, "manager-a@news-mod.test").await;
    let repo = NewsArticleRepository::new(pool.clone());

    let article_b = repo
        .create(org_b, author_b, sample_article("B commentable"))
        .await
        .expect("create org B article");
    let comment_b = repo
        .add_comment(
            article_b.id,
            commenter_b,
            CreateArticleComment {
                content: "hello".to_string(),
                parent_id: None,
            },
        )
        .await
        .expect("add comment");

    // A manager in org A cannot moderate a comment on org B's article.
    assert!(
        repo.moderate_comment(
            comment_b.id,
            manager_a,
            org_a,
            true,
            Some("spam".to_string())
        )
        .await
        .expect("moderate query")
        .is_none(),
        "org A manager must not moderate org B's comment"
    );

    // Same manager acting within org B's scope succeeds.
    let moderated = repo
        .moderate_comment(
            comment_b.id,
            manager_a,
            org_b,
            true,
            Some("spam".to_string()),
        )
        .await
        .expect("moderate query")
        .expect("moderation within the article's org succeeds");
    assert!(moderated.is_moderated);
}

/// Aggregate statistics must be per-org; previously the query summed across ALL
/// organizations.
#[sqlx::test]
async fn statistics_are_org_scoped(pool: PgPool) {
    let org_a = seed_org(&pool, "news-stats-a").await;
    let org_b = seed_org(&pool, "news-stats-b").await;
    let author_b = seed_user(&pool, "author-b@news-stats.test").await;
    let repo = NewsArticleRepository::new(pool.clone());

    repo.create(org_b, author_b, sample_article("B one"))
        .await
        .expect("create");
    repo.create(org_b, author_b, sample_article("B two"))
        .await
        .expect("create");

    let stats_a = repo.get_statistics(org_a).await.expect("stats A");
    assert_eq!(
        stats_a.total_articles, 0,
        "org A must not see org B's articles in its statistics"
    );

    let stats_b = repo.get_statistics(org_b).await.expect("stats B");
    assert_eq!(
        stats_b.total_articles, 2,
        "org B counts only its own articles"
    );
}
