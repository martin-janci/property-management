//! Realtor profiles (Story 33.1).

use super::RealityPortalRepository;
use crate::models::reality_portal::*;
use sqlx::Error as SqlxError;
use uuid::Uuid;

impl RealityPortalRepository {
    // ========================================================================
    // Realtor Profiles (Story 33.1)
    // ========================================================================

    /// Create or update realtor profile.
    pub async fn upsert_realtor_profile(
        &self,
        user_id: Uuid,
        data: CreateRealtorProfile,
    ) -> Result<RealtorProfile, SqlxError> {
        sqlx::query_as::<_, RealtorProfile>(
            r#"
            INSERT INTO realtor_profiles (
                user_id, bio, tagline, specializations, experience_years,
                languages, license_number, phone, whatsapp, email_public
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (user_id) DO UPDATE SET
                bio = COALESCE($2, realtor_profiles.bio),
                tagline = COALESCE($3, realtor_profiles.tagline),
                specializations = COALESCE($4, realtor_profiles.specializations),
                experience_years = COALESCE($5, realtor_profiles.experience_years),
                languages = COALESCE($6, realtor_profiles.languages),
                license_number = COALESCE($7, realtor_profiles.license_number),
                phone = COALESCE($8, realtor_profiles.phone),
                whatsapp = COALESCE($9, realtor_profiles.whatsapp),
                email_public = COALESCE($10, realtor_profiles.email_public),
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(&data.bio)
        .bind(&data.tagline)
        .bind(&data.specializations)
        .bind(data.experience_years)
        .bind(&data.languages)
        .bind(&data.license_number)
        .bind(&data.phone)
        .bind(&data.whatsapp)
        .bind(&data.email_public)
        .fetch_one(&self.pool)
        .await
    }

    /// Get realtor profile.
    pub async fn get_realtor_profile(
        &self,
        user_id: Uuid,
    ) -> Result<Option<RealtorProfile>, SqlxError> {
        sqlx::query_as::<_, RealtorProfile>("SELECT * FROM realtor_profiles WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
    }

    /// Update realtor profile.
    #[allow(clippy::too_many_arguments)]
    pub async fn update_realtor_profile(
        &self,
        user_id: Uuid,
        data: UpdateRealtorProfile,
    ) -> Result<RealtorProfile, SqlxError> {
        sqlx::query_as::<_, RealtorProfile>(
            r#"
            UPDATE realtor_profiles SET
                photo_url = COALESCE($2, photo_url),
                bio = COALESCE($3, bio),
                tagline = COALESCE($4, tagline),
                specializations = COALESCE($5, specializations),
                experience_years = COALESCE($6, experience_years),
                languages = COALESCE($7, languages),
                license_number = COALESCE($8, license_number),
                phone = COALESCE($9, phone),
                whatsapp = COALESCE($10, whatsapp),
                email_public = COALESCE($11, email_public),
                linkedin_url = COALESCE($12, linkedin_url),
                facebook_url = COALESCE($13, facebook_url),
                instagram_url = COALESCE($14, instagram_url),
                show_phone = COALESCE($15, show_phone),
                show_email = COALESCE($16, show_email),
                accept_inquiries = COALESCE($17, accept_inquiries),
                updated_at = NOW()
            WHERE user_id = $1
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(&data.photo_url)
        .bind(&data.bio)
        .bind(&data.tagline)
        .bind(&data.specializations)
        .bind(data.experience_years)
        .bind(&data.languages)
        .bind(&data.license_number)
        .bind(&data.phone)
        .bind(&data.whatsapp)
        .bind(&data.email_public)
        .bind(&data.linkedin_url)
        .bind(&data.facebook_url)
        .bind(&data.instagram_url)
        .bind(data.show_phone)
        .bind(data.show_email)
        .bind(data.accept_inquiries)
        .fetch_one(&self.pool)
        .await
    }
}
