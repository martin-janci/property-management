//! End-to-end tests for the Stripe Checkout settlement webhook
//! (Story 11.5, BIT-181), driven through the wired Axum router via `TestApp`.
//!
//! These pin the security-critical contract of
//! `POST /api/v1/integrations/webhooks/payments/{provider}`:
//!
//! - **Fail closed** when `STRIPE_WEBHOOK_SECRET` is unset (`503`, never accept).
//! - **Reject forged signatures** with `401` before any DB mutation.
//! - **Settle on a valid signature**: a correctly-signed
//!   `checkout.session.completed` (`payment_status = "paid"`) records a payment,
//!   allocates it to the invoice (the DB trigger flips the invoice to `paid`),
//!   and marks the session `completed`.
//! - **Idempotent**: a second delivery of the same event does not double-settle.
//!
//! They need a `PgPool` because `TestApp` builds the full `AppState`; CI runs
//! them against an ephemeral migrated database.

#![allow(dead_code)]

use crate::common::{seed_org, TestApp};
use axum::http::StatusCode;
use hmac::{Hmac, KeyInit, Mac};
use rust_decimal::Decimal;
use sha2::Sha256;
use sqlx::PgPool;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

const SECRET_ENV: &str = "STRIPE_WEBHOOK_SECRET";
const SIG_HEADER: &str = "Stripe-Signature";
const WEBHOOK_URI: &str = "/api/v1/integrations/webhooks/payments/stripe";

/// `std::env::{set_var, remove_var}` are not safe to interleave with `getenv`
/// on glibc, and integration tests in this binary may run concurrently. Every
/// test that mutates the webhook-secret env serializes behind this mutex, held
/// across the whole test body so the env stays stable while `TestApp` boots.
static ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Build the `t=<ts>,v1=<hex>` Stripe-Signature header for `body`.
fn sign(secret: &str, ts: i64, body: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("hmac key");
    mac.update(ts.to_string().as_bytes());
    mac.update(b".");
    mac.update(body);
    format!("t={ts},v1={}", hex::encode(mac.finalize().into_bytes()))
}

/// Seed an org → building → unit → invoice (balance_due = `total`) → pending
/// `online_payment_sessions` row, and return `(invoice_id, session_id_str)`.
async fn seed_invoice_with_session(
    pool: &PgPool,
    slug: &str,
    provider_session_id: &str,
    total: Decimal,
) -> (Uuid, Uuid) {
    let org_id = seed_org(pool, slug).await;

    let building_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, street, city, postal_code, country)
           VALUES ($1, $2, 'Bratislava', '81101', 'Slovakia') RETURNING id"#,
    )
    .bind(org_id)
    .bind(format!("{slug} Street 1"))
    .fetch_one(pool)
    .await
    .expect("seed building");

    let unit_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO units (building_id, designation) VALUES ($1, $2) RETURNING id"#,
    )
    .bind(building_id)
    .bind(format!("U-{slug}"))
    .fetch_one(pool)
    .await
    .expect("seed unit");

    let invoice_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO invoices
               (organization_id, unit_id, invoice_number, status, due_date,
                subtotal, total, balance_due, currency)
           VALUES ($1, $2, $3, 'sent', CURRENT_DATE,
                   $4, $4, $4, 'EUR')
           RETURNING id"#,
    )
    .bind(org_id)
    .bind(unit_id)
    .bind(format!("INV-{slug}"))
    .bind(total)
    .fetch_one(pool)
    .await
    .expect("seed invoice");

    let session_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO online_payment_sessions
               (organization_id, invoice_id, provider, session_id, checkout_url,
                amount, currency, status)
           VALUES ($1, $2, 'stripe', $3, 'https://checkout.stripe.test/x',
                   $4, 'EUR', 'pending')
           RETURNING id"#,
    )
    .bind(org_id)
    .bind(invoice_id)
    .bind(provider_session_id)
    .bind(total)
    .fetch_one(pool)
    .await
    .expect("seed session");

    (invoice_id, session_id)
}

/// Fail-closed: with NO secret configured the route rejects any webhook with
/// `503` — it never silently accepts an unverified payload.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn unconfigured_secret_fails_closed(pool: PgPool) {
    let _guard = ENV_LOCK.lock().await;
    std::env::remove_var(SECRET_ENV);

    let app = TestApp::new(pool).await;
    let body = serde_json::json!({ "type": "checkout.session.completed" });
    let req = app
        .post(WEBHOOK_URI)
        .header(SIG_HEADER, "t=1,v1=deadbeef")
        .json(body)
        .build();
    let resp = app.execute(req).await;

    assert_eq!(
        resp.status,
        StatusCode::SERVICE_UNAVAILABLE,
        "no secret must fail closed; body: {}",
        resp.text()
    );
}

/// A forged signature is rejected with `401`.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn invalid_signature_rejected(pool: PgPool) {
    let _guard = ENV_LOCK.lock().await;
    std::env::set_var(SECRET_ENV, "whsec_test");

    let app = TestApp::new(pool).await;
    let body = serde_json::json!({ "type": "checkout.session.completed" });
    let req = app
        .post(WEBHOOK_URI)
        .header(SIG_HEADER, "t=1700000000,v1=deadbeef")
        .json(body)
        .build();
    let resp = app.execute(req).await;

    std::env::remove_var(SECRET_ENV);
    assert_eq!(
        resp.status,
        StatusCode::UNAUTHORIZED,
        "forged signature must be rejected; body: {}",
        resp.text()
    );
}

/// Happy path: a valid `checkout.session.completed` settles the invoice and the
/// session, and a duplicate delivery is idempotent (no double-settlement).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn valid_webhook_settles_invoice_and_is_idempotent(pool: PgPool) {
    let _guard = ENV_LOCK.lock().await;
    let secret = "whsec_settle";
    std::env::set_var(SECRET_ENV, secret);

    let provider_session_id = "cs_test_settle_1";
    let (invoice_id, session_pk) =
        seed_invoice_with_session(&pool, "settle", provider_session_id, Decimal::new(10000, 2))
            .await;

    let app = TestApp::new(pool.clone()).await;

    let body = serde_json::json!({
        "type": "checkout.session.completed",
        "data": { "object": {
            "id": provider_session_id,
            "payment_status": "paid",
            "payment_intent": "pi_test_settle_1"
        }}
    });
    // Sign the exact bytes the test client will send (`Value::to_string`).
    let raw = body.to_string();
    let ts = chrono::Utc::now().timestamp();
    let header = sign(secret, ts, raw.as_bytes());

    let resp = app
        .execute(
            app.post(WEBHOOK_URI)
                .header(SIG_HEADER, &header)
                .json(body.clone())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "valid webhook should be accepted; body: {}",
        resp.text()
    );

    // Invoice is now paid with zero balance.
    let (status, balance): (String, Decimal) =
        sqlx::query_as("SELECT status::text, balance_due FROM invoices WHERE id = $1")
            .bind(invoice_id)
            .fetch_one(&pool)
            .await
            .expect("load invoice");
    assert_eq!(status, "paid", "invoice should be marked paid");
    assert_eq!(balance, Decimal::ZERO, "balance should be cleared");

    // A payment was recorded with the gateway reference and no internal user.
    // `count(recorded_by)` counts only non-null values (Postgres has no
    // `max(uuid)` aggregate), so 0 confirms the single payment has a NULL
    // `recorded_by` — the payer is the gateway, not an internal user.
    let (pay_count, ext_ref, recorded_by_count): (i64, Option<String>, i64) = sqlx::query_as(
        "SELECT count(*)::int8, max(external_reference), count(recorded_by)::int8 \
         FROM payments WHERE unit_id = (SELECT unit_id FROM invoices WHERE id = $1)",
    )
    .bind(invoice_id)
    .fetch_one(&pool)
    .await
    .expect("load payments");
    assert_eq!(pay_count, 1, "exactly one payment recorded");
    assert_eq!(ext_ref.as_deref(), Some("pi_test_settle_1"));
    assert_eq!(
        recorded_by_count, 0,
        "gateway payment has no internal user (recorded_by NULL)"
    );

    // The session is marked completed and linked to the payment.
    let sess_status: String =
        sqlx::query_scalar("SELECT status FROM online_payment_sessions WHERE id = $1")
            .bind(session_pk)
            .fetch_one(&pool)
            .await
            .expect("load session");
    assert_eq!(sess_status, "completed");

    // Idempotency: replaying the same delivery must not record a second payment.
    let resp2 = app
        .execute(
            app.post(WEBHOOK_URI)
                .header(SIG_HEADER, &header)
                .json(body)
                .build(),
        )
        .await;
    assert_eq!(resp2.status, StatusCode::OK, "replay should be acked");

    let pay_count2: i64 = sqlx::query_scalar(
        "SELECT count(*)::int8 FROM payments \
         WHERE unit_id = (SELECT unit_id FROM invoices WHERE id = $1)",
    )
    .bind(invoice_id)
    .fetch_one(&pool)
    .await
    .expect("recount payments");

    std::env::remove_var(SECRET_ENV);
    assert_eq!(pay_count2, 1, "replay must not double-settle");
}

/// Defense-in-depth: a correctly-SIGNED `checkout.session.completed` whose
/// `amount_total` disagrees with our stored session amount must NOT settle the
/// invoice. The handler acks (`200`) so Stripe stops retrying the mismatched
/// event, but records no payment and leaves the invoice unpaid.
///
/// On code that ignores `amount_total` this FAILS (it would settle); after the
/// cross-check it PASSES.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn webhook_amount_mismatch_does_not_settle(pool: PgPool) {
    let _guard = ENV_LOCK.lock().await;
    let secret = "whsec_mismatch";
    std::env::set_var(SECRET_ENV, secret);

    // Stored session is for 100.00 EUR (10000 minor units).
    let provider_session_id = "cs_test_mismatch_1";
    let (invoice_id, session_pk) = seed_invoice_with_session(
        &pool,
        "mismatch",
        provider_session_id,
        Decimal::new(10000, 2),
    )
    .await;

    let app = TestApp::new(pool.clone()).await;

    // Stripe reports collecting only 5.00 EUR (500 minor units) — a mismatch.
    let body = serde_json::json!({
        "type": "checkout.session.completed",
        "data": { "object": {
            "id": provider_session_id,
            "payment_status": "paid",
            "payment_intent": "pi_test_mismatch_1",
            "amount_total": 500,
            "currency": "eur"
        }}
    });
    let raw = body.to_string();
    let ts = chrono::Utc::now().timestamp();
    let header = sign(secret, ts, raw.as_bytes());

    let resp = app
        .execute(
            app.post(WEBHOOK_URI)
                .header(SIG_HEADER, &header)
                .json(body)
                .build(),
        )
        .await;
    // Acked so Stripe stops retrying, but nothing was settled.
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "mismatched amount should be acked; body: {}",
        resp.text()
    );

    // Invoice is untouched: still sent, full balance outstanding.
    let (status, balance): (String, Decimal) =
        sqlx::query_as("SELECT status::text, balance_due FROM invoices WHERE id = $1")
            .bind(invoice_id)
            .fetch_one(&pool)
            .await
            .expect("load invoice");
    assert_eq!(
        status, "sent",
        "invoice must NOT be settled on amount mismatch"
    );
    assert_eq!(
        balance,
        Decimal::new(10000, 2),
        "balance must be unchanged on amount mismatch"
    );

    // No payment was recorded.
    let pay_count: i64 = sqlx::query_scalar(
        "SELECT count(*)::int8 FROM payments \
         WHERE unit_id = (SELECT unit_id FROM invoices WHERE id = $1)",
    )
    .bind(invoice_id)
    .fetch_one(&pool)
    .await
    .expect("count payments");
    assert_eq!(
        pay_count, 0,
        "no payment may be recorded on amount mismatch"
    );

    // The session stays pending (not completed).
    let sess_status: String =
        sqlx::query_scalar("SELECT status FROM online_payment_sessions WHERE id = $1")
            .bind(session_pk)
            .fetch_one(&pool)
            .await
            .expect("load session");

    std::env::remove_var(SECRET_ENV);
    assert_eq!(
        sess_status, "pending",
        "session must stay pending on amount mismatch"
    );
}

/// Positive control for the cross-check: when `amount_total`/`currency` match
/// the stored session, settlement proceeds exactly as without the fields.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn webhook_matching_amount_total_settles(pool: PgPool) {
    let _guard = ENV_LOCK.lock().await;
    let secret = "whsec_match";
    std::env::set_var(SECRET_ENV, secret);

    let provider_session_id = "cs_test_match_1";
    let (invoice_id, session_pk) =
        seed_invoice_with_session(&pool, "match", provider_session_id, Decimal::new(10000, 2))
            .await;

    let app = TestApp::new(pool.clone()).await;

    // amount_total = 10000 minor units == 100.00 EUR stored amount.
    let body = serde_json::json!({
        "type": "checkout.session.completed",
        "data": { "object": {
            "id": provider_session_id,
            "payment_status": "paid",
            "payment_intent": "pi_test_match_1",
            "amount_total": 10000,
            "currency": "eur"
        }}
    });
    let raw = body.to_string();
    let ts = chrono::Utc::now().timestamp();
    let header = sign(secret, ts, raw.as_bytes());

    let resp = app
        .execute(
            app.post(WEBHOOK_URI)
                .header(SIG_HEADER, &header)
                .json(body)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "matching webhook should be accepted; body: {}",
        resp.text()
    );

    let (status, balance): (String, Decimal) =
        sqlx::query_as("SELECT status::text, balance_due FROM invoices WHERE id = $1")
            .bind(invoice_id)
            .fetch_one(&pool)
            .await
            .expect("load invoice");
    assert_eq!(status, "paid", "matching amount must settle the invoice");
    assert_eq!(balance, Decimal::ZERO, "balance should be cleared");

    let sess_status: String =
        sqlx::query_scalar("SELECT status FROM online_payment_sessions WHERE id = $1")
            .bind(session_pk)
            .fetch_one(&pool)
            .await
            .expect("load session");

    std::env::remove_var(SECRET_ENV);
    assert_eq!(sess_status, "completed", "session marked completed");
}

/// Concurrent-replay (the race the sequential test above cannot catch):
/// Stripe is at-least-once and does not serialize deliveries, so two duplicate
/// `checkout.session.completed` events can BOTH read the session as not-yet
/// `completed` before either commits. The pre-tx status read in the handler is
/// a TOCTOU window; the security property is enforced at the DB level by the
/// atomic claim inside `settle_invoice_from_gateway`. We exercise that claim
/// directly — two settlements racing on the same pending session snapshot must
/// produce exactly ONE payment + allocation (no double-credit), and the loser
/// must return `Ok(None)`.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn concurrent_settlement_does_not_double_credit(pool: PgPool) {
    use db::repositories::FinancialRepository;

    let provider_session_id = "cs_test_concurrent_1";
    let (invoice_id, _session_pk) = seed_invoice_with_session(
        &pool,
        "concurrent",
        provider_session_id,
        Decimal::new(10000, 2),
    )
    .await;

    let repo = FinancialRepository::new(pool.clone());

    // Both "deliveries" load the SAME pending snapshot — emulating two
    // duplicates that each passed the handler's pre-tx status read.
    let session = repo
        .get_payment_session_by_provider_id(provider_session_id)
        .await
        .expect("load session")
        .expect("session exists");
    let invoice = repo
        .get_invoice(invoice_id)
        .await
        .expect("load invoice")
        .expect("invoice exists");
    assert_eq!(session.status, "pending", "precondition: not yet settled");

    let task1 = {
        let (repo, session, invoice) = (repo.clone(), session.clone(), invoice.clone());
        tokio::spawn(async move {
            repo.settle_invoice_from_gateway(&session, &invoice, "pi_concurrent_1")
                .await
        })
    };
    let task2 = {
        let (repo, session, invoice) = (repo.clone(), session.clone(), invoice.clone());
        tokio::spawn(async move {
            repo.settle_invoice_from_gateway(&session, &invoice, "pi_concurrent_1")
                .await
        })
    };

    let r1 = task1.await.expect("join task1").expect("settle task1 ok");
    let r2 = task2.await.expect("join task2").expect("settle task2 ok");

    // Exactly one delivery settles (Some), the other no-ops (None).
    assert!(
        r1.is_some() ^ r2.is_some(),
        "exactly one of the two concurrent deliveries must settle; got {:?} / {:?}",
        r1.is_some(),
        r2.is_some(),
    );

    // The DB-level invariant: one payment, one allocation, invoice paid once.
    let pay_count: i64 = sqlx::query_scalar(
        "SELECT count(*)::int8 FROM payments \
         WHERE unit_id = (SELECT unit_id FROM invoices WHERE id = $1)",
    )
    .bind(invoice_id)
    .fetch_one(&pool)
    .await
    .expect("count payments");
    assert_eq!(
        pay_count, 1,
        "concurrent duplicate deliveries must record exactly one payment"
    );

    let alloc_count: i64 =
        sqlx::query_scalar("SELECT count(*)::int8 FROM payment_allocations WHERE invoice_id = $1")
            .bind(invoice_id)
            .fetch_one(&pool)
            .await
            .expect("count allocations");
    assert_eq!(alloc_count, 1, "exactly one allocation");

    let (status, balance): (String, Decimal) =
        sqlx::query_as("SELECT status::text, balance_due FROM invoices WHERE id = $1")
            .bind(invoice_id)
            .fetch_one(&pool)
            .await
            .expect("load invoice");
    assert_eq!(status, "paid", "invoice settled exactly once");
    assert_eq!(balance, Decimal::ZERO, "balance cleared, not over-credited");
}
