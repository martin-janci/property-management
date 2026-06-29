//! Form-field CRUD & reordering.

use super::FormRepository;
use crate::models::{CreateFormField, FormField, UpdateFormField};
use sqlx::{Executor, Postgres};
use uuid::Uuid;

impl FormRepository {
    /// Creates a new field for a form.
    pub async fn create_field<'e, E>(
        &self,
        executor: E,
        form_id: Uuid,
        data: CreateFormField,
        order: i32,
    ) -> Result<FormField, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let validation_rules = data
            .validation_rules
            .map(|r| serde_json::to_value(r).unwrap_or_default())
            .unwrap_or_else(|| serde_json::json!({}));

        let options = data
            .options
            .map(|o| serde_json::to_value(o).unwrap_or_default())
            .unwrap_or_else(|| serde_json::json!([]));

        let conditional_display = data
            .conditional_display
            .map(|c| serde_json::to_value(c).unwrap_or_default());

        sqlx::query_as::<_, FormField>(
            r#"
            INSERT INTO form_fields (
                form_id, field_key, label, field_type, required,
                help_text, placeholder, default_value, validation_rules,
                options, field_order, width, section, conditional_display
            )
            VALUES ($1, $2, $3, $4::form_field_type, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            RETURNING id, form_id, field_key, label, field_type::text AS field_type, required,
                help_text, placeholder, default_value, validation_rules, options, field_order,
                width, section, conditional_display, created_at, updated_at
            "#,
        )
        .bind(form_id)
        .bind(&data.field_key)
        .bind(&data.label)
        .bind(&data.field_type)
        .bind(data.required)
        .bind(&data.help_text)
        .bind(&data.placeholder)
        .bind(&data.default_value)
        .bind(&validation_rules)
        .bind(&options)
        .bind(if data.field_order > 0 {
            data.field_order
        } else {
            order
        })
        .bind(&data.width)
        .bind(&data.section)
        .bind(&conditional_display)
        .fetch_one(executor)
        .await
    }

    /// Gets all fields for a form.
    pub async fn get_fields<'e, E>(
        &self,
        executor: E,
        form_id: Uuid,
    ) -> Result<Vec<FormField>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, FormField>(
            r#"
            SELECT id, form_id, field_key, label, field_type::text AS field_type, required,
                help_text, placeholder, default_value, validation_rules, options, field_order,
                width, section, conditional_display, created_at, updated_at
            FROM form_fields
            WHERE form_id = $1
            ORDER BY field_order ASC
            "#,
        )
        .bind(form_id)
        .fetch_all(executor)
        .await
    }

    /// Updates a form field.
    pub async fn update_field<'e, E>(
        &self,
        executor: E,
        form_id: Uuid,
        field_id: Uuid,
        data: UpdateFormField,
    ) -> Result<FormField, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let validation_rules = data
            .validation_rules
            .map(|r| serde_json::to_value(r).unwrap_or_default());

        let options = data
            .options
            .map(|o| serde_json::to_value(o).unwrap_or_default());

        let conditional_display = data
            .conditional_display
            .map(|c| serde_json::to_value(c).unwrap_or_default());

        sqlx::query_as::<_, FormField>(
            r#"
            UPDATE form_fields SET
                label = COALESCE($1, label),
                field_type = COALESCE($2::form_field_type, field_type),
                required = COALESCE($3, required),
                help_text = COALESCE($4, help_text),
                placeholder = COALESCE($5, placeholder),
                default_value = COALESCE($6, default_value),
                validation_rules = COALESCE($7, validation_rules),
                options = COALESCE($8, options),
                field_order = COALESCE($9, field_order),
                width = COALESCE($10, width),
                section = COALESCE($11, section),
                conditional_display = COALESCE($12, conditional_display),
                updated_at = NOW()
            WHERE id = $13 AND form_id = $14
            RETURNING id, form_id, field_key, label, field_type::text AS field_type, required,
                help_text, placeholder, default_value, validation_rules, options, field_order,
                width, section, conditional_display, created_at, updated_at
            "#,
        )
        .bind(&data.label)
        .bind(&data.field_type)
        .bind(data.required)
        .bind(&data.help_text)
        .bind(&data.placeholder)
        .bind(&data.default_value)
        .bind(&validation_rules)
        .bind(&options)
        .bind(data.field_order)
        .bind(&data.width)
        .bind(&data.section)
        .bind(&conditional_display)
        .bind(field_id)
        .bind(form_id)
        .fetch_one(executor)
        .await
    }

    /// Deletes a form field.
    pub async fn delete_field<'e, E>(
        &self,
        executor: E,
        form_id: Uuid,
        field_id: Uuid,
    ) -> Result<(), sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query("DELETE FROM form_fields WHERE id = $1 AND form_id = $2")
            .bind(field_id)
            .bind(form_id)
            .execute(executor)
            .await?;

        Ok(())
    }

    /// Reorders form fields.
    pub async fn reorder_fields<'e, E>(
        &self,
        executor: E,
        form_id: Uuid,
        field_orders: Vec<(Uuid, i32)>,
    ) -> Result<(), sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // If there are no fields to reorder, avoid running a no-op query.
        if field_orders.is_empty() {
            return Ok(());
        }

        // Split the (field_id, order) pairs into parallel vectors for efficient bulk update.
        let (field_ids, orders): (Vec<Uuid>, Vec<i32>) = field_orders.into_iter().unzip();

        // Perform a single bulk UPDATE using array parameters and UNNEST.
        sqlx::query(
            r#"
            UPDATE form_fields AS f
            SET field_order = v.field_order,
                updated_at = NOW()
            FROM (
                SELECT
                    UNNEST($1::uuid[]) AS id,
                    UNNEST($2::int4[]) AS field_order
            ) AS v
            WHERE f.form_id = $3
              AND f.id = v.id
            "#,
        )
        .bind(&field_ids)
        .bind(&orders)
        .bind(form_id)
        .execute(executor)
        .await?;

        Ok(())
    }
}
