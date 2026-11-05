//! Round-5 security fixes (sec-b): nested-id scoping and DB-role gates.
//!
//! Every test uses a real login token + `X-Tenant-ID` with a seeded
//! `organization_members` row, so `RlsConnection` resolves the role from the
//! DB. TestApp connects as superuser (RLS is bypassed), so these assertions
//! prove the explicit org / parent predicates, not RLS.

#![allow(dead_code)]

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{create_authenticated_user, seed_membership, seed_org, TestApp, TestUser};

/// Register a user, add them to `org` with `role`. Returns (token, user_id).
async fn member(app: &TestApp, org: Uuid, role: &str) -> (String, Uuid) {
    let user = TestUser::new();
    let (token, _refresh) = create_authenticated_user(app, &user).await;
    let uid = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .expect("user id");
    seed_membership(&app.pool, org, uid, role).await;
    (token, uid)
}

async fn seed_building(pool: &PgPool, org: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO buildings (organization_id, street, city, postal_code, country) \
         VALUES ($1, 'Main 1', 'Bratislava', '81101', 'SK') RETURNING id",
    )
    .bind(org)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

async fn seed_unit(pool: &PgPool, building: Uuid, designation: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO units (building_id, designation) VALUES ($1, $2) RETURNING id",
    )
    .bind(building)
    .bind(designation)
    .fetch_one(pool)
    .await
    .expect("seed unit")
}

// ==================== building_certifications ====================

async fn seed_cert(pool: &PgPool, org: Uuid, building: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO building_certifications (organization_id, building_id, program, level) \
         VALUES ($1, $2, 'leed', 'gold') RETURNING id",
    )
    .bind(org)
    .bind(building)
    .fetch_one(pool)
    .await
    .expect("seed cert")
}

async fn seed_credit(pool: &PgPool, org: Uuid, cert: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO certification_credits (organization_id, certification_id, category, credit_name) \
         VALUES ($1, $2, 'energy_efficiency', 'Credit') RETURNING id",
    )
    .bind(org)
    .bind(cert)
    .fetch_one(pool)
    .await
    .expect("seed credit")
}

/// A child id addressed through the wrong parent certification is 404, both
/// for a sibling cert in the same org and for another org's cert.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn cert_child_routes_require_matching_parent(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "r5-cert").await;
    let other_org = seed_org(&pool, "r5-cert-o").await;
    let (token, _) = member(&app, org, "manager").await;
    let building = seed_building(&pool, org).await;
    let cert_a = seed_cert(&pool, org, building).await;
    let cert_b = seed_cert(&pool, org, building).await;
    let credit_a = seed_credit(&pool, org, cert_a).await;
    let foreign_building = seed_building(&pool, other_org).await;
    let foreign_cert = seed_cert(&pool, other_org, foreign_building).await;

    let base = "/api/v1/building-certifications";

    // Correct parent works.
    let resp = app
        .execute(
            app.get(&format!("{base}/{cert_a}/credits/{credit_a}"))
                .bearer(&token)
                .tenant(org)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());

    for parent in [cert_b, foreign_cert] {
        let path = format!("{base}/{parent}/credits/{credit_a}");
        let resp = app
            .execute(app.get(&path).bearer(&token).tenant(org).build())
            .await;
        assert_eq!(resp.status, StatusCode::NOT_FOUND, "get via {parent}");

        let resp = app
            .execute(
                app.put(&path)
                    .bearer(&token)
                    .tenant(org)
                    .json(json!({ "credit_name": "HIJACKED" }))
                    .build(),
            )
            .await;
        assert_eq!(resp.status, StatusCode::NOT_FOUND, "update via {parent}");

        let resp = app
            .execute(app.delete(&path).bearer(&token).tenant(org).build())
            .await;
        assert_eq!(resp.status, StatusCode::NOT_FOUND, "delete via {parent}");
    }

    // A document cannot reference a credit of another certification.
    let resp = app
        .execute(
            app.post(&format!("{base}/{cert_b}/documents"))
                .bearer(&token)
                .tenant(org)
                .json(json!({
                    "certification_id": cert_b,
                    "credit_id": credit_a,
                    "document_type": "evidence",
                    "title": "Doc",
                    "file_url": "https://example.test/doc.pdf"
                }))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::NOT_FOUND, "{}", resp.text());

    let name: String =
        sqlx::query_scalar("SELECT credit_name FROM certification_credits WHERE id = $1")
            .bind(credit_a)
            .fetch_one(&pool)
            .await
            .expect("credit must survive");
    assert_eq!(name, "Credit");
}

// ==================== registry ====================

async fn seed_document(pool: &PgPool, org: Uuid, created_by: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO documents \
             (organization_id, title, category, file_key, file_name, mime_type, size_bytes, created_by) \
         VALUES ($1, 'Doc', 'other', $2, 'doc.pdf', 'application/pdf', 1, $3) RETURNING id",
    )
    .bind(org)
    .bind(format!("{org}/r5/{}.pdf", Uuid::new_v4()))
    .bind(created_by)
    .fetch_one(pool)
    .await
    .expect("seed document")
}

fn vehicle_body(unit: Uuid) -> serde_json::Value {
    json!({
        "unit_id": unit,
        "vehicle_type": "car",
        "make": "Skoda",
        "model": "Octavia",
        "license_plate": "BA123AB"
    })
}

/// A resident may register only on a unit they are linked to; a manager may
/// register on any unit of the org.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn registry_create_requires_own_unit_unless_manager(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "r5-reg").await;
    let building = seed_building(&pool, org).await;
    let own_unit = seed_unit(&pool, building, "R-1").await;
    let other_unit = seed_unit(&pool, building, "R-2").await;
    let (resident_token, resident_id) = member(&app, org, "resident").await;
    let (manager_token, _) = member(&app, org, "manager").await;
    sqlx::query(
        "INSERT INTO unit_residents (unit_id, user_id, resident_type) VALUES ($1, $2, 'tenant')",
    )
    .bind(own_unit)
    .bind(resident_id)
    .execute(&pool)
    .await
    .unwrap();

    let pet = |unit: Uuid| json!({ "unit_id": unit, "pet_name": "Rex", "pet_type": "dog", "pet_size": "small" });

    let resp = app
        .execute(
            app.post("/api/v1/registry/pets")
                .bearer(&resident_token)
                .tenant(org)
                .json(pet(other_unit))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "pet on foreign unit: {}",
        resp.text()
    );

    let resp = app
        .execute(
            app.post("/api/v1/registry/vehicles")
                .bearer(&resident_token)
                .tenant(org)
                .json(vehicle_body(other_unit))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "vehicle on foreign unit: {}",
        resp.text()
    );

    let resp = app
        .execute(
            app.post("/api/v1/registry/pets")
                .bearer(&resident_token)
                .tenant(org)
                .json(pet(own_unit))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "pet on own unit: {}",
        resp.text()
    );

    let resp = app
        .execute(
            app.post("/api/v1/registry/vehicles")
                .bearer(&manager_token)
                .tenant(org)
                .json(vehicle_body(other_unit))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "manager on any unit: {}",
        resp.text()
    );
}

/// Document and parking-spot references must belong to the caller's org.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn registry_rejects_foreign_document_and_parking_spot(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "r5-reg-ref").await;
    let other_org = seed_org(&pool, "r5-reg-ref-o").await;
    let building = seed_building(&pool, org).await;
    let unit = seed_unit(&pool, building, "R-1").await;
    let other_building = seed_building(&pool, other_org).await;
    let (token, manager_id) = member(&app, org, "manager").await;
    let foreign_doc = seed_document(&pool, other_org, manager_id).await;
    let own_doc = seed_document(&pool, org, manager_id).await;
    let foreign_spot = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO parking_spots (tenant_id, building_id, spot_number) VALUES ($1, $2, 'X-1') RETURNING id",
    )
    .bind(other_org)
    .bind(other_building)
    .fetch_one(&pool)
    .await
    .unwrap();

    let mut body = vehicle_body(unit);
    body["insurance_document_id"] = json!(foreign_doc);
    let resp = app
        .execute(
            app.post("/api/v1/registry/vehicles")
                .bearer(&token)
                .tenant(org)
                .json(&body)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "foreign doc: {}",
        resp.text()
    );

    let resp = app
        .execute(
            app.post("/api/v1/registry/pets")
                .bearer(&token)
                .tenant(org)
                .json(json!({
                    "unit_id": unit, "pet_name": "Rex", "pet_type": "dog", "pet_size": "small",
                    "vaccination_document_id": foreign_doc
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "foreign vaccination doc: {}",
        resp.text()
    );

    let mut body = vehicle_body(unit);
    body["insurance_document_id"] = json!(own_doc);
    let resp = app
        .execute(
            app.post("/api/v1/registry/vehicles")
                .bearer(&token)
                .tenant(org)
                .json(&body)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::CREATED, "own doc: {}", resp.text());
    let vehicle = resp.json_value()["registration"]["id"]
        .as_str()
        .expect("registration.id")
        .to_string();

    let resp = app
        .execute(
            app.put(&format!("/api/v1/registry/vehicles/{vehicle}"))
                .bearer(&token)
                .tenant(org)
                .json(json!({ "parking_spot_id": foreign_spot }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "foreign spot: {}",
        resp.text()
    );
}

// ==================== predictive_maintenance ====================

/// Attaching a maintenance photo is a manager-tier mutation: a resident gets
/// 403 and nothing is written; a technical manager succeeds.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn add_maintenance_photo_requires_manager(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "r5-pm").await;
    let building = seed_building(&pool, org).await;
    let equipment = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO equipment_registry (organization_id, building_id, name, equipment_type, status) \
         VALUES ($1, $2, 'Boiler', 'hvac', 'operational') RETURNING id",
    )
    .bind(org)
    .bind(building)
    .fetch_one(&pool)
    .await
    .unwrap();
    let log = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO maintenance_logs (equipment_id, organization_id, maintenance_type, description) \
         VALUES ($1, $2, 'preventive', 'Annual service') RETURNING id",
    )
    .bind(equipment)
    .bind(org)
    .fetch_one(&pool)
    .await
    .unwrap();
    let (resident, _) = member(&app, org, "resident").await;
    let (tech, _) = member(&app, org, "technical_manager").await;
    let uri = format!("/api/v1/predictive-maintenance/maintenance-logs/{log}/photos");
    let body = json!({ "file_path": "s3://photos/x.jpg", "photo_type": "before" });

    let resp = app
        .execute(
            app.post(&uri)
                .bearer(&resident)
                .tenant(org)
                .json(&body)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::FORBIDDEN, "{}", resp.text());
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM maintenance_log_photos WHERE maintenance_log_id = $1",
    )
    .bind(log)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 0, "a forbidden request must not write a photo");

    let resp = app
        .execute(app.post(&uri).bearer(&tech).tenant(org).json(&body).build())
        .await;
    assert!(
        resp.status.is_success(),
        "tech manager: {} {}",
        resp.status,
        resp.text()
    );
}

// ==================== ai/equipment ====================

/// `create_maintenance` rejects a `fault_id` or `performed_by` outside the
/// caller's org with 404; in-org references are accepted.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn equipment_maintenance_refs_must_be_in_org(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "r5-eq").await;
    let other_org = seed_org(&pool, "r5-eq-o").await;
    let building = seed_building(&pool, org).await;
    let other_building = seed_building(&pool, other_org).await;
    let (token, manager_id) = member(&app, org, "manager").await;
    let (_, outsider_id) = member(&app, other_org, "manager").await;
    let equipment = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO equipment (organization_id, building_id, name, category, status) \
         VALUES ($1, $2, 'Boiler', 'hvac', 'operational') RETURNING id",
    )
    .bind(org)
    .bind(building)
    .fetch_one(&pool)
    .await
    .unwrap();
    let foreign_fault = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO faults (organization_id, building_id, reporter_id, title, description, \
             category, priority, idempotency_key) \
         VALUES ($1, $2, $3, 'Leak', 'Leak', 'plumbing'::fault_category, 'high'::fault_priority, $4) \
         RETURNING id",
    )
    .bind(other_org)
    .bind(other_building)
    .bind(outsider_id)
    .bind(Uuid::new_v4().to_string())
    .fetch_one(&pool)
    .await
    .unwrap();

    let uri = format!("/api/v1/ai/equipment/{equipment}/maintenance");
    let body = |fault: Option<Uuid>, performer: Option<Uuid>| {
        json!({
            "equipment_id": equipment,
            "maintenance_type": "preventive",
            "description": "Service",
            "fault_id": fault,
            "performed_by": performer
        })
    };

    let resp = app
        .execute(
            app.post(&uri)
                .bearer(&token)
                .tenant(org)
                .json(body(Some(foreign_fault), None))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "foreign fault: {}",
        resp.text()
    );

    let resp = app
        .execute(
            app.post(&uri)
                .bearer(&token)
                .tenant(org)
                .json(body(None, Some(outsider_id)))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "foreign performer: {}",
        resp.text()
    );

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM equipment_maintenance WHERE equipment_id = $1")
            .bind(equipment)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 0);

    let resp = app
        .execute(
            app.post(&uri)
                .bearer(&token)
                .tenant(org)
                .json(body(None, Some(manager_id)))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "in-org performer: {}",
        resp.text()
    );
}

/// `create_certification` rejects another org's building with 404.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_certification_rejects_foreign_building(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "r5-cc").await;
    let other_org = seed_org(&pool, "r5-cc-o").await;
    let own_building = seed_building(&pool, org).await;
    let foreign_building = seed_building(&pool, other_org).await;
    let (token, _) = member(&app, org, "manager").await;

    let body = |b: Uuid| json!({ "building_id": b, "program": "Leed", "level": "Gold" });
    let resp = app
        .execute(
            app.post("/api/v1/building-certifications")
                .bearer(&token)
                .tenant(org)
                .json(body(foreign_building))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::NOT_FOUND, "{}", resp.text());
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM building_certifications WHERE building_id = $1")
            .bind(foreign_building)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 0);

    let resp = app
        .execute(
            app.post("/api/v1/building-certifications")
                .bearer(&token)
                .tenant(org)
                .json(body(own_building))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::CREATED, "{}", resp.text());
}

// ==================== subscriptions ====================

/// The subscription platform-admin gate trusts the DB principal kind, not a
/// `roles: ["super_admin"]` claim in the token.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn subscription_admin_gate_ignores_jwt_roles_claim(pool: PgPool) {
    use api_server::services::JwtService;
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "r5-subs").await;
    let secret = crate::common::TestConfig::default().jwt_secret;
    let jwt = JwtService::new(&secret).expect("jwt service");

    // A tenant user (principal_kind = public) whose token claims super_admin.
    let (_, user_id) = member(&app, org, "org_admin").await;
    let forged = jwt
        .generate_access_token(
            user_id,
            "x@test.test",
            "X",
            None,
            Some(vec!["super_admin".into()]),
        )
        .expect("token");
    let resp = app
        .execute(
            app.get("/api/v1/subscriptions/statistics")
                .bearer(&forged)
                .tenant(org)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::FORBIDDEN, "{}", resp.text());

    // A real platform principal passes without any roles claim.
    let platform_id = crate::common::seed_platform_user(
        &pool,
        &format!("r5-platform-{}@test.test", Uuid::new_v4()),
    )
    .await;
    seed_membership(&pool, org, platform_id, "org_admin").await;
    let token = jwt
        .generate_access_token(platform_id, "p@test.test", "P", None, None)
        .expect("token");
    let resp = app
        .execute(
            app.get("/api/v1/subscriptions/statistics")
                .bearer(&token)
                .tenant(org)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
}

// ==================== features ====================

/// When the host pinned the request to an org (`ResolvedTenant`), features
/// uses that org, and an `X-Tenant-ID` naming another org is a 403.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn features_respect_resolved_tenant(pool: PgPool) {
    use api_core::middleware::host_tenant::{ResolvedTenant, TenantSource};
    let app = TestApp::new(pool.clone()).await;
    let host_org = seed_org(&pool, "r5-feat-host").await;
    let other_org = seed_org(&pool, "r5-feat-other").await;
    let (token, user_id) = member(&app, host_org, "resident").await;
    seed_membership(&pool, other_org, user_id, "resident").await;
    let body = json!({ "feature_key": "new_dashboard", "event_type": "access", "metadata": {} });
    let pinned = ResolvedTenant {
        organization_id: host_org,
        source: TenantSource::Subdomain,
    };

    let mut req = app
        .post("/api/v1/features/analytics/event")
        .bearer(&token)
        .tenant(other_org)
        .json(&body)
        .build();
    req.extensions_mut().insert(pinned);
    let resp = app.execute(req).await;
    assert_eq!(resp.status, StatusCode::FORBIDDEN, "{}", resp.text());

    let mut req = app
        .post("/api/v1/features/analytics/event")
        .bearer(&token)
        .json(&body)
        .build();
    req.extensions_mut().insert(pinned);
    let resp = app.execute(req).await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());

    let orgs: Vec<Uuid> =
        sqlx::query_scalar("SELECT organization_id FROM feature_usage_events WHERE user_id = $1")
            .bind(user_id)
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(
        orgs,
        vec![host_org],
        "the event is recorded for the host org"
    );
}

// ==================== onboarding ====================

/// Role-targeted tours use the role in the *current* org only: a manager in
/// org B who is a resident in org A does not see manager tours in org A.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn onboarding_roles_are_scoped_to_current_org(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "r5-onb-a").await;
    let org_b = seed_org(&pool, "r5-onb-b").await;
    let (token, user_id) = member(&app, org_a, "resident").await;
    seed_membership(&pool, org_b, user_id, "manager").await;
    let tour_id = format!("r5-mgr-{}", &Uuid::new_v4().simple().to_string()[..12]);
    sqlx::query(
        "INSERT INTO onboarding_tours (tour_id, name, target_roles) VALUES ($1, 'Mgr', ARRAY['manager'])",
    )
    .bind(&tour_id)
    .execute(&pool)
    .await
    .unwrap();

    let sees_tour = |org: Uuid| {
        let app = &app;
        let token = token.clone();
        let tour_id = tour_id.clone();
        async move {
            let resp = app
                .execute(
                    app.get("/api/v1/onboarding/tours")
                        .bearer(&token)
                        .tenant(org)
                        .build(),
                )
                .await;
            assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
            resp.json_value()
                .as_array()
                .expect("tours array")
                .iter()
                .any(|t| t["tour"]["tour_id"] == tour_id.as_str())
        }
    };

    assert!(
        !sees_tour(org_a).await,
        "resident in org A must not get the manager tour"
    );
    assert!(
        sees_tour(org_b).await,
        "manager in org B gets the manager tour"
    );
}

// ==================== data_residency ====================

/// Residency reads are open to every org member and do not persist anything;
/// writes (configure, update, log access, compliance run) are admin-only —
/// a manager or resident gets 403.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn data_residency_reads_for_members_writes_for_admins(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "r5-res").await;
    let (resident, _) = member(&app, org, "resident").await;
    let (manager, _) = member(&app, org, "manager").await;
    let (admin, _) = member(&app, org, "org_admin").await;
    let config = json!({
        "primary_region": "eu_central",
        "backup_region": "ch_north",
        "allow_cross_region_access": false,
        "data_type_overrides": [],
        "compliance_notes": null
    });

    for (who, label) in [(&resident, "resident"), (&manager, "manager")] {
        let resp = app
            .execute(
                app.get("/api/v1/data-residency/config")
                    .bearer(who)
                    .tenant(org)
                    .build(),
            )
            .await;
        assert_eq!(resp.status, StatusCode::OK, "{label} read: {}", resp.text());
        assert_eq!(resp.json_value()["primary_region"], "eu_west");

        let resp = app
            .execute(
                app.post("/api/v1/data-residency/config")
                    .bearer(who)
                    .tenant(org)
                    .json(&config)
                    .build(),
            )
            .await;
        assert_eq!(resp.status, StatusCode::FORBIDDEN, "{label} configure");

        let resp = app
            .execute(
                app.post("/api/v1/data-residency/compliance/verify")
                    .bearer(who)
                    .tenant(org)
                    .json(json!({ "include_details": true }))
                    .build(),
            )
            .await;
        assert_eq!(resp.status, StatusCode::FORBIDDEN, "{label} compliance run");
    }

    let rows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM data_residency_config WHERE organization_id = $1")
            .bind(org)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(rows, 0, "member reads must not persist a configuration");

    let resp = app
        .execute(
            app.post("/api/v1/data-residency/config")
                .bearer(&admin)
                .tenant(org)
                .json(&config)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "admin configure: {}",
        resp.text()
    );

    let resp = app
        .execute(
            app.get("/api/v1/data-residency/config")
                .bearer(&resident)
                .tenant(org)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    assert_eq!(resp.json_value()["primary_region"], "eu_central");
}

// ==================== facilities / person_months error paths ====================

/// Error paths in facilities and person_months release the RLS connection
/// before returning. The release itself is not observable over HTTP, so this
/// drives each refactored error path (authorization failure, nested-id 404,
/// validation 400 after a held connection) and checks the status codes, then
/// that a follow-up request on the same pool still succeeds.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn facilities_and_person_months_error_paths(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "r5-fac").await;
    let building = seed_building(&pool, org).await;
    let unit = seed_unit(&pool, building, "F-1").await;
    let (resident, _) = member(&app, org, "resident").await;
    let (manager, _) = member(&app, org, "manager").await;

    // authorize_building: role failure -> 403.
    let resp = app
        .execute(
            app.post(&format!("/api/v1/buildings/{building}/facilities"))
                .bearer(&resident)
                .tenant(org)
                .json(json!({
                    "name": "Gym",
                    "facility_type": "gym",
                    "is_bookable": true,
                    "requires_approval": false
                }))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::FORBIDDEN, "{}", resp.text());

    // create_booking: facility not in building -> 404 after authorization.
    let resp = app
        .execute(
            app.post(&format!(
                "/api/v1/buildings/{building}/facilities/{}/bookings",
                Uuid::new_v4()
            ))
            .bearer(&resident)
            .tenant(org)
            .json(json!({
                "start_time": "2030-01-01T10:00:00Z",
                "end_time": "2030-01-01T11:00:00Z"
            }))
            .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::NOT_FOUND, "{}", resp.text());

    // upsert_person_month: validation failure while the connection is held.
    let resp = app
        .execute(
            app.post(&format!(
                "/api/v1/buildings/{building}/units/{unit}/person-months"
            ))
            .bearer(&manager)
            .tenant(org)
            .json(json!({ "year": 2030, "month": 13, "count": 1, "source": "manual" }))
            .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::BAD_REQUEST, "{}", resp.text());

    // bulk upsert: invalid month -> 400.
    let resp = app
        .execute(
            app.post(&format!("/api/v1/buildings/{building}/person-months/bulk"))
                .bearer(&manager)
                .tenant(org)
                .json(json!({ "year": 2030, "month": 0, "entries": [] }))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::BAD_REQUEST, "{}", resp.text());

    // The pool is still healthy for a normal request.
    let resp = app
        .execute(
            app.get(&format!("/api/v1/buildings/{building}/facilities"))
                .bearer(&resident)
                .tenant(org)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
}
