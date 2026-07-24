-- Add missing unique constraint on slovak_gdpr_consents so that
-- ON CONFLICT (user_id, organization_id, category) in the repository works.
--
-- NOTE (BIT-575): originally authored as 00204, which collided with
-- 00204_acc_share_link_token_hash.sql. sqlx records one migration per version,
-- so the duplicate-versioned constraint was silently skipped and the GDPR
-- consent upsert 500'd ("no unique or exclusion constraint matching the ON
-- CONFLICT specification"). Renumbered to 00222 so it actually applies.
-- Guarded so re-application on any env that already has the constraint is a
-- no-op.
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'uq_gdpr_consent_user_org_category'
    ) THEN
        ALTER TABLE slovak_gdpr_consents
            ADD CONSTRAINT uq_gdpr_consent_user_org_category
            UNIQUE (user_id, organization_id, category);
    END IF;
END $$;
