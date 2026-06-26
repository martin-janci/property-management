//! Repository for ACC contacts & CRM (EPIC-ACC-03). OWNED BY: BE-Contacts+Catalog.
//!
//! RLS-aware, generic-executor pattern (see
//! [`crate::repositories::accounting::AccountingRepository`]). Builds on the
//! existing 00184 `contact` table plus the 00195 CRM extensions
//! (addresses, tags, per-contact defaults via [`AccContactExt`]).
//!
//! Column names verified against migrations 00184 (`contact`) + 00195
//! (`contact` ALTERs, `acc_contact_address`, `acc_contact_tag`). Per-tenant
//! isolation is enforced by RLS — SELECT statements never add a
//! `tenant_id = $` clause, but INSERT statements MUST set `tenant_id`.

use crate::models::acc_contacts_ext::{AccContactAddress, AccContactExt, AccContactTag};
use crate::models::accounting::Invoice;
use crate::DbPool;
use rust_decimal::Decimal;
use sqlx::{Executor, Postgres};
use uuid::Uuid;

/// Stable projection of the full extended `contact` column set, in the exact
/// order [`AccContactExt`] declares its fields. Reused by every read so a
/// schema/struct mismatch surfaces in one place. (UC-ACC-03.1)
const CONTACT_EXT_COLUMNS: &str = "id, tenant_id, name, ico, dic, email, phone, address, iban, \
     created_at, updated_at, kind, is_active, default_currency, default_price_level, \
     default_payment_terms_days, default_discount_pct, default_language, vat_verified_at, \
     vat_verified_valid, credit_limit, notes, merged_into_id";

/// Receivables summary for a contact (UC-ACC-03.11). Derived from open
/// documents; `credit_limit` is the optional limit copied from the contact so
/// callers can warn at document time without a second query.
#[derive(Debug, Clone)]
pub struct ContactReceivables {
    /// Sum of (total_amount - paid_amount) over the contact's non-settled
    /// invoices (issued/sent/partially_paid/overdue).
    pub outstanding: Decimal,
    /// Optional credit limit from the contact (UC-ACC-03.11).
    pub credit_limit: Option<Decimal>,
}

/// Repository for ACC contacts/CRM (EPIC-ACC-03).
#[derive(Debug, Clone)]
pub struct AccContactsRepository {
    pub pool: DbPool,
}

impl AccContactsRepository {
    /// Create a new contacts repository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // --- Contact CRUD with extended columns (UC-ACC-03.1/.5/.9) ---

    /// List contacts (extended projection) for the current tenant. When
    /// `include_inactive` is false we hide deactivated and merged-away contacts
    /// so pickers only show live parties — historical documents that reference
    /// them are unaffected (UC-ACC-03.1).
    pub async fn list_contacts_ext_rls<'e, E>(
        &self,
        executor: E,
        include_inactive: bool,
    ) -> Result<Vec<AccContactExt>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // A merged contact (merged_into_id IS NOT NULL) is always hidden from
        // the live directory regardless of is_active.
        let sql = format!(
            "SELECT {CONTACT_EXT_COLUMNS} FROM contact \
             WHERE merged_into_id IS NULL AND ($1 OR is_active) \
             ORDER BY name ASC"
        );
        sqlx::query_as::<_, AccContactExt>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(include_inactive)
            .fetch_all(executor)
            .await
    }

    /// Find a single contact (extended projection) by id (UC-ACC-03.1).
    pub async fn find_contact_ext_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<AccContactExt>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let sql = format!("SELECT {CONTACT_EXT_COLUMNS} FROM contact WHERE id = $1");
        sqlx::query_as::<_, AccContactExt>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(id)
            .fetch_optional(executor)
            .await
    }

    /// Create a contact with extended defaults (UC-ACC-03.1/.9). `tenant_id` on
    /// the supplied struct MUST be the request tenant (RLS only filters reads).
    /// `id`/`created_at`/`updated_at` are DB-defaulted; we ignore the struct's
    /// placeholder values and return the row the DB actually wrote.
    pub async fn create_contact_ext_rls<'e, E>(
        &self,
        executor: E,
        contact: AccContactExt,
    ) -> Result<AccContactExt, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let sql = format!(
            "INSERT INTO contact (\
                tenant_id, name, ico, dic, email, phone, address, iban, \
                kind, is_active, default_currency, default_price_level, \
                default_payment_terms_days, default_discount_pct, default_language, \
                vat_verified_at, vat_verified_valid, credit_limit, notes) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19) \
             RETURNING {CONTACT_EXT_COLUMNS}"
        );
        sqlx::query_as::<_, AccContactExt>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(contact.tenant_id)
            .bind(&contact.name)
            .bind(&contact.ico)
            .bind(&contact.dic)
            .bind(&contact.email)
            .bind(&contact.phone)
            .bind(&contact.address)
            .bind(&contact.iban)
            .bind(&contact.kind)
            .bind(contact.is_active)
            .bind(&contact.default_currency)
            .bind(&contact.default_price_level)
            .bind(contact.default_payment_terms_days)
            .bind(contact.default_discount_pct)
            .bind(&contact.default_language)
            .bind(contact.vat_verified_at)
            .bind(contact.vat_verified_valid)
            .bind(contact.credit_limit)
            .bind(&contact.notes)
            .fetch_one(executor)
            .await
    }

    /// Update a contact's extended columns (UC-ACC-03.1/.9). All editable
    /// fields are overwritten from the supplied struct (handler merges partial
    /// PATCH semantics before calling — see routes/contacts.rs).
    pub async fn update_contact_ext_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        contact: AccContactExt,
    ) -> Result<Option<AccContactExt>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let sql = format!(
            "UPDATE contact SET \
                name = $1, ico = $2, dic = $3, email = $4, phone = $5, address = $6, iban = $7, \
                kind = $8, is_active = $9, default_currency = $10, default_price_level = $11, \
                default_payment_terms_days = $12, default_discount_pct = $13, default_language = $14, \
                vat_verified_at = $15, vat_verified_valid = $16, credit_limit = $17, notes = $18, \
                updated_at = NOW() \
             WHERE id = $19 \
             RETURNING {CONTACT_EXT_COLUMNS}"
        );
        sqlx::query_as::<_, AccContactExt>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(&contact.name)
            .bind(&contact.ico)
            .bind(&contact.dic)
            .bind(&contact.email)
            .bind(&contact.phone)
            .bind(&contact.address)
            .bind(&contact.iban)
            .bind(&contact.kind)
            .bind(contact.is_active)
            .bind(&contact.default_currency)
            .bind(&contact.default_price_level)
            .bind(contact.default_payment_terms_days)
            .bind(contact.default_discount_pct)
            .bind(&contact.default_language)
            .bind(contact.vat_verified_at)
            .bind(contact.vat_verified_valid)
            .bind(contact.credit_limit)
            .bind(&contact.notes)
            .bind(id)
            .fetch_optional(executor)
            .await
    }

    /// Persist a VAT-verification result on the contact (UC-ACC-03.2/.3). Stores
    /// validity + the verification timestamp so a third-party outage never
    /// blocks the save path; callers update this out-of-band once a lookup
    /// returns (graceful degradation).
    pub async fn set_vat_verification_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        valid: bool,
    ) -> Result<(), sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query(
            "UPDATE contact SET vat_verified_valid = $1, vat_verified_at = NOW(), updated_at = NOW() \
             WHERE id = $2",
        )
        .bind(valid)
        .bind(id)
        .execute(executor)
        .await?;
        Ok(())
    }

    /// Soft-deactivate a contact (UC-ACC-03.1). We NEVER hard-delete a referenced
    /// contact — historical documents must keep pointing at it. Deactivated
    /// contacts disappear from pickers via [`Self::list_contacts_ext_rls`].
    pub async fn deactivate_contact_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<(), sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query("UPDATE contact SET is_active = false, updated_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(executor)
            .await?;
        Ok(())
    }

    /// Merge `source` into `target` (UC-ACC-03.5): re-point every foreign
    /// reference to `source` onto `target`, then mark `source` merged + inactive
    /// so no data is lost and the source vanishes from pickers. Runs inside one
    /// transaction (`&mut PgConnection`) so the re-key + flag are atomic — a
    /// partial merge can never leave dangling references.
    ///
    /// References re-pointed: `invoice.contact_id` (documents/history),
    /// `acc_contact_address.contact_id`, `acc_contact_tag.contact_id`.
    /// Tags are de-duplicated against the target's existing tags first to honour
    /// the `UNIQUE (tenant_id, contact_id, tag)` constraint.
    pub async fn merge_contacts_rls(
        &self,
        conn: &mut sqlx::PgConnection,
        source_id: Uuid,
        target_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        // 1) Re-point all invoices (and therefore their payment/communication
        //    history, which hang off invoice_id) to the surviving contact.
        sqlx::query("UPDATE invoice SET contact_id = $1, updated_at = NOW() WHERE contact_id = $2")
            .bind(target_id)
            .bind(source_id)
            .execute(&mut *conn)
            .await?;

        // 2) Move addresses to the target contact.
        sqlx::query(
            "UPDATE acc_contact_address SET contact_id = $1, updated_at = NOW() WHERE contact_id = $2",
        )
        .bind(target_id)
        .bind(source_id)
        .execute(&mut *conn)
        .await?;

        // 3) Move tags, dropping any that already exist on the target so the
        //    unique (tenant_id, contact_id, tag) constraint is never violated.
        sqlx::query(
            "DELETE FROM acc_contact_tag s \
             WHERE s.contact_id = $1 \
               AND EXISTS (SELECT 1 FROM acc_contact_tag t \
                           WHERE t.contact_id = $2 AND t.tag = s.tag)",
        )
        .bind(source_id)
        .bind(target_id)
        .execute(&mut *conn)
        .await?;
        sqlx::query("UPDATE acc_contact_tag SET contact_id = $1 WHERE contact_id = $2")
            .bind(target_id)
            .bind(source_id)
            .execute(&mut *conn)
            .await?;

        // 4) Mark the source merged + inactive (soft, never deleted).
        sqlx::query(
            "UPDATE contact SET merged_into_id = $1, is_active = false, updated_at = NOW() \
             WHERE id = $2",
        )
        .bind(target_id)
        .bind(source_id)
        .execute(&mut *conn)
        .await?;

        Ok(())
    }

    // --- Addresses (UC-ACC-03.4) ---

    /// List addresses for a contact, billing first then delivery, so the UI can
    /// default-select a billing address (UC-ACC-03.4).
    pub async fn list_addresses_rls<'e, E>(
        &self,
        executor: E,
        contact_id: Uuid,
    ) -> Result<Vec<AccContactAddress>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, AccContactAddress>(
            "SELECT * FROM acc_contact_address WHERE contact_id = $1 \
             ORDER BY kind ASC, is_default DESC, created_at ASC",
        )
        .bind(contact_id)
        .fetch_all(executor)
        .await
    }

    /// Create an address for a contact (UC-ACC-03.4). `tenant_id` set from the
    /// request; `id`/timestamps DB-defaulted.
    pub async fn create_address_rls<'e, E>(
        &self,
        executor: E,
        address: AccContactAddress,
    ) -> Result<AccContactAddress, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, AccContactAddress>(
            "INSERT INTO acc_contact_address (\
                tenant_id, contact_id, kind, label, street, city, postal_code, country, is_default) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
             RETURNING *",
        )
        .bind(address.tenant_id)
        .bind(address.contact_id)
        .bind(&address.kind)
        .bind(&address.label)
        .bind(&address.street)
        .bind(&address.city)
        .bind(&address.postal_code)
        .bind(&address.country)
        .bind(address.is_default)
        .fetch_one(executor)
        .await
    }

    // --- Tags (UC-ACC-03.6) ---

    /// List tags for a contact (UC-ACC-03.6).
    pub async fn list_tags_rls<'e, E>(
        &self,
        executor: E,
        contact_id: Uuid,
    ) -> Result<Vec<AccContactTag>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, AccContactTag>(
            "SELECT * FROM acc_contact_tag WHERE contact_id = $1 ORDER BY tag ASC",
        )
        .bind(contact_id)
        .fetch_all(executor)
        .await
    }

    /// List every contact id carrying `tag` for the tenant (UC-ACC-03.6 segment
    /// filter / bulk-action scoping). RLS scopes the rows to the active company.
    pub async fn list_contact_ids_by_tag_rls<'e, E>(
        &self,
        executor: E,
        tag: &str,
    ) -> Result<Vec<Uuid>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let rows: Vec<(Uuid,)> =
            sqlx::query_as("SELECT contact_id FROM acc_contact_tag WHERE tag = $1")
                .bind(tag)
                .fetch_all(executor)
                .await?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// Add a tag to a contact (UC-ACC-03.6). Idempotent on the unique
    /// `(tenant_id, contact_id, tag)` tuple — re-tagging returns the existing
    /// row rather than erroring.
    pub async fn add_tag_rls<'e, E>(
        &self,
        executor: E,
        tag: AccContactTag,
    ) -> Result<AccContactTag, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, AccContactTag>(
            "INSERT INTO acc_contact_tag (tenant_id, contact_id, tag) \
             VALUES ($1, $2, $3) \
             ON CONFLICT (tenant_id, contact_id, tag) \
             DO UPDATE SET tag = EXCLUDED.tag \
             RETURNING *",
        )
        .bind(tag.tenant_id)
        .bind(tag.contact_id)
        .bind(&tag.tag)
        .fetch_one(executor)
        .await
    }

    // --- Transaction history & receivables (UC-ACC-03.7/.8/.11) ---

    /// All documents (invoices) for a contact, newest first (UC-ACC-03.7
    /// transaction history). Communication history (UC-ACC-03.8) is sourced from
    /// the audit log via the platform repo, so it is not duplicated here.
    pub async fn list_contact_invoices_rls<'e, E>(
        &self,
        executor: E,
        contact_id: Uuid,
    ) -> Result<Vec<Invoice>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, Invoice>(
            "SELECT * FROM invoice WHERE contact_id = $1 \
             ORDER BY issue_date DESC, created_at DESC",
        )
        .bind(contact_id)
        .fetch_all(executor)
        .await
    }

    /// Outstanding receivables for a contact (UC-ACC-03.11). Sums the unpaid
    /// remainder `(total_amount - paid_amount)` over open invoices and returns
    /// it alongside the contact's optional credit limit so a handler can warn on
    /// a limit breach at document time without a second round-trip.
    ///
    /// "Open" = any status except fully paid / cancelled; we approximate via
    /// `paid_amount < total_amount AND status <> 'cancelled'` which is robust to
    /// the relaxed status CHECK (draft/issued/sent/partially_paid/overdue).
    pub async fn contact_receivables_rls<'e, E>(
        &self,
        executor: E,
        contact_id: Uuid,
    ) -> Result<ContactReceivables, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // One row: COALESCE'd outstanding + the contact's credit_limit.
        let row: (Decimal, Option<Decimal>) = sqlx::query_as(
            "SELECT \
                COALESCE(( \
                    SELECT SUM(i.total_amount - i.paid_amount) \
                    FROM invoice i \
                    WHERE i.contact_id = c.id \
                      AND i.status <> 'cancelled' \
                      AND i.paid_amount < i.total_amount \
                 ), 0)::numeric AS outstanding, \
                c.credit_limit \
             FROM contact c WHERE c.id = $1",
        )
        .bind(contact_id)
        .fetch_one(executor)
        .await?;
        Ok(ContactReceivables {
            outstanding: row.0,
            credit_limit: row.1,
        })
    }

    /// Look up a contact id by a business identifier (ICO or DIC) for import
    /// dedupe (UC-ACC-03.10). Returns the first live match within the tenant.
    pub async fn find_contact_by_identifier_rls<'e, E>(
        &self,
        executor: E,
        ico: Option<&str>,
        dic: Option<&str>,
    ) -> Result<Option<Uuid>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let row: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM contact \
             WHERE merged_into_id IS NULL \
               AND ( ($1::text IS NOT NULL AND ico = $1) \
                  OR ($2::text IS NOT NULL AND dic = $2) ) \
             ORDER BY created_at ASC LIMIT 1",
        )
        .bind(ico)
        .bind(dic)
        .fetch_optional(executor)
        .await?;
        Ok(row.map(|(id,)| id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The repository's value lies in its SQL against a live RLS-enabled
    // Postgres, which the write-only Phase 1 cannot exercise. These unit tests
    // pin the pure, query-independent helpers (column projection + receivables
    // value object) so a refactor that drops a column is caught without a DB.

    #[test]
    fn contact_ext_columns_match_struct_field_count() {
        // AccContactExt has 23 fields (11 base 00184 + 12 ext 00195); the
        // shared projection must list exactly that many comma-separated names so
        // `query_as::<AccContactExt>` maps positionally without drift.
        let count = CONTACT_EXT_COLUMNS.split(',').count();
        assert_eq!(
            count, 23,
            "CONTACT_EXT_COLUMNS must list all AccContactExt fields"
        );
    }

    #[test]
    fn contact_ext_columns_include_merge_and_default_cols() {
        // Guard the 00195 extension columns the merge/defaults features rely on.
        for col in [
            "merged_into_id",
            "is_active",
            "default_currency",
            "credit_limit",
            "vat_verified_at",
        ] {
            assert!(
                CONTACT_EXT_COLUMNS.contains(col),
                "projection missing {col}"
            );
        }
    }

    #[test]
    fn receivables_value_object_roundtrips() {
        let r = ContactReceivables {
            outstanding: Decimal::new(12345, 2), // 123.45
            credit_limit: Some(Decimal::new(100000, 2)),
        };
        assert_eq!(r.outstanding, Decimal::new(12345, 2));
        assert_eq!(r.credit_limit, Some(Decimal::new(100000, 2)));
    }
}
