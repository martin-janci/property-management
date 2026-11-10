//! Community repository (Epic 37).
//!
//! Repository for community groups, posts, events, and marketplace.

use crate::models::community::*;
use crate::DbPool;
use sqlx::{Error as SqlxError, Row};
use uuid::Uuid;

// Explicit column lists (compile-time `&str` constants spliced with
// `format!`, never user input — hence `AssertSqlSafe`). `group_type`, `visibility`, `post_type`,
// `event_type`, RSVP `status`, `category` and `condition` are Postgres enum
// columns, but the models carry them as `String`; sqlx refuses to decode an
// enum OID into `String` (`mismatched types`), so every read casts them
// `::text`. Writes cast the bound text params to the enum type (`$n::type`),
// since a `TEXT`-typed parameter is not implicitly assignable to an enum.

/// `community_groups` columns for [`CommunityGroup`].
const GROUP_COLUMNS: &str = "id, building_id, name, slug, description, \
    group_type::text AS group_type, visibility::text AS visibility, cover_image_url, icon, \
    color, rules, max_members, auto_join_new_residents, requires_approval, is_official, \
    member_count, post_count, created_by, created_at, updated_at";

/// `community_posts` columns for [`CommunityPost`].
const POST_COLUMNS: &str = "id, group_id, author_id, post_type::text AS post_type, title, \
    content, media_urls, poll_options, poll_ends_at, poll_multiple_choice, is_pinned, \
    is_locked, is_anonymous, view_count, like_count, comment_count, share_count, created_at, \
    updated_at, edited_at";

/// `community_events` columns for [`CommunityEvent`].
const EVENT_COLUMNS: &str = "id, group_id, building_id, organizer_id, title, description, \
    event_type::text AS event_type, location, location_details, is_virtual, virtual_link, \
    start_time, end_time, all_day, recurring_rule, cover_image_url, max_attendees, \
    requires_rsvp, rsvp_deadline, cost_per_person, cost_currency, is_public, status, \
    attendee_count, created_at, updated_at";

/// `community_event_rsvps` columns for [`CommunityEventRsvp`].
const RSVP_COLUMNS: &str = "id, event_id, user_id, status::text AS status, guests, note, \
    reminder_sent, created_at, updated_at";

/// `marketplace_items` columns for [`MarketplaceItem`].
const ITEM_COLUMNS: &str = "id, building_id, seller_id, title, description, \
    category::text AS category, condition::text AS condition, price, currency, is_free, \
    is_negotiable, is_trade_accepted, photo_urls, location, pickup_details, view_count, \
    inquiry_count, status, sold_to, sold_at, expires_at, created_at, updated_at";

/// Repository for community operations.
#[derive(Clone)]
pub struct CommunityRepository {
    pool: DbPool,
}

impl CommunityRepository {
    /// Create a new CommunityRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // ========================================================================
    // Community Groups (Story 37.1)
    // ========================================================================

    /// Create a new community group.
    pub async fn create_group(
        &self,
        building_id: Uuid,
        created_by: Uuid,
        data: CreateCommunityGroup,
    ) -> Result<CommunityGroup, SqlxError> {
        // Generate base slug from name
        let base_slug: String = data
            .name
            .to_lowercase()
            .replace(' ', "-")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-')
            .collect();

        // Use a CTE to generate unique slug and atomically insert group + add creator as owner
        let id = sqlx::query_scalar::<_, Uuid>(
            r#"
            WITH unique_slug AS (
                SELECT CASE
                    WHEN NOT EXISTS (SELECT 1 FROM community_groups WHERE building_id = $1 AND slug = $3)
                    THEN $3
                    ELSE $3 || '-' || (
                        SELECT COALESCE(MAX(
                            CASE WHEN slug ~ ('^' || $3 || '-[0-9]+$')
                            THEN CAST(SUBSTRING(slug FROM LENGTH($3) + 2) AS INTEGER)
                            ELSE 0 END
                        ), 0) + 1
                        FROM community_groups
                        WHERE building_id = $1 AND slug LIKE $3 || '%'
                    )::text
                END AS slug
            ),
            new_group AS (
                INSERT INTO community_groups (
                    building_id, name, slug, description, group_type, visibility,
                    icon, color, rules, max_members, auto_join_new_residents,
                    requires_approval, created_by
                )
                SELECT $1, $2, unique_slug.slug, $4,
                       COALESCE($5::community_group_type, 'interest'),
                       COALESCE($6::community_group_visibility, 'public'),
                       $7, $8, $9, $10, COALESCE($11, false), COALESCE($12, false), $13
                FROM unique_slug
                RETURNING id
            ),
            add_owner AS (
                INSERT INTO community_group_members (group_id, user_id, role)
                SELECT id, $13, 'owner' FROM new_group
            )
            SELECT id FROM new_group
            "#,
        )
            .bind(building_id)
            .bind(&data.name)
            .bind(&base_slug)
            .bind(&data.description)
            .bind(&data.group_type)
            .bind(&data.visibility)
            .bind(&data.icon)
            .bind(&data.color)
            .bind(&data.rules)
            .bind(data.max_members)
            .bind(data.auto_join_new_residents)
            .bind(data.requires_approval)
            .bind(created_by)
            .fetch_one(&self.pool)
            .await?;

        // Re-read after the statement so the response reflects the owner
        // membership counted by `trg_group_member_count` (a data-modifying
        // CTE cannot see the trigger's update within the same statement).
        self.get_group(id).await?.ok_or(SqlxError::RowNotFound)
    }

    /// Get community group by ID.
    pub async fn get_group(&self, id: Uuid) -> Result<Option<CommunityGroup>, SqlxError> {
        let sql = format!("SELECT {GROUP_COLUMNS} FROM community_groups WHERE id = $1");
        sqlx::query_as::<_, CommunityGroup>(sqlx::AssertSqlSafe(sql))
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    /// Get a group plus `user_id`'s membership. Returns `None` when the group
    /// is absent, or when it is `secret` and `user_id` is not a member (secret
    /// groups are not discoverable by non-members). Tenant scoping is the
    /// caller's responsibility (`group_exists_for_org`).
    pub async fn get_group_detail(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<CommunityGroupDetail>, SqlxError> {
        let Some(group) = self.get_group(id).await? else {
            return Ok(None);
        };
        let my_role = sqlx::query_scalar::<_, String>(
            "SELECT role::text FROM community_group_members WHERE group_id = $1 AND user_id = $2",
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;
        if group.visibility == "secret" && my_role.is_none() {
            return Ok(None);
        }
        Ok(Some(CommunityGroupDetail {
            group,
            is_member: my_role.is_some(),
            my_role,
        }))
    }

    /// `true` unless the group is `secret` and `user_id` is not a member.
    /// Absent groups return `true` here; existence/tenancy is checked
    /// separately by `group_exists_for_org`.
    pub async fn group_visible_to_user(
        &self,
        group_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, SqlxError> {
        sqlx::query_scalar::<_, bool>(
            r#"
            SELECT NOT EXISTS(
                SELECT 1 FROM community_groups g
                 WHERE g.id = $1
                   AND g.visibility = 'secret'
                   AND NOT EXISTS (
                       SELECT 1 FROM community_group_members m
                        WHERE m.group_id = g.id AND m.user_id = $2
                   )
            )
            "#,
        )
        .bind(group_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
    }

    /// `true` unless the post belongs to a `secret` group `user_id` is not a
    /// member of.
    pub async fn post_visible_to_user(
        &self,
        post_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, SqlxError> {
        sqlx::query_scalar::<_, bool>(
            r#"
            SELECT NOT EXISTS(
                SELECT 1 FROM community_posts p
                  JOIN community_groups g ON g.id = p.group_id
                 WHERE p.id = $1
                   AND g.visibility = 'secret'
                   AND NOT EXISTS (
                       SELECT 1 FROM community_group_members m
                        WHERE m.group_id = g.id AND m.user_id = $2
                   )
            )
            "#,
        )
        .bind(post_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
    }

    /// `true` when `group_id` belongs to `building_id`.
    pub async fn group_in_building(
        &self,
        group_id: Uuid,
        building_id: Uuid,
    ) -> Result<bool, SqlxError> {
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM community_groups WHERE id = $1 AND building_id = $2)",
        )
        .bind(group_id)
        .bind(building_id)
        .fetch_one(&self.pool)
        .await
    }

    /// `true` when comment `comment_id` belongs to post `post_id`.
    pub async fn comment_in_post(
        &self,
        comment_id: Uuid,
        post_id: Uuid,
    ) -> Result<bool, SqlxError> {
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM community_comments WHERE id = $1 AND post_id = $2)",
        )
        .bind(comment_id)
        .bind(post_id)
        .fetch_one(&self.pool)
        .await
    }

    /// Tenant-safe group existence check: `true` only when the group belongs
    /// to `org_id` (resolved via `buildings.organization_id`). `false` when the
    /// group is absent OR owned by another organization, so handlers can map
    /// the miss to a 404 without leaking cross-tenant existence (mirrors the
    /// disputes IDOR fix — enforce the JWT-derived org, never a path-supplied
    /// id). Uses `EXISTS` (not a row fetch) so the check never has to decode a
    /// `CommunityGroup`, whose `group_type`/`visibility` are Postgres enum
    /// columns typed as `String` in the model.
    pub async fn group_exists_for_org(&self, id: Uuid, org_id: Uuid) -> Result<bool, SqlxError> {
        sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1
                FROM community_groups g
                JOIN buildings b ON b.id = g.building_id
                WHERE g.id = $1 AND b.organization_id = $2
            )
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_one(&self.pool)
        .await
    }

    /// List groups for a building.
    pub async fn list_groups(
        &self,
        building_id: Uuid,
        user_id: Option<Uuid>,
    ) -> Result<Vec<CommunityGroupWithMembership>, SqlxError> {
        let rows = sqlx::query(
            r#"
            SELECT g.id, g.building_id, g.name, g.slug, g.description,
                g.group_type::text AS group_type, g.visibility::text AS visibility,
                g.cover_image_url, g.icon, g.color, g.rules, g.max_members,
                g.auto_join_new_residents, g.requires_approval, g.is_official,
                g.member_count, g.post_count, g.created_by, g.created_at, g.updated_at,
                m.user_id IS NOT NULL as is_member,
                m.role::text as member_role,
                m.joined_at
            FROM community_groups g
            LEFT JOIN community_group_members m ON m.group_id = g.id AND m.user_id = $2
            WHERE g.building_id = $1
              AND (g.visibility != 'secret' OR m.user_id IS NOT NULL)
            ORDER BY g.is_official DESC, g.member_count DESC
            "#,
        )
        .bind(building_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let groups = rows
            .iter()
            .map(|row| CommunityGroupWithMembership {
                group: CommunityGroup {
                    id: row.get("id"),
                    building_id: row.get("building_id"),
                    name: row.get("name"),
                    slug: row.get("slug"),
                    description: row.get("description"),
                    group_type: row.get("group_type"),
                    visibility: row.get("visibility"),
                    cover_image_url: row.get("cover_image_url"),
                    icon: row.get("icon"),
                    color: row.get("color"),
                    rules: row.get("rules"),
                    max_members: row.get("max_members"),
                    auto_join_new_residents: row.get("auto_join_new_residents"),
                    requires_approval: row.get("requires_approval"),
                    is_official: row.get("is_official"),
                    member_count: row.get("member_count"),
                    post_count: row.get("post_count"),
                    created_by: row.get("created_by"),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                },
                is_member: row.get("is_member"),
                member_role: row.get("member_role"),
                joined_at: row.get("joined_at"),
            })
            .collect();

        Ok(groups)
    }

    /// Join a group.
    pub async fn join_group(&self, group_id: Uuid, user_id: Uuid) -> Result<(), SqlxError> {
        sqlx::query(
            r#"
            INSERT INTO community_group_members (group_id, user_id, role)
            VALUES ($1, $2, 'member')
            ON CONFLICT (group_id, user_id) DO NOTHING
            "#,
        )
        .bind(group_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Leave a group. Refuses (`LastOwner`) when the caller is the group's
    /// only `owner`, so a group can never be left without an owner. The group
    /// row is locked `FOR UPDATE` so two owners leaving concurrently cannot
    /// both pass the "another owner remains" check.
    pub async fn leave_group(
        &self,
        group_id: Uuid,
        user_id: Uuid,
    ) -> Result<LeaveGroupOutcome, SqlxError> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT 1 FROM community_groups WHERE id = $1 FOR UPDATE")
            .bind(group_id)
            .fetch_optional(&mut *tx)
            .await?;
        let role = sqlx::query_scalar::<_, String>(
            "SELECT role::text FROM community_group_members WHERE group_id = $1 AND user_id = $2",
        )
        .bind(group_id)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(role) = role else {
            return Ok(LeaveGroupOutcome::NotMember);
        };
        if role == "owner" {
            let other_owner = sqlx::query_scalar::<_, bool>(
                r#"
                SELECT EXISTS(
                    SELECT 1 FROM community_group_members
                     WHERE group_id = $1 AND role = 'owner' AND user_id <> $2
                )
                "#,
            )
            .bind(group_id)
            .bind(user_id)
            .fetch_one(&mut *tx)
            .await?;
            if !other_owner {
                return Ok(LeaveGroupOutcome::LastOwner);
            }
        }
        sqlx::query("DELETE FROM community_group_members WHERE group_id = $1 AND user_id = $2")
            .bind(group_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(LeaveGroupOutcome::Left)
    }

    /// `true` when `user_id` owns `group_id` — either they created it
    /// (`community_groups.created_by`) or they hold an `owner`-role membership
    /// row. This answers "is this user an owner of the group?" only; tenant
    /// scoping is the caller's responsibility (via `group_exists_for_org`), and
    /// org-manager override is decided in the handler. Deletion is destructive
    /// and cascading, so the owner check is enforced server-side rather than
    /// trusting a client-supplied `isOwner` flag.
    pub async fn user_owns_group(&self, group_id: Uuid, user_id: Uuid) -> Result<bool, SqlxError> {
        sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM community_groups
                 WHERE id = $1 AND created_by = $2
                UNION ALL
                SELECT 1 FROM community_group_members
                 WHERE group_id = $1 AND user_id = $2 AND role = 'owner'
            )
            "#,
        )
        .bind(group_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
    }

    /// Delete a community group scoped to `org_id` (resolved via
    /// `buildings.organization_id`), returning the number of rows removed — `0`
    /// when the group is absent or belongs to another tenant, so a cross-tenant
    /// delete cannot silently succeed even if the handler's access check were
    /// bypassed (defense in depth). Members, posts, post reactions, comments and
    /// event RSVPs cascade via the `ON DELETE CASCADE` foreign keys declared in
    /// migration 00064; community events reference the group with
    /// `ON DELETE SET NULL` and are intentionally preserved.
    pub async fn delete_group_for_org(
        &self,
        group_id: Uuid,
        org_id: Uuid,
    ) -> Result<u64, SqlxError> {
        let result = sqlx::query(
            r#"
            DELETE FROM community_groups g
             USING buildings b
             WHERE g.id = $1
               AND g.building_id = b.id
               AND b.organization_id = $2
            "#,
        )
        .bind(group_id)
        .bind(org_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    // ========================================================================
    // Community Posts (Story 37.2)
    // ========================================================================

    /// Create a post.
    pub async fn create_post(
        &self,
        group_id: Uuid,
        author_id: Uuid,
        data: CreateCommunityPost,
    ) -> Result<CommunityPost, SqlxError> {
        let poll_options_json = data.poll_options.map(|opts| {
            serde_json::to_value(
                opts.into_iter()
                    .enumerate()
                    .map(|(i, o)| {
                        serde_json::json!({
                            "id": (i + 1).to_string(),
                            "text": o.text,
                            "votes": 0
                        })
                    })
                    .collect::<Vec<_>>(),
            )
            .unwrap_or_default()
        });

        let sql = format!(
            r#"
            INSERT INTO community_posts (
                group_id, author_id, post_type, title, content, media_urls,
                poll_options, poll_ends_at, poll_multiple_choice, is_anonymous
            )
            VALUES ($1, $2, COALESCE($3::community_post_type, 'text'), $4, $5, $6, $7, $8, $9,
                    COALESCE($10, false))
            RETURNING {POST_COLUMNS}
            "#
        );
        sqlx::query_as::<_, CommunityPost>(sqlx::AssertSqlSafe(sql))
            .bind(group_id)
            .bind(author_id)
            .bind(&data.post_type)
            .bind(&data.title)
            .bind(&data.content)
            .bind(&data.media_urls)
            .bind(&poll_options_json)
            .bind(data.poll_ends_at)
            .bind(data.poll_multiple_choice)
            .bind(data.is_anonymous)
            .fetch_one(&self.pool)
            .await
    }

    /// Get posts for a group.
    pub async fn get_group_posts(
        &self,
        group_id: Uuid,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<CommunityPost>, SqlxError> {
        let sql = format!(
            r#"
            SELECT {POST_COLUMNS} FROM community_posts
            WHERE group_id = $1
            ORDER BY is_pinned DESC, created_at DESC
            LIMIT $2 OFFSET $3
            "#
        );
        sqlx::query_as::<_, CommunityPost>(sqlx::AssertSqlSafe(sql))
            .bind(group_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
    }

    /// Tenant-safe post existence check: `true` only when the post's group's
    /// building belongs to `org_id`. `false` when the post is absent OR owned
    /// by another organization. `EXISTS` avoids decoding `CommunityPost`
    /// (`post_type` is a Postgres enum typed as `String`).
    pub async fn post_exists_for_org(&self, id: Uuid, org_id: Uuid) -> Result<bool, SqlxError> {
        sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1
                FROM community_posts p
                JOIN community_groups g ON g.id = p.group_id
                JOIN buildings b ON b.id = g.building_id
                WHERE p.id = $1 AND b.organization_id = $2
            )
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_one(&self.pool)
        .await
    }

    /// Add reaction to post.
    pub async fn add_post_reaction(
        &self,
        post_id: Uuid,
        user_id: Uuid,
        reaction_type: &str,
    ) -> Result<(), SqlxError> {
        // Insert + counter bump in one statement. The counter is incremented
        // rather than recounted: a `COUNT(*)` subquery runs on the statement's
        // snapshot, which cannot see the row the CTE just inserted, so a
        // recount always lagged one reaction behind (first like -> 0). The
        // row-locking `like_count + 1` is also safe under concurrent inserts.
        sqlx::query(
            r#"
            WITH inserted AS (
                INSERT INTO community_post_reactions (post_id, user_id, reaction_type)
                VALUES ($1, $2, $3)
                ON CONFLICT (post_id, user_id, reaction_type) DO NOTHING
                RETURNING post_id
            )
            UPDATE community_posts
            SET like_count = like_count + 1
            WHERE id = $1 AND EXISTS (SELECT 1 FROM inserted)
            "#,
        )
        .bind(post_id)
        .bind(user_id)
        .bind(reaction_type)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Create comment.
    pub async fn create_comment(
        &self,
        post_id: Uuid,
        author_id: Uuid,
        data: CreateCommunityComment,
    ) -> Result<CommunityComment, SqlxError> {
        sqlx::query_as::<_, CommunityComment>(
            r#"
            INSERT INTO community_comments (post_id, parent_id, author_id, content, is_anonymous)
            VALUES ($1, $2, $3, $4, COALESCE($5, false))
            RETURNING *
            "#,
        )
        .bind(post_id)
        .bind(data.parent_id)
        .bind(author_id)
        .bind(&data.content)
        .bind(data.is_anonymous)
        .fetch_one(&self.pool)
        .await
    }

    // ========================================================================
    // Community Events (Story 37.3)
    // ========================================================================

    /// Create an event.
    pub async fn create_event(
        &self,
        building_id: Uuid,
        organizer_id: Uuid,
        data: CreateCommunityEvent,
    ) -> Result<CommunityEvent, SqlxError> {
        let sql = format!(
            r#"
            INSERT INTO community_events (
                group_id, building_id, organizer_id, title, description, event_type,
                location, location_details, is_virtual, virtual_link, start_time, end_time,
                all_day, max_attendees, requires_rsvp, rsvp_deadline, cost_per_person, is_public
            )
            VALUES ($1, $2, $3, $4, $5, COALESCE($6::community_event_type, 'social'), $7, $8,
                    COALESCE($9, false), $10, $11, $12, COALESCE($13, false), $14,
                    COALESCE($15, true), $16, $17, COALESCE($18, true))
            RETURNING {EVENT_COLUMNS}
            "#
        );
        sqlx::query_as::<_, CommunityEvent>(sqlx::AssertSqlSafe(sql))
            .bind(data.group_id)
            .bind(building_id)
            .bind(organizer_id)
            .bind(&data.title)
            .bind(&data.description)
            .bind(&data.event_type)
            .bind(&data.location)
            .bind(&data.location_details)
            .bind(data.is_virtual)
            .bind(&data.virtual_link)
            .bind(data.start_time)
            .bind(data.end_time)
            .bind(data.all_day)
            .bind(data.max_attendees)
            .bind(data.requires_rsvp)
            .bind(data.rsvp_deadline)
            .bind(data.cost_per_person)
            .bind(data.is_public)
            .fetch_one(&self.pool)
            .await
    }

    /// Get upcoming events for a building.
    pub async fn get_upcoming_events(
        &self,
        building_id: Uuid,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<CommunityEvent>, SqlxError> {
        let sql = format!(
            r#"
            SELECT {EVENT_COLUMNS} FROM community_events
            WHERE building_id = $1 AND start_time > NOW() AND status = 'scheduled'
            ORDER BY start_time ASC
            LIMIT $2 OFFSET $3
            "#
        );
        sqlx::query_as::<_, CommunityEvent>(sqlx::AssertSqlSafe(sql))
            .bind(building_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
    }

    /// Tenant-safe event existence check: `true` only when the event's
    /// building belongs to `org_id`. `false` when the event is absent OR owned
    /// by another organization. `EXISTS` avoids decoding `CommunityEvent`
    /// (`event_type` is a Postgres enum typed as `String`).
    pub async fn event_exists_for_org(&self, id: Uuid, org_id: Uuid) -> Result<bool, SqlxError> {
        sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1
                FROM community_events e
                JOIN buildings b ON b.id = e.building_id
                WHERE e.id = $1 AND b.organization_id = $2
            )
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_one(&self.pool)
        .await
    }

    /// RSVP to an event.
    pub async fn rsvp_event(
        &self,
        event_id: Uuid,
        user_id: Uuid,
        data: EventRsvpRequest,
    ) -> Result<CommunityEventRsvp, SqlxError> {
        let sql = format!(
            r#"
            INSERT INTO community_event_rsvps (event_id, user_id, status, guests, note)
            VALUES ($1, $2, $3::event_rsvp_status, COALESCE($4, 0), $5)
            ON CONFLICT (event_id, user_id) DO UPDATE SET
                status = EXCLUDED.status,
                guests = EXCLUDED.guests,
                note = EXCLUDED.note,
                updated_at = NOW()
            RETURNING {RSVP_COLUMNS}
            "#
        );
        sqlx::query_as::<_, CommunityEventRsvp>(sqlx::AssertSqlSafe(sql))
            .bind(event_id)
            .bind(user_id)
            .bind(&data.status)
            .bind(data.guests)
            .bind(&data.note)
            .fetch_one(&self.pool)
            .await
    }

    // ========================================================================
    // Marketplace (Story 37.4)
    // ========================================================================

    /// Create a marketplace item.
    pub async fn create_item(
        &self,
        building_id: Uuid,
        seller_id: Uuid,
        data: CreateMarketplaceItem,
    ) -> Result<MarketplaceItem, SqlxError> {
        let sql = format!(
            r#"
            INSERT INTO marketplace_items (
                building_id, seller_id, title, description, category, condition,
                price, is_free, is_negotiable, is_trade_accepted, photo_urls,
                location, pickup_details
            )
            VALUES ($1, $2, $3, $4, $5::marketplace_category, $6::item_condition, $7,
                    COALESCE($8, false), COALESCE($9, true), COALESCE($10, false), $11, $12, $13)
            RETURNING {ITEM_COLUMNS}
            "#
        );
        sqlx::query_as::<_, MarketplaceItem>(sqlx::AssertSqlSafe(sql))
            .bind(building_id)
            .bind(seller_id)
            .bind(&data.title)
            .bind(&data.description)
            .bind(&data.category)
            .bind(&data.condition)
            .bind(data.price)
            .bind(data.is_free)
            .bind(data.is_negotiable)
            .bind(data.is_trade_accepted)
            .bind(&data.photo_urls)
            .bind(&data.location)
            .bind(&data.pickup_details)
            .fetch_one(&self.pool)
            .await
    }

    /// List active marketplace items.
    pub async fn list_items(
        &self,
        building_id: Uuid,
        category: Option<String>,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<MarketplaceItem>, SqlxError> {
        let sql = format!(
            r#"
            SELECT {ITEM_COLUMNS} FROM marketplace_items
            WHERE building_id = $1
              AND status = 'active'
              AND ($2::marketplace_category IS NULL OR category = $2::marketplace_category)
            ORDER BY created_at DESC
            LIMIT $3 OFFSET $4
            "#
        );
        sqlx::query_as::<_, MarketplaceItem>(sqlx::AssertSqlSafe(sql))
            .bind(building_id)
            .bind(&category)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
    }

    /// Get item by ID.
    pub async fn get_item(&self, id: Uuid) -> Result<Option<MarketplaceItem>, SqlxError> {
        let sql = format!("SELECT {ITEM_COLUMNS} FROM marketplace_items WHERE id = $1");
        sqlx::query_as::<_, MarketplaceItem>(sqlx::AssertSqlSafe(sql))
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    /// Tenant-safe marketplace-item existence check: `true` only when the
    /// item's building belongs to `org_id`. `false` when the item is absent OR
    /// owned by another organization. `EXISTS` avoids decoding `MarketplaceItem`
    /// (`category`/`condition` are Postgres enums typed as `String`).
    pub async fn item_exists_for_org(&self, id: Uuid, org_id: Uuid) -> Result<bool, SqlxError> {
        sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1
                FROM marketplace_items i
                JOIN buildings b ON b.id = i.building_id
                WHERE i.id = $1 AND b.organization_id = $2
            )
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_one(&self.pool)
        .await
    }

    /// Create inquiry on item.
    pub async fn create_inquiry(
        &self,
        item_id: Uuid,
        buyer_id: Uuid,
        data: CreateMarketplaceInquiry,
    ) -> Result<MarketplaceInquiry, SqlxError> {
        // Use CTE for atomic insert and count update to prevent race conditions
        sqlx::query_as::<_, MarketplaceInquiry>(
            r#"
            WITH new_inquiry AS (
                INSERT INTO marketplace_inquiries (item_id, buyer_id, message, offer_price)
                VALUES ($1, $2, $3, $4)
                RETURNING *
            ),
            update_count AS (
                UPDATE marketplace_items
                SET inquiry_count = inquiry_count + 1
                WHERE id = $1
            )
            SELECT * FROM new_inquiry
            "#,
        )
        .bind(item_id)
        .bind(buyer_id)
        .bind(&data.message)
        .bind(data.offer_price)
        .fetch_one(&self.pool)
        .await
    }
}
