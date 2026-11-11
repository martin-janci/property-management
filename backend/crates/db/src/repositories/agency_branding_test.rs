//! Regression tests for `AgencyBrandingRepository::upsert_rls` three-state
//! partial-update (clear) semantics — the organization-keyed sibling of the
//! reality-server agency-branding fix (#3430), issue #3431.
//!
//! The original `upsert_rls` upserted with `COALESCE(EXCLUDED.x,
//! agency_branding.x)`, so a client could NEVER clear a branding field back to
//! NULL — an explicit `null` bound as SQL NULL and `COALESCE` kept the old
//! value, making a clear indistinguishable from "no change". The fix gives each
//! nullable field three-state semantics:
//!   * absent            → leave the column unchanged,
//!   * explicit `null`   → clear the column to NULL,
//!   * explicit `""`     → clear the column to NULL,
//!   * a value           → set the column.
//!
//! These tests build the `UpdateOrganizationBranding` payload by deserializing
//! JSON (the real request surface) rather than constructing the struct with its
//! internal field types, so the same test compiles against the pre-fix code —
//! where `logo_url: null` collapses to `None`, the COALESCE keeps the old value
//! and the clear assertions FAIL — and against the post-fix code, where
//! `double_option` preserves the explicit null and the clears PASS. That is the
//! IG3 "fails-before / passes-after" proof.

#[cfg(test)]
mod tests {
    use crate::models::agency_branding::UpdateOrganizationBranding;
    use crate::repositories::agency_branding::AgencyBrandingRepository;
    use serde_json::json;
    use sqlx::Row;
    use uuid::Uuid;

    /// A super-admin RLS context clears the `agency_branding` FOR ALL policy
    /// (`is_super_admin()`), matching the admin update handler. Must be set on
    /// the same connection that then runs the upsert / read-back.
    async fn set_super_admin(conn: &mut sqlx::PgConnection) {
        crate::tenant_context::set_request_context(&mut *conn, None, None, true)
            .await
            .expect("set super-admin RLS context");
    }

    async fn seed_org(pool: &sqlx::PgPool, slug: &str) -> Uuid {
        // `contact_email` is NOT NULL on `organizations`.
        sqlx::query(
            "INSERT INTO organizations (name, slug, contact_email) \
             VALUES ($1, $2, $3) RETURNING id",
        )
        .bind(slug)
        .bind(slug)
        .bind(format!("{slug}@example.com"))
        .fetch_one(pool)
        .await
        .unwrap()
        .get("id")
    }

    /// Read one branding column straight from the row (static query per column —
    /// no interpolated SQL). Runs on `conn`, which already holds super-admin
    /// context so the SELECT passes RLS.
    async fn fetch_col(conn: &mut sqlx::PgConnection, org_id: Uuid, col: &str) -> Option<String> {
        let sql: &'static str = match col {
            "logo_url" => "SELECT logo_url FROM agency_branding WHERE organization_id = $1",
            "banner_url" => "SELECT banner_url FROM agency_branding WHERE organization_id = $1",
            "primary_color" => {
                "SELECT primary_color FROM agency_branding WHERE organization_id = $1"
            }
            "secondary_color" => {
                "SELECT secondary_color FROM agency_branding WHERE organization_id = $1"
            }
            "font_family" => "SELECT font_family FROM agency_branding WHERE organization_id = $1",
            other => panic!("fetch_col: unsupported column {other}"),
        };
        sqlx::query_scalar::<_, Option<String>>(sql)
            .bind(org_id)
            .fetch_one(&mut *conn)
            .await
            .unwrap_or_else(|e| panic!("fetch_col({col}): {e}"))
    }

    fn payload(body: serde_json::Value) -> UpdateOrganizationBranding {
        serde_json::from_value(body).expect("deserialize UpdateOrganizationBranding")
    }

    /// Core regression: an explicit `null` clears a previously-set field, while
    /// an omitted field is left untouched. Under the old COALESCE upsert the
    /// cleared field retained its value — this test fails on the pre-fix code.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn explicit_null_clears_field_absent_leaves_unchanged(pool: sqlx::PgPool) {
        let org_id = seed_org(&pool, "branding-null").await;
        let repo = AgencyBrandingRepository::new(pool.clone());
        let mut conn = pool.acquire().await.unwrap();
        set_super_admin(&mut conn).await;

        // 1. Set logo_url and banner_url.
        repo.upsert_rls(
            &mut *conn,
            org_id,
            payload(json!({
                "logo_url": "https://cdn.example.com/logo.png",
                "banner_url": "https://cdn.example.com/banner.png"
            })),
        )
        .await
        .expect("initial set");
        assert_eq!(
            fetch_col(&mut conn, org_id, "logo_url").await.as_deref(),
            Some("https://cdn.example.com/logo.png")
        );
        assert_eq!(
            fetch_col(&mut conn, org_id, "banner_url").await.as_deref(),
            Some("https://cdn.example.com/banner.png")
        );

        // 2. Clear logo_url with an explicit null; omit banner_url entirely.
        repo.upsert_rls(&mut *conn, org_id, payload(json!({ "logo_url": null })))
            .await
            .expect("clear logo_url");

        // logo_url must now be NULL (cleared) ...
        assert_eq!(
            fetch_col(&mut conn, org_id, "logo_url").await,
            None,
            "explicit null must clear logo_url back to NULL (COALESCE-no-clear regression)"
        );
        // ... and banner_url must be untouched (absent field = leave unchanged).
        assert_eq!(
            fetch_col(&mut conn, org_id, "banner_url").await.as_deref(),
            Some("https://cdn.example.com/banner.png"),
            "an omitted field must be left unchanged"
        );
    }

    /// An empty string also clears the field (stored as NULL), so the frontend
    /// can clear via `""` as well as `null`.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn empty_string_clears_field(pool: sqlx::PgPool) {
        let org_id = seed_org(&pool, "branding-empty").await;
        let repo = AgencyBrandingRepository::new(pool.clone());
        let mut conn = pool.acquire().await.unwrap();
        set_super_admin(&mut conn).await;

        repo.upsert_rls(
            &mut *conn,
            org_id,
            payload(json!({ "primary_color": "#ff0000" })),
        )
        .await
        .expect("set primary_color");
        assert_eq!(
            fetch_col(&mut conn, org_id, "primary_color")
                .await
                .as_deref(),
            Some("#ff0000")
        );

        repo.upsert_rls(&mut *conn, org_id, payload(json!({ "primary_color": "" })))
            .await
            .expect("clear primary_color via empty string");
        assert_eq!(
            fetch_col(&mut conn, org_id, "primary_color").await,
            None,
            "empty string must clear primary_color back to NULL"
        );
    }

    /// Negative control: setting a field to a new value still works (the fix
    /// does not break the set path), and clearing one field does not disturb a
    /// sibling that is set in the same request.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn set_and_clear_in_same_request(pool: sqlx::PgPool) {
        let org_id = seed_org(&pool, "branding-mixed").await;
        let repo = AgencyBrandingRepository::new(pool.clone());
        let mut conn = pool.acquire().await.unwrap();
        set_super_admin(&mut conn).await;

        // Seed both colors.
        repo.upsert_rls(
            &mut *conn,
            org_id,
            payload(json!({ "primary_color": "#111111", "secondary_color": "#222222" })),
        )
        .await
        .expect("seed colors");

        // Clear primary, re-set secondary, in one request.
        repo.upsert_rls(
            &mut *conn,
            org_id,
            payload(json!({ "primary_color": null, "secondary_color": "#333333" })),
        )
        .await
        .expect("mixed set+clear");

        assert_eq!(
            fetch_col(&mut conn, org_id, "primary_color").await,
            None,
            "primary_color must be cleared"
        );
        assert_eq!(
            fetch_col(&mut conn, org_id, "secondary_color")
                .await
                .as_deref(),
            Some("#333333"),
            "secondary_color must be updated to the new value"
        );
    }
}
