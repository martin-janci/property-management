//! End-to-end DB verification for `ComplianceRepository::resolve_content_owner`
//! (PAP-38 / PAP-48).
//!
//! `report_content` (`api-server/src/routes/aml_dsa.rs`) used to persist a
//! `Uuid::new_v4()` placeholder as the moderated content's owner. PAP-38
//! replaced that with a real owner lookup via `resolve_content_owner`, whose
//! per-type SQL is runtime `query_as` (NOT compile-checked) — so it needs a
//! live-DB test to prove the columns/joins are valid and the owner is real.
//!
//! These tests assert, for every moderatable content type:
//!   1. a seeded piece of content resolves to its **real** owner id, and
//!   2. an unknown `content_id` resolves to `None` (the handler maps this to a
//!      404 *before* any moderation row is written — no bogus-owner rows), and
//!   3. content owned by org A is **not** resolvable when the caller's tenant
//!      is org B (the explicit `organization_id = $2` tenant filter — PAP-48).
//!
//! Tenant scoping note (PAP-48): the four content types whose tables have no
//! direct `organization_id` column — Message, CommunityPost, Comment,
//! UserProfile — are scoped here by joining to their org-bearing parent
//! (`message_threads`, `community_groups`->`buildings`) or, for UserProfile,
//! by requiring an active `organization_members` row. Before PAP-48 those four
//! branches ignored the passed `organization_id` entirely.
//!
//! `#[sqlx::test]` connects as the Postgres superuser, which bypasses RLS — so
//! these tests exercise the *query logic and the explicit tenant filter* in
//! isolation. RLS-enforcement of the same path under the production owner role
//! is a separate concern tracked alongside this work.

use db::models::ModeratedContentType;
use db::repositories::ComplianceRepository;
use sqlx::PgPool;
use uuid::Uuid;

/// Mirror `db::tenant_context::set_request_context`. Seeding `organizations`
/// fires a roles-trigger with an RLS `WITH CHECK`; super-admin context
/// satisfies it.
async fn set_ctx(pool: &PgPool, org_id: Option<Uuid>, user_id: Option<Uuid>, is_super_admin: bool) {
    sqlx::query("SELECT set_request_context($1, $2, $3)")
        .bind(org_id)
        .bind(user_id)
        .bind(is_super_admin)
        .execute(pool)
        .await
        .expect("set_request_context");
}

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO organizations (name, slug, contact_email, status)
           VALUES ($1, $2, $3, 'active') RETURNING id"#,
    )
    .bind(format!("Mod {slug}"))
    .bind(slug)
    .bind(format!("{slug}@mod.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
           VALUES ($1, 'test_hash', 'Mod User', 'active', NOW(), 'public') RETURNING id"#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Make `user` an active member of `org` (required for UserProfile scoping).
async fn seed_membership(pool: &PgPool, org: Uuid, user: Uuid) {
    sqlx::query(
        r#"INSERT INTO organization_members (organization_id, user_id, role_type, status)
           VALUES ($1, $2, 'member', 'active')"#,
    )
    .bind(org)
    .bind(user)
    .execute(pool)
    .await
    .expect("seed membership");
}

async fn seed_building(pool: &PgPool, org: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, name, street, city, postal_code, country)
           VALUES ($1, 'Mod Bldg', 'St', 'City', '00000', 'SK') RETURNING id"#,
    )
    .bind(org)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

async fn seed_listing(pool: &PgPool, org: Uuid, owner: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO listings (organization_id, created_by, transaction_type, title,
               property_type, street, city, postal_code, country, price)
           VALUES ($1, $2, 'sale', 'L', 'apartment', 'St', 'City', '00000', 'SK', 100000)
           RETURNING id"#,
    )
    .bind(org)
    .bind(owner)
    .fetch_one(pool)
    .await
    .expect("seed listing")
}

async fn seed_listing_photo(pool: &PgPool, listing: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO listing_photos (listing_id, url) VALUES ($1, 'https://x/p.jpg')
           RETURNING id"#,
    )
    .bind(listing)
    .fetch_one(pool)
    .await
    .expect("seed listing_photo")
}

async fn seed_announcement(pool: &PgPool, org: Uuid, author: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO announcements (organization_id, author_id, title, content)
           VALUES ($1, $2, 'A', 'body') RETURNING id"#,
    )
    .bind(org)
    .bind(author)
    .fetch_one(pool)
    .await
    .expect("seed announcement")
}

async fn seed_document(pool: &PgPool, org: Uuid, owner: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO documents (organization_id, title, category, file_key, file_name,
               mime_type, size_bytes, created_by)
           VALUES ($1, 'D', 'other', 's3/d.pdf', 'd.pdf', 'application/pdf', 1024, $2)
           RETURNING id"#,
    )
    .bind(org)
    .bind(owner)
    .fetch_one(pool)
    .await
    .expect("seed document")
}

/// Seed a message in `org` sent by `sender`; returns the message id.
async fn seed_message(pool: &PgPool, org: Uuid, sender: Uuid, other: Uuid) -> Uuid {
    let thread = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO message_threads (organization_id, participant_ids)
           VALUES ($1, ARRAY[$2, $3]::uuid[]) RETURNING id"#,
    )
    .bind(org)
    .bind(sender)
    .bind(other)
    .fetch_one(pool)
    .await
    .expect("seed thread");
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO messages (thread_id, sender_id, content) VALUES ($1, $2, 'hi')
           RETURNING id"#,
    )
    .bind(thread)
    .bind(sender)
    .fetch_one(pool)
    .await
    .expect("seed message")
}

/// Seed a community group/post/comment chain rooted in `org`'s building.
/// Returns `(post_id, comment_id)`, both authored by `author`.
async fn seed_community(pool: &PgPool, building: Uuid, author: Uuid) -> (Uuid, Uuid) {
    let group = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO community_groups (building_id, name, slug) VALUES ($1, 'G', $2)
           RETURNING id"#,
    )
    .bind(building)
    .bind(format!("g-{}", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed group");
    let post = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO community_posts (group_id, author_id, content) VALUES ($1, $2, 'p')
           RETURNING id"#,
    )
    .bind(group)
    .bind(author)
    .fetch_one(pool)
    .await
    .expect("seed post");
    let comment = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO community_comments (post_id, author_id, content) VALUES ($1, $2, 'c')
           RETURNING id"#,
    )
    .bind(post)
    .bind(author)
    .fetch_one(pool)
    .await
    .expect("seed comment");
    (post, comment)
}

/// All resolvable types for org A, seeded and owned by a known user, must
/// resolve to that real owner id — never a placeholder.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resolve_content_owner_returns_real_owner_for_every_type(pool: PgPool) {
    set_ctx(&pool, None, None, true).await;

    let org = seed_org(&pool, "owner-a").await;
    let owner = seed_user(&pool, "owner@mod.test").await;
    let other = seed_user(&pool, "other@mod.test").await;
    seed_membership(&pool, org, owner).await;
    let building = seed_building(&pool, org).await;

    let listing = seed_listing(&pool, org, owner).await;
    let photo = seed_listing_photo(&pool, listing).await;
    let announcement = seed_announcement(&pool, org, owner).await;
    let document = seed_document(&pool, org, owner).await;
    let message = seed_message(&pool, org, owner, other).await;
    let (post, comment) = seed_community(&pool, building, owner).await;

    let repo = ComplianceRepository::new(pool.clone());
    let scope = Some(org);

    let cases = [
        (ModeratedContentType::Listing, listing),
        (ModeratedContentType::ListingPhoto, photo),
        (ModeratedContentType::Announcement, announcement),
        (ModeratedContentType::Document, document),
        (ModeratedContentType::Message, message),
        (ModeratedContentType::CommunityPost, post),
        (ModeratedContentType::Comment, comment),
        (ModeratedContentType::UserProfile, owner),
    ];

    for (content_type, content_id) in cases {
        let resolved = repo
            .resolve_content_owner(content_type, content_id, scope)
            .await
            .expect("resolve_content_owner should not error");
        assert_eq!(
            resolved,
            Some(owner),
            "{content_type:?} must resolve to the real owner, got {resolved:?}"
        );
    }
}

/// An unknown `content_id` resolves to `None` for every type. The handler turns
/// this into a 404 *before* writing a moderation case — so no row is ever
/// persisted with a bogus owner (test-plan step 3).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resolve_content_owner_unknown_id_is_none(pool: PgPool) {
    set_ctx(&pool, None, None, true).await;
    let org = seed_org(&pool, "unknown-a").await;
    let repo = ComplianceRepository::new(pool.clone());
    let bogus = Uuid::new_v4();

    for content_type in [
        ModeratedContentType::Listing,
        ModeratedContentType::ListingPhoto,
        ModeratedContentType::Announcement,
        ModeratedContentType::Document,
        ModeratedContentType::Message,
        ModeratedContentType::CommunityPost,
        ModeratedContentType::Comment,
        ModeratedContentType::UserProfile,
        ModeratedContentType::Review, // no backing table -> always None
    ] {
        let resolved = repo
            .resolve_content_owner(content_type, bogus, Some(org))
            .await
            .expect("resolve_content_owner should not error");
        assert_eq!(resolved, None, "{content_type:?} unknown id must be None");
    }
}

/// Content owned by org A must NOT be resolvable when the caller's tenant is
/// org B — for every type, including the four (Message/CommunityPost/Comment/
/// UserProfile) that ignored `organization_id` before PAP-48 (test-plan step 4).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resolve_content_owner_rejects_cross_tenant(pool: PgPool) {
    set_ctx(&pool, None, None, true).await;

    let org_a = seed_org(&pool, "xt-a").await;
    let org_b = seed_org(&pool, "xt-b").await;
    let owner = seed_user(&pool, "xt-owner@mod.test").await;
    let other = seed_user(&pool, "xt-other@mod.test").await;
    // `owner` is a member of org A only.
    seed_membership(&pool, org_a, owner).await;
    let building_a = seed_building(&pool, org_a).await;

    let listing = seed_listing(&pool, org_a, owner).await;
    let photo = seed_listing_photo(&pool, listing).await;
    let announcement = seed_announcement(&pool, org_a, owner).await;
    let document = seed_document(&pool, org_a, owner).await;
    let message = seed_message(&pool, org_a, owner, other).await;
    let (post, comment) = seed_community(&pool, building_a, owner).await;

    let repo = ComplianceRepository::new(pool.clone());
    let attacker_scope = Some(org_b);

    let cases = [
        (ModeratedContentType::Listing, listing),
        (ModeratedContentType::ListingPhoto, photo),
        (ModeratedContentType::Announcement, announcement),
        (ModeratedContentType::Document, document),
        (ModeratedContentType::Message, message),
        (ModeratedContentType::CommunityPost, post),
        (ModeratedContentType::Comment, comment),
        (ModeratedContentType::UserProfile, owner),
    ];

    for (content_type, content_id) in cases {
        let resolved = repo
            .resolve_content_owner(content_type, content_id, attacker_scope)
            .await
            .expect("resolve_content_owner should not error");
        assert_eq!(
            resolved, None,
            "{content_type:?} from org A must NOT resolve for an org-B caller (got {resolved:?})"
        );

        // ...but the same lookup under org A's scope still succeeds, proving the
        // None above is tenant scoping, not a broken query.
        let same_tenant = repo
            .resolve_content_owner(content_type, content_id, Some(org_a))
            .await
            .expect("resolve_content_owner should not error");
        assert_eq!(
            same_tenant,
            Some(owner),
            "{content_type:?} must still resolve for its own tenant"
        );
    }
}
