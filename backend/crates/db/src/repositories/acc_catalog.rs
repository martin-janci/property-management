//! Repository for ACC product & price-list catalog (EPIC-ACC-04). OWNED BY: BE-Contacts+Catalog.
//!
//! RLS-aware, generic-executor pattern (see
//! [`crate::repositories::accounting::AccountingRepository`]). Backs
//! `acc_catalog_item`, `acc_item_category`, `acc_price_level` (migration 00195).
//!
//! Column names verified against migration 00195. Per-tenant isolation is
//! enforced by RLS — reads never add a `tenant_id = $` clause, but every INSERT
//! sets `tenant_id` to the request tenant.

use crate::models::acc_catalog::{AccCatalogItem, AccItemCategory, AccPriceLevel};
use crate::DbPool;
use sqlx::{Executor, Postgres};
use uuid::Uuid;

/// Repository for ACC catalog (EPIC-ACC-04).
#[derive(Debug, Clone)]
pub struct AccCatalogRepository {
    pub pool: DbPool,
}

impl AccCatalogRepository {
    /// Create a new catalog repository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // --- Items (UC-ACC-04.1/.3/.7) ---

    /// List catalog items for the current tenant. When `include_inactive` is
    /// false, deactivated items are hidden from pickers — but existing documents
    /// keep their snapshotted line values, so a deactivated item never alters
    /// past documents (UC-ACC-04.1/.3).
    pub async fn list_items_rls<'e, E>(
        &self,
        executor: E,
        include_inactive: bool,
    ) -> Result<Vec<AccCatalogItem>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, AccCatalogItem>(
            "SELECT * FROM acc_catalog_item WHERE ($1 OR is_active) ORDER BY name ASC",
        )
        .bind(include_inactive)
        .fetch_all(executor)
        .await
    }

    /// List items in a single category (UC-ACC-04.4 reporting/filter).
    pub async fn list_items_by_category_rls<'e, E>(
        &self,
        executor: E,
        category_id: Uuid,
    ) -> Result<Vec<AccCatalogItem>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, AccCatalogItem>(
            "SELECT * FROM acc_catalog_item WHERE category_id = $1 ORDER BY name ASC",
        )
        .bind(category_id)
        .fetch_all(executor)
        .await
    }

    /// Find a catalog item by id (UC-ACC-04.1).
    pub async fn find_item_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<AccCatalogItem>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, AccCatalogItem>("SELECT * FROM acc_catalog_item WHERE id = $1")
            .bind(id)
            .fetch_optional(executor)
            .await
    }

    /// Find a catalog item by its (tenant-unique) code (UC-ACC-04.6 import
    /// dedupe). RLS scopes to the active company; the `UNIQUE (tenant_id, code)`
    /// constraint guarantees at most one match.
    pub async fn find_item_by_code_rls<'e, E>(
        &self,
        executor: E,
        code: &str,
    ) -> Result<Option<AccCatalogItem>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, AccCatalogItem>("SELECT * FROM acc_catalog_item WHERE code = $1")
            .bind(code)
            .fetch_optional(executor)
            .await
    }

    /// Create a catalog item (UC-ACC-04.1/.8 inline quick-add). `tenant_id` from
    /// the request; `id`/timestamps DB-defaulted. Inline quick-add from a
    /// document line persists a FULL catalog item here so it is reusable later.
    pub async fn create_item_rls<'e, E>(
        &self,
        executor: E,
        item: AccCatalogItem,
    ) -> Result<AccCatalogItem, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, AccCatalogItem>(
            "INSERT INTO acc_catalog_item (\
                tenant_id, code, name, description, kind, unit_id, vat_rate, \
                unit_price, price_is_gross, category_id, stock_tracked, is_active) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) \
             RETURNING *",
        )
        .bind(item.tenant_id)
        .bind(&item.code)
        .bind(&item.name)
        .bind(&item.description)
        .bind(&item.kind)
        .bind(item.unit_id)
        .bind(item.vat_rate)
        .bind(item.unit_price)
        .bind(item.price_is_gross)
        .bind(item.category_id)
        .bind(item.stock_tracked)
        .bind(item.is_active)
        .fetch_one(executor)
        .await
    }

    /// Update a catalog item (UC-ACC-04.1). Editing an item does NOT mutate past
    /// documents — those carry their own snapshotted line values.
    pub async fn update_item_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        item: AccCatalogItem,
    ) -> Result<Option<AccCatalogItem>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, AccCatalogItem>(
            "UPDATE acc_catalog_item SET \
                code = $1, name = $2, description = $3, kind = $4, unit_id = $5, \
                vat_rate = $6, unit_price = $7, price_is_gross = $8, category_id = $9, \
                stock_tracked = $10, is_active = $11, updated_at = NOW() \
             WHERE id = $12 \
             RETURNING *",
        )
        .bind(&item.code)
        .bind(&item.name)
        .bind(&item.description)
        .bind(&item.kind)
        .bind(item.unit_id)
        .bind(item.vat_rate)
        .bind(item.unit_price)
        .bind(item.price_is_gross)
        .bind(item.category_id)
        .bind(item.stock_tracked)
        .bind(id)
        .fetch_optional(executor)
        .await
    }

    /// Soft-deactivate a catalog item (UC-ACC-04.1). Deactivate != delete:
    /// referenced items stay in the table so past documents are never broken.
    pub async fn deactivate_item_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<(), sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query("UPDATE acc_catalog_item SET is_active = false, updated_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(executor)
            .await?;
        Ok(())
    }

    // --- Categories (UC-ACC-04.4) ---

    /// List categories for the current tenant (UC-ACC-04.4).
    pub async fn list_categories_rls<'e, E>(
        &self,
        executor: E,
    ) -> Result<Vec<AccItemCategory>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, AccItemCategory>("SELECT * FROM acc_item_category ORDER BY name ASC")
            .fetch_all(executor)
            .await
    }

    /// Create a category (UC-ACC-04.4). Optional `parent_id` enables a tree.
    pub async fn create_category_rls<'e, E>(
        &self,
        executor: E,
        category: AccItemCategory,
    ) -> Result<AccItemCategory, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, AccItemCategory>(
            "INSERT INTO acc_item_category (tenant_id, name, parent_id) \
             VALUES ($1, $2, $3) \
             RETURNING *",
        )
        .bind(category.tenant_id)
        .bind(&category.name)
        .bind(category.parent_id)
        .fetch_one(executor)
        .await
    }

    // --- Price levels (UC-ACC-04.2) ---

    /// List price levels for a catalog item (UC-ACC-04.2).
    pub async fn list_price_levels_rls<'e, E>(
        &self,
        executor: E,
        item_id: Uuid,
    ) -> Result<Vec<AccPriceLevel>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, AccPriceLevel>(
            "SELECT * FROM acc_price_level WHERE item_id = $1 ORDER BY name ASC",
        )
        .bind(item_id)
        .fetch_all(executor)
        .await
    }

    /// Find one named price level for an item (UC-ACC-04.2 level resolution by a
    /// contact's `default_price_level`). Returns None when the contact's default
    /// level is unset or has no matching row, so the caller falls back to the
    /// item's base `unit_price`.
    pub async fn find_price_level_by_name_rls<'e, E>(
        &self,
        executor: E,
        item_id: Uuid,
        name: &str,
    ) -> Result<Option<AccPriceLevel>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, AccPriceLevel>(
            "SELECT * FROM acc_price_level WHERE item_id = $1 AND name = $2",
        )
        .bind(item_id)
        .bind(name)
        .fetch_optional(executor)
        .await
    }

    /// Create a price level for a catalog item (UC-ACC-04.2). `tenant_id` from
    /// the request; unique on `(tenant_id, item_id, name)`.
    pub async fn create_price_level_rls<'e, E>(
        &self,
        executor: E,
        level: AccPriceLevel,
    ) -> Result<AccPriceLevel, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, AccPriceLevel>(
            "INSERT INTO acc_price_level (tenant_id, item_id, name, price, price_is_gross, currency) \
             VALUES ($1, $2, $3, $4, $5, $6) \
             RETURNING *",
        )
        .bind(level.tenant_id)
        .bind(level.item_id)
        .bind(&level.name)
        .bind(level.price)
        .bind(level.price_is_gross)
        .bind(&level.currency)
        .fetch_one(executor)
        .await
    }
}

#[cfg(test)]
mod tests {
    // The catalog repository is thin SQL over an RLS Postgres which the
    // write-only phase cannot run. The price-resolution semantics it enables
    // (net/gross + level selection) are exercised in the route module's tests
    // and in accounting-core's money/vat tests; here we only assert the
    // module compiles and the public surface is stable, which the crate build
    // verifies. No DB-free logic lives in this file to unit-test directly.
    #[test]
    fn repository_constructs_without_panicking_in_principle() {
        // Compile-time guard: the type exists with the expected ctor shape.
        fn _assert_ctor(p: crate::DbPool) -> super::AccCatalogRepository {
            super::AccCatalogRepository::new(p)
        }
    }
}
