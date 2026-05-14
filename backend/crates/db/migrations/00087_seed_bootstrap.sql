-- Bootstrap seed function for fresh installs.
--
-- This migration creates a helper function for minimal bootstrap seeding.
-- The function does NOT run automatically during migrations.
--
-- Usage:
--   SELECT * FROM seed_bootstrap('admin@example.com', '<argon2-hash>');
--
-- For full seeding with sample data, use the CLI:
--   cargo run -p api-server --bin ppt-seed

-- Create the bootstrap seed function
-- SECURITY DEFINER: Runs with owner privileges to bypass RLS.
-- Input validation is critical to prevent misuse of elevated privileges.
CREATE OR REPLACE FUNCTION seed_bootstrap(
    p_admin_email VARCHAR,
    p_admin_password_hash VARCHAR
) RETURNS TABLE(admin_id UUID, org_id UUID) AS $$
DECLARE
    v_admin_id UUID;
    v_org_id UUID;
    v_role_id UUID;
BEGIN
    -- Basic input validation to reduce risk of misuse under SECURITY DEFINER
    IF p_admin_email IS NULL OR length(trim(p_admin_email)) = 0 THEN
        RAISE EXCEPTION 'seed_bootstrap: admin email must be provided';
    END IF;

    IF position('@' IN p_admin_email) = 0 THEN
        RAISE EXCEPTION 'seed_bootstrap: admin email must contain ''@''';
    END IF;

    IF p_admin_password_hash IS NULL OR length(p_admin_password_hash) = 0 THEN
        RAISE EXCEPTION 'seed_bootstrap: admin password hash must be provided';
    END IF;

    -- Password hash is expected to be Argon2id; enforce a basic shape check
    IF p_admin_password_hash NOT LIKE '$argon2%' THEN
        RAISE EXCEPTION 'seed_bootstrap: admin password hash must be an Argon2 hash starting with "$argon2"';
    END IF;

    -- Run privileged seed operations in a protected block so that the
    -- elevated request context is always cleared, even on error.
    BEGIN
        -- Temporarily set super admin context to bypass RLS
        PERFORM set_request_context(NULL, NULL, TRUE);

        -- Check if already seeded (by checking for demo-property.test domain)
        IF EXISTS (
            SELECT 1
            FROM organizations
            WHERE contact_email LIKE '%@demo-property.test'
        ) THEN
            RAISE EXCEPTION 'Seed data already exists. Use the CLI with --force to re-seed.';
        END IF;

        -- Create demo organization
        INSERT INTO organizations (name, slug, contact_email, status, settings)
        VALUES (
            'Demo Property Management',
            'demo-property',
            'contact@demo-property.test',
            'active',
            '{}'::jsonb
        )
        RETURNING id INTO v_org_id;

        -- Create admin user (verified and active)
        INSERT INTO users (
            email,
            password_hash,
            name,
            status,
            email_verified_at,
            locale
        )
        VALUES (
            p_admin_email,
            p_admin_password_hash,
            'System Administrator',
            'active',
            NOW(),
            'en'
        )
        RETURNING id INTO v_admin_id;

        -- Get Super Admin role (created automatically by organization trigger)
        SELECT id INTO v_role_id
        FROM roles
        WHERE organization_id = v_org_id AND name = 'Super Admin';

        -- Add admin to organization with Super Admin role
        INSERT INTO organization_members (
            organization_id,
            user_id,
            role_id,
            role_type,
            status,
            joined_at
        )
        VALUES (
            v_org_id,
            v_admin_id,
            v_role_id,
            'Super Admin',
            'active',
            NOW()
        );

        -- Clear RLS context on success
        PERFORM clear_request_context();
    EXCEPTION
        WHEN OTHERS THEN
            -- Ensure RLS context is cleared even if an error occurs
            PERFORM clear_request_context();
            RAISE;
    END;

    RETURN QUERY SELECT v_admin_id, v_org_id;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- Add helpful comment
COMMENT ON FUNCTION seed_bootstrap IS
'Bootstrap seed function for fresh database installs.
Creates a minimal setup with one organization and one super admin user.
Does NOT run automatically during migrations.

Usage:
  SELECT * FROM seed_bootstrap(''admin@example.com'', ''<argon2-hash>'');

For full seeding with sample data (buildings, units, multiple users), use the CLI:
  cargo run -p api-server --bin ppt-seed

Password hash must be generated using Argon2id. Example using the CLI:
  cargo run -p api-server --bin ppt-seed -- --admin-email admin@example.com
';
