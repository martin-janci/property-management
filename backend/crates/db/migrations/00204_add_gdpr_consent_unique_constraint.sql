-- Add missing unique constraint on slovak_gdpr_consents so that
-- ON CONFLICT (user_id, organization_id, category) in the repository works.
ALTER TABLE slovak_gdpr_consents
    ADD CONSTRAINT uq_gdpr_consent_user_org_category
    UNIQUE (user_id, organization_id, category);
