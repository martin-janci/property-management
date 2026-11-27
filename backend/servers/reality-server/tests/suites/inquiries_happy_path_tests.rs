//! Happy-path 2xx tests for the inquiries surface
//! (`reality-server/src/routes/inquiries.rs`) — BIT-344 Wave 6.
//!
//! Complements `inquiries_authz_tests.rs` (401/403 only).
//!
//! Two auth shapes here:
//! - `send_contact_message` / `request_viewing` are public; they look up the
//!   listing's `created_by` as the inquiry's `realtor_id`, so a seeded `active`
//!   listing is enough (FK → `users` after migration 00148).
//! - the listing endpoints (`list_my_inquiries`, `get_inquiry`, …) use
//!   `RequestPrincipal`; the caller is seeded `principal_kind = 'platform'` to
//!   clear the tenant gate on the tenant-less test router, and owned inquiry
//!   rows are seeded with `realtor_id` = that user.

use crate::common::{make_app_state, mint_token, send, send_json, send_read};
use axum::{http::Method, Router};
use reality_server::routes;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

fn inquiries_router(pool: PgPool) -> Router {
    let state = make_app_state(pool);
    Router::new()
        .nest("/api/v1/inquiries", routes::inquiries::router())
        .with_state(state)
}

async fn seed_user(pool: &PgPool, tag: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'hash', $2, 'active', NOW(), 'platform')
        RETURNING id
        "#,
    )
    .bind(format!("{tag}@inq-hp.test"))
    .bind(format!("InqHP {tag}"))
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_user({tag}): {e}"))
}

/// Seed an `active` listing owned by `owner` and return its id.
async fn seed_listing(pool: &PgPool, tag: &str, owner: Uuid) -> Uuid {
    let org = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active')
        RETURNING id
        "#,
    )
    .bind(format!("Inq Org {tag}"))
    .bind(format!("inq-org-{tag}"))
    .bind(format!("{tag}@inq-org.test"))
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_org({tag}): {e}"));

    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO listings
            (organization_id, created_by, status, transaction_type,
             title, property_type, price, currency,
             street, city, postal_code, country)
        VALUES ($1, $2, 'active', 'sale',
                'Inquiry Listing', 'apartment', 200000.00, 'EUR',
                'Test Street 3', 'Bratislava', '81101', 'SK')
        RETURNING id
        "#,
    )
    .bind(org)
    .bind(owner)
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_listing({tag}): {e}"))
}

/// Seed an inquiry addressed to `realtor` (the authenticated caller).
async fn seed_inquiry(pool: &PgPool, listing_id: Uuid, realtor: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO listing_inquiries
            (listing_id, realtor_id, name, email, message)
        VALUES ($1, $2, 'Buyer', 'buyer@inq-hp.test', 'Is this available?')
        RETURNING id
        "#,
    )
    .bind(listing_id)
    .bind(realtor)
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_inquiry: {e}"))
}

// ── send_contact_message (POST /contact/{listing_id}) ────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn send_contact_message_returns_2xx(pool: PgPool) {
    let owner = seed_user(&pool, "contact-owner").await;
    let listing_id = seed_listing(&pool, "contact", owner).await;
    let app = inquiries_router(pool);
    let status = send_json(
        &app,
        Method::POST,
        &format!("/api/v1/inquiries/contact/{listing_id}"),
        None,
        json!({
            "name": "Jane Buyer",
            "email": "jane@example.com",
            "message": "Is this property still available?"
        }),
    )
    .await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

// Regression: the utoipa contract documents `201 Created` for this
// inquiry-creation POST; the handler must agree (it previously returned 200).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn send_contact_message_returns_201(pool: PgPool) {
    let owner = seed_user(&pool, "contact-owner-201").await;
    let listing_id = seed_listing(&pool, "contact-201", owner).await;
    let app = inquiries_router(pool);
    let status = send_json(
        &app,
        Method::POST,
        &format!("/api/v1/inquiries/contact/{listing_id}"),
        None,
        json!({
            "name": "Jane Buyer",
            "email": "jane@example.com",
            "message": "Is this property still available?"
        }),
    )
    .await;
    assert_eq!(status, 201, "send_contact_message must return 201 Created");
}

// ── request_viewing (POST /viewing/{listing_id}) ─────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn request_viewing_returns_2xx(pool: PgPool) {
    let owner = seed_user(&pool, "viewing-owner").await;
    let listing_id = seed_listing(&pool, "viewing", owner).await;
    let app = inquiries_router(pool);
    let status = send_json(
        &app,
        Method::POST,
        &format!("/api/v1/inquiries/viewing/{listing_id}"),
        None,
        json!({
            "name": "Jane Buyer",
            "email": "jane@example.com"
        }),
    )
    .await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

// Regression: the utoipa contract documents `201 Created` for this
// inquiry-creation POST; the handler must agree (it previously returned 200).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn request_viewing_returns_201(pool: PgPool) {
    let owner = seed_user(&pool, "viewing-owner-201").await;
    let listing_id = seed_listing(&pool, "viewing-201", owner).await;
    let app = inquiries_router(pool);
    let status = send_json(
        &app,
        Method::POST,
        &format!("/api/v1/inquiries/viewing/{listing_id}"),
        None,
        json!({
            "name": "Jane Buyer",
            "email": "jane@example.com"
        }),
    )
    .await;
    assert_eq!(status, 201, "request_viewing must return 201 Created");
}

// ── list_my_inquiries (GET /) ────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_my_inquiries_returns_2xx(pool: PgPool) {
    let user = seed_user(&pool, "list-realtor").await;
    let token = mint_token(user);
    let app = inquiries_router(pool);
    let status = send(&app, Method::GET, "/api/v1/inquiries", Some(&token)).await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

// ── list_buyer_inquiries (GET /mine) ─────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_buyer_inquiries_returns_2xx(pool: PgPool) {
    let user = seed_user(&pool, "list-buyer").await;
    let token = mint_token(user);
    let app = inquiries_router(pool);
    let status = send(&app, Method::GET, "/api/v1/inquiries/mine", Some(&token)).await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

// ── get_inquiry (GET /{id}) ──────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_inquiry_returns_2xx(pool: PgPool) {
    let realtor = seed_user(&pool, "get-realtor").await;
    let listing_id = seed_listing(&pool, "get", realtor).await;
    let inquiry_id = seed_inquiry(&pool, listing_id, realtor).await;
    let token = mint_token(realtor);
    let app = inquiries_router(pool);
    let status = send(
        &app,
        Method::GET,
        &format!("/api/v1/inquiries/{inquiry_id}"),
        Some(&token),
    )
    .await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

/// Seed a conversation message on `inquiry_id` from the realtor side.
async fn seed_inquiry_message(pool: &PgPool, inquiry_id: Uuid, sender: Uuid, body: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO inquiry_messages (inquiry_id, sender_type, sender_id, message)
        VALUES ($1, 'realtor', $2, $3)
        RETURNING id
        "#,
    )
    .bind(inquiry_id)
    .bind(sender)
    .bind(body)
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_inquiry_message: {e}"))
}

// Regression (code-review-reality-server-get-inquiry-empty-messages):
// `get_inquiry` once hardcoded `messages: vec![]` with a "Would fetch
// conversation messages" comment, so the realtor detail view always showed an
// empty conversation even when messages existed. The handler must now load the
// persisted thread and return it in the `messages` array.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_inquiry_returns_persisted_messages(pool: PgPool) {
    let realtor = seed_user(&pool, "msgs-realtor").await;
    let listing_id = seed_listing(&pool, "msgs", realtor).await;
    let inquiry_id = seed_inquiry(&pool, listing_id, realtor).await;
    seed_inquiry_message(&pool, inquiry_id, realtor, "Yes, still available.").await;
    let token = mint_token(realtor);
    let app = inquiries_router(pool);

    let (status, body) = send_read(
        &app,
        Method::GET,
        &format!("/api/v1/inquiries/{inquiry_id}"),
        Some(&token),
    )
    .await;

    assert!(status.is_success(), "expected 2xx, got {status}");
    let messages = body
        .get("messages")
        .and_then(|m| m.as_array())
        .expect("response must carry a `messages` array");
    assert_eq!(
        messages.len(),
        1,
        "the persisted conversation message must be returned, got {messages:?}"
    );
    assert_eq!(
        messages[0].get("message").and_then(|m| m.as_str()),
        Some("Yes, still available."),
        "the returned message content must match the persisted thread"
    );
}

// Regression (code-review-reality-server-get-inquiry-empty-msgs):
// the empty-messages contract. An inquiry that exists but has no persisted
// conversation messages yet (the common case right after it is created — the
// buyer's opening text lives on the `listing_inquiries` row, NOT in
// `inquiry_messages`) must still resolve to `200` with an EMPTY `messages`
// array. It must NOT 404 (the inquiry exists) and the array must not collapse
// to `null`/absent. The opening question stays on `inquiry.message`, so the
// thread being empty is correct, not a "not found".
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_inquiry_with_no_messages_returns_200_and_empty_array(pool: PgPool) {
    let realtor = seed_user(&pool, "empty-realtor").await;
    let listing_id = seed_listing(&pool, "empty", realtor).await;
    // Seeded inquiry carries message "Is this available?" and has NO
    // inquiry_messages rows — an untouched, just-submitted inquiry.
    let inquiry_id = seed_inquiry(&pool, listing_id, realtor).await;
    let token = mint_token(realtor);
    let app = inquiries_router(pool);

    let (status, body) = send_read(
        &app,
        Method::GET,
        &format!("/api/v1/inquiries/{inquiry_id}"),
        Some(&token),
    )
    .await;

    // Inquiry exists -> 200, never 404, even with an empty thread.
    assert!(
        status.is_success(),
        "an inquiry with no messages must still be 200, got {status}"
    );
    let messages = body
        .get("messages")
        .expect("response must always carry a `messages` key");
    let messages = messages
        .as_array()
        .expect("`messages` must be a JSON array, not null");
    assert!(
        messages.is_empty(),
        "a thread with no persisted conversation messages must be empty, got {messages:?}"
    );
    // The opening question is carried on the inquiry itself, which is why the
    // thread is legitimately empty rather than a 404.
    assert_eq!(
        body.get("inquiry")
            .and_then(|i| i.get("message"))
            .and_then(|m| m.as_str()),
        Some("Is this available?"),
        "the buyer's opening message lives on `inquiry.message`, not in the empty thread"
    );
}

// Companion to the realtor case above, on the buyer-facing detail endpoint
// `GET /api/v1/inquiries/mine/{id}`: the buyer's own just-submitted inquiry
// (no realtor reply, no follow-up yet) must resolve 200 with an empty
// `messages` array, not 404.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_buyer_inquiry_with_no_messages_returns_200_and_empty_array(pool: PgPool) {
    let buyer = seed_user(&pool, "empty-buyer").await;
    let listing_id = seed_listing(&pool, "empty-buyer", buyer).await;
    // Scope the inquiry to the buyer (`user_id`) so the buyer-axis lookup
    // (`get_inquiry_for_buyer`) resolves it.
    let inquiry_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO listing_inquiries
            (listing_id, realtor_id, user_id, name, email, message)
        VALUES ($1, $2, $2, 'Buyer', 'buyer@inq-hp.test', 'Is this available?')
        RETURNING id
        "#,
    )
    .bind(listing_id)
    .bind(buyer)
    .fetch_one(&pool)
    .await
    .unwrap_or_else(|e| panic!("seed buyer inquiry: {e}"));
    let token = mint_token(buyer);
    let app = inquiries_router(pool);

    let (status, body) = send_read(
        &app,
        Method::GET,
        &format!("/api/v1/inquiries/mine/{inquiry_id}"),
        Some(&token),
    )
    .await;

    assert!(
        status.is_success(),
        "a buyer inquiry with no messages must still be 200, got {status}"
    );
    let messages = body
        .get("messages")
        .and_then(|m| m.as_array())
        .expect("`messages` must be a JSON array, not null");
    assert!(
        messages.is_empty(),
        "an untouched buyer thread must be empty, got {messages:?}"
    );
}

// ── mark_as_read (PUT /{id}/read) ────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn mark_as_read_returns_2xx(pool: PgPool) {
    let realtor = seed_user(&pool, "read-realtor").await;
    let listing_id = seed_listing(&pool, "read", realtor).await;
    let inquiry_id = seed_inquiry(&pool, listing_id, realtor).await;
    let token = mint_token(realtor);
    let app = inquiries_router(pool);
    let status = send(
        &app,
        Method::PUT,
        &format!("/api/v1/inquiries/{inquiry_id}/read"),
        Some(&token),
    )
    .await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

// ── buyer linkage: signed-in submissions carry user_id ───────────────────────
//
// Regression: contact/viewing submissions were always created with
// `user_id = NULL`, so `GET /inquiries/mine` stayed empty for every buyer.
// A request with a valid bearer token must link the inquiry to that user;
// an anonymous one stays unlinked.

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn signed_in_contact_and_viewing_are_linked_to_buyer(pool: PgPool) {
    let owner = seed_user(&pool, "link-owner").await;
    let listing_id = seed_listing(&pool, "link", owner).await;
    let buyer = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ('buyer@inquiries-link.test', 'hash', 'Buyer', 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .fetch_one(&pool)
    .await
    .expect("seed buyer");
    let token = mint_token(buyer);
    let app = inquiries_router(pool.clone());

    let contact = send_json(
        &app,
        Method::POST,
        &format!("/api/v1/inquiries/contact/{listing_id}"),
        Some(&token),
        json!({
            "name": "Jane Buyer",
            "email": "jane@example.com",
            "message": "Is this property still available?"
        }),
    )
    .await;
    assert_eq!(contact, 201, "signed-in contact must succeed");

    let viewing = send_json(
        &app,
        Method::POST,
        &format!("/api/v1/inquiries/viewing/{listing_id}"),
        Some(&token),
        json!({
            "name": "Jane Buyer",
            "email": "jane@example.com",
            "preferred_times": ["  ", "Saturday 10:00"]
        }),
    )
    .await;
    assert_eq!(viewing, 201, "signed-in viewing request must succeed");

    let anonymous = send_json(
        &app,
        Method::POST,
        &format!("/api/v1/inquiries/contact/{listing_id}"),
        None,
        json!({
            "name": "Anon",
            "email": "anon@example.com",
            "message": "Anonymous question about the flat"
        }),
    )
    .await;
    assert_eq!(anonymous, 201);

    let linked: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM listing_inquiries WHERE listing_id = $1 AND user_id = $2",
    )
    .bind(listing_id)
    .bind(buyer)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(linked, 2, "both signed-in submissions must carry user_id");

    let unlinked: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM listing_inquiries WHERE listing_id = $1 AND user_id IS NULL",
    )
    .bind(listing_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(unlinked, 1, "anonymous submission stays unlinked");

    let slot: Option<String> = sqlx::query_scalar(
        "SELECT preferred_time FROM listing_inquiries
         WHERE listing_id = $1 AND inquiry_type = 'viewing'",
    )
    .bind(listing_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(slot.as_deref(), Some("Saturday 10:00"));
}

// ── buyer thread: detail / reply / cancel viewing (GET|POST /mine/{id}…) ─────

async fn seed_buyer_inquiry(
    pool: &PgPool,
    listing_id: Uuid,
    realtor: Uuid,
    buyer: Uuid,
    inquiry_type: &str,
    status: &str,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO listing_inquiries
            (listing_id, realtor_id, user_id, name, email, message, inquiry_type, status)
        VALUES ($1, $2, $3, 'Buyer', 'buyer@inq-hp.test', 'Hello', $4, $5)
        RETURNING id
        "#,
    )
    .bind(listing_id)
    .bind(realtor)
    .bind(buyer)
    .bind(inquiry_type)
    .bind(status)
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_buyer_inquiry: {e}"))
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn buyer_thread_detail_reply_and_cancel(pool: PgPool) {
    let realtor = seed_user(&pool, "thread-realtor").await;
    let buyer = seed_user(&pool, "thread-buyer").await;
    let other = seed_user(&pool, "thread-other").await;
    let listing_id = seed_listing(&pool, "thread", realtor).await;
    let info = seed_buyer_inquiry(&pool, listing_id, realtor, buyer, "info", "responded").await;
    let viewing = seed_buyer_inquiry(&pool, listing_id, realtor, buyer, "viewing", "new").await;
    let buyer_token = mint_token(buyer);
    let other_token = mint_token(other);
    let app = inquiries_router(pool.clone());

    // Detail: owner 200, another buyer 404 (no leak).
    let own = send(
        &app,
        Method::GET,
        &format!("/api/v1/inquiries/mine/{info}"),
        Some(&buyer_token),
    )
    .await;
    assert_eq!(own, 200);
    let foreign = send(
        &app,
        Method::GET,
        &format!("/api/v1/inquiries/mine/{info}"),
        Some(&other_token),
    )
    .await;
    assert_eq!(foreign, 404, "another buyer's inquiry must 404");

    // Reply: owner 201 and the inquiry returns to the realtor's unanswered queue.
    let reply = send_json(
        &app,
        Method::POST,
        &format!("/api/v1/inquiries/mine/{info}/messages"),
        Some(&buyer_token),
        json!({ "message": "Can we talk on Monday?" }),
    )
    .await;
    assert_eq!(reply, 201);
    let (sender, status): (String, String) = sqlx::query_as(
        "SELECT m.sender_type, i.status FROM inquiry_messages m
         JOIN listing_inquiries i ON i.id = m.inquiry_id WHERE m.inquiry_id = $1",
    )
    .bind(info)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(sender, "user");
    assert_eq!(status, "new");

    let foreign_reply = send_json(
        &app,
        Method::POST,
        &format!("/api/v1/inquiries/mine/{info}/messages"),
        Some(&other_token),
        json!({ "message": "Not my thread" }),
    )
    .await;
    assert_eq!(foreign_reply, 404);

    // Cancel: only viewing requests qualify.
    let not_viewing = send(
        &app,
        Method::POST,
        &format!("/api/v1/inquiries/mine/{info}/cancel"),
        Some(&buyer_token),
    )
    .await;
    assert_eq!(not_viewing, 404, "an info inquiry is not a viewing");
    let foreign_cancel = send(
        &app,
        Method::POST,
        &format!("/api/v1/inquiries/mine/{viewing}/cancel"),
        Some(&other_token),
    )
    .await;
    assert_eq!(foreign_cancel, 404);
    let cancelled = send(
        &app,
        Method::POST,
        &format!("/api/v1/inquiries/mine/{viewing}/cancel"),
        Some(&buyer_token),
    )
    .await;
    assert_eq!(cancelled, 204);

    // A closed thread no longer accepts replies.
    let closed_reply = send_json(
        &app,
        Method::POST,
        &format!("/api/v1/inquiries/mine/{viewing}/messages"),
        Some(&buyer_token),
        json!({ "message": "Still there?" }),
    )
    .await;
    assert_eq!(closed_reply, 404);
}

// ── schedule_viewing (POST /{id}/schedule-viewing) ───────────────────────────
// Story 33.3: the realtor promotes one of their own inquiries into a confirmed
// `viewing_schedules` row. Before this endpoint existed, the viewing-schedule
// table + reminder seam were dead (no writer).

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn schedule_viewing_creates_row_for_owning_realtor(pool: PgPool) {
    let realtor = seed_user(&pool, "sched-realtor").await;
    let listing_id = seed_listing(&pool, "sched", realtor).await;
    let inquiry_id = seed_inquiry(&pool, listing_id, realtor).await;
    let token = mint_token(realtor);
    let app = inquiries_router(pool.clone());

    let status = send_json(
        &app,
        Method::POST,
        &format!("/api/v1/inquiries/{inquiry_id}/schedule-viewing"),
        Some(&token),
        json!({ "scheduled_at": "2030-01-01T10:00:00Z", "duration_minutes": 45 }),
    )
    .await;
    assert_eq!(
        status, 201,
        "owning realtor must be able to schedule a viewing"
    );

    // The persisted row copies attendee contact from the inquiry and keeps the
    // realtor + requested duration.
    let (att_email, duration, vrealtor): (String, i32, Uuid) = sqlx::query_as(
        "SELECT attendee_email, duration_minutes, realtor_id
         FROM viewing_schedules WHERE inquiry_id = $1",
    )
    .bind(inquiry_id)
    .fetch_one(&pool)
    .await
    .expect("a viewing_schedules row must exist");
    assert_eq!(att_email, "buyer@inq-hp.test");
    assert_eq!(duration, 45);
    assert_eq!(vrealtor, realtor);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn schedule_viewing_rejects_past_time(pool: PgPool) {
    let realtor = seed_user(&pool, "sched-past-realtor").await;
    let listing_id = seed_listing(&pool, "sched-past", realtor).await;
    let inquiry_id = seed_inquiry(&pool, listing_id, realtor).await;
    let token = mint_token(realtor);
    let app = inquiries_router(pool);

    let status = send_json(
        &app,
        Method::POST,
        &format!("/api/v1/inquiries/{inquiry_id}/schedule-viewing"),
        Some(&token),
        json!({ "scheduled_at": "2000-01-01T10:00:00Z" }),
    )
    .await;
    assert_eq!(status, 400, "a viewing in the past must be rejected");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn schedule_viewing_404_for_foreign_realtor(pool: PgPool) {
    let realtor = seed_user(&pool, "sched-owner").await;
    let other = seed_user(&pool, "sched-other").await;
    let listing_id = seed_listing(&pool, "sched-foreign", realtor).await;
    let inquiry_id = seed_inquiry(&pool, listing_id, realtor).await;
    let other_token = mint_token(other);
    let app = inquiries_router(pool.clone());

    let status = send_json(
        &app,
        Method::POST,
        &format!("/api/v1/inquiries/{inquiry_id}/schedule-viewing"),
        Some(&other_token),
        json!({ "scheduled_at": "2030-01-01T10:00:00Z" }),
    )
    .await;
    assert_eq!(
        status, 404,
        "a realtor must not schedule another realtor's inquiry"
    );

    // IDOR: no row leaked into the table under the foreign realtor.
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM viewing_schedules WHERE inquiry_id = $1")
            .bind(inquiry_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        count, 0,
        "no viewing_schedules row may be created for a foreign realtor"
    );
}
