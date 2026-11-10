-- Energy & Sustainability tables (Epic 65) that `EnergyRepository`
-- (crates/db/src/repositories/energy.rs) and the `/api/v1/energy` routes
-- query, but that no migration ever created. Every energy endpoint therefore
-- failed at runtime with `relation "..." does not exist`.
--
-- Columns, types and unique keys mirror the repository SQL and the
-- `models::energy` structs exactly:
--   * `Option<_>` fields are nullable, everything else is NOT NULL.
--   * `ON CONFLICT` targets in the repository get a matching UNIQUE constraint
--     (carbon_emissions, carbon_targets, sustainability_scores).
--   * The Postgres enum labels match the `sqlx(rename_all = ...)` mapping.
--
-- RLS: ENABLE (not FORCE), with org-scoped policies like the neighbouring
-- analytics tables. The repository already filters every query by
-- `organization_id`; the api-server pool connects as the owner role, which
-- bypasses non-FORCE RLS, so the policies protect non-owner roles only.

DO $$ BEGIN
    CREATE TYPE energy_rating AS ENUM ('A', 'B', 'C', 'D', 'E', 'F', 'G');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

DO $$ BEGIN
    CREATE TYPE emission_source_type AS ENUM (
        'electricity', 'gas', 'heating', 'cooling', 'water', 'waste', 'transport', 'other'
    );
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

DO $$ BEGIN
    CREATE TYPE heating_type AS ENUM (
        'gas', 'electric', 'heat_pump', 'district_heating', 'oil', 'biomass', 'solar',
        'geothermal', 'none'
    );
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

DO $$ BEGIN
    CREATE TYPE insulation_rating AS ENUM ('excellent', 'good', 'average', 'poor', 'none');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

DO $$ BEGIN
    CREATE TYPE benchmark_metric_type AS ENUM (
        'electricity_per_sqm', 'gas_per_sqm', 'water_per_person', 'carbon_per_sqm',
        'total_energy', 'renewable_percentage'
    );
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

DO $$ BEGIN
    CREATE TYPE benchmark_alert_severity AS ENUM ('info', 'warning', 'critical');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

-- ---------------------------------------------------------------------------
-- Energy performance certificates (Story 65.1)
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS energy_performance_certificates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    unit_id UUID NOT NULL REFERENCES units(id) ON DELETE CASCADE,
    rating energy_rating NOT NULL,
    certificate_number VARCHAR(100),
    valid_until DATE,
    annual_energy_kwh DECIMAL(15, 2),
    annual_co2_kg DECIMAL(15, 2),
    primary_energy_kwh_per_sqm DECIMAL(10, 2),
    issued_by VARCHAR(255),
    issued_at DATE,
    document_url TEXT,
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID REFERENCES users(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_epc_org ON energy_performance_certificates (organization_id);
CREATE INDEX IF NOT EXISTS idx_epc_unit ON energy_performance_certificates (unit_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_epc_valid_until ON energy_performance_certificates (valid_until);

DROP TRIGGER IF EXISTS energy_performance_certificates_updated_at ON energy_performance_certificates;
CREATE TRIGGER energy_performance_certificates_updated_at
    BEFORE UPDATE ON energy_performance_certificates
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ---------------------------------------------------------------------------
-- Carbon emissions and targets (Story 65.2)
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS carbon_emissions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    building_id UUID NOT NULL REFERENCES buildings(id) ON DELETE CASCADE,
    year INTEGER NOT NULL,
    month INTEGER NOT NULL CHECK (month BETWEEN 1 AND 12),
    source_type emission_source_type NOT NULL,
    co2_kg DECIMAL(15, 2) NOT NULL,
    energy_kwh DECIMAL(15, 2),
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    -- `record_emission` upserts ON CONFLICT (building_id, year, month, source_type).
    CONSTRAINT uq_carbon_emissions_period UNIQUE (building_id, year, month, source_type)
);

CREATE INDEX IF NOT EXISTS idx_carbon_emissions_org ON carbon_emissions (organization_id);

CREATE TABLE IF NOT EXISTS carbon_targets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    building_id UUID NOT NULL REFERENCES buildings(id) ON DELETE CASCADE,
    year INTEGER NOT NULL,
    target_co2_kg DECIMAL(15, 2) NOT NULL,
    baseline_co2_kg DECIMAL(15, 2),
    baseline_year INTEGER,
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- `set_carbon_target` upserts ON CONFLICT (building_id, year).
    CONSTRAINT uq_carbon_targets_year UNIQUE (building_id, year)
);

CREATE INDEX IF NOT EXISTS idx_carbon_targets_org ON carbon_targets (organization_id);

-- ---------------------------------------------------------------------------
-- Sustainability scores for listings (Story 65.3)
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS sustainability_scores (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    listing_id UUID NOT NULL REFERENCES listings(id) ON DELETE CASCADE,
    score INTEGER NOT NULL CHECK (score BETWEEN 1 AND 100),
    has_solar BOOLEAN NOT NULL DEFAULT FALSE,
    has_heat_pump BOOLEAN NOT NULL DEFAULT FALSE,
    insulation_rating insulation_rating NOT NULL,
    heating_type heating_type NOT NULL,
    has_ev_charging BOOLEAN,
    has_rainwater_harvesting BOOLEAN,
    has_smart_thermostat BOOLEAN,
    has_led_lighting BOOLEAN,
    has_double_glazing BOOLEAN,
    renewable_energy_percentage INTEGER CHECK (
        renewable_energy_percentage IS NULL OR renewable_energy_percentage BETWEEN 0 AND 100
    ),
    annual_energy_cost_estimate DECIMAL(15, 2),
    energy_rating energy_rating,
    calculated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    notes TEXT,
    -- `upsert_sustainability_score` upserts ON CONFLICT (listing_id).
    CONSTRAINT uq_sustainability_scores_listing UNIQUE (listing_id)
);

CREATE INDEX IF NOT EXISTS idx_sustainability_scores_org ON sustainability_scores (organization_id);
CREATE INDEX IF NOT EXISTS idx_sustainability_scores_score ON sustainability_scores (score DESC);

-- ---------------------------------------------------------------------------
-- Building benchmarks and alerts (Story 65.4)
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS building_benchmarks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    building_id UUID NOT NULL REFERENCES buildings(id) ON DELETE CASCADE,
    metric_type benchmark_metric_type NOT NULL,
    value DECIMAL(15, 4) NOT NULL,
    percentile INTEGER NOT NULL CHECK (percentile BETWEEN 0 AND 100),
    comparable_buildings_count INTEGER NOT NULL DEFAULT 0,
    period_start DATE NOT NULL,
    period_end DATE NOT NULL,
    region VARCHAR(100),
    building_type VARCHAR(100),
    calculated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_building_benchmarks_org ON building_benchmarks (organization_id);
CREATE INDEX IF NOT EXISTS idx_building_benchmarks_building
    ON building_benchmarks (building_id, calculated_at DESC);

CREATE TABLE IF NOT EXISTS benchmark_alerts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    building_id UUID NOT NULL REFERENCES buildings(id) ON DELETE CASCADE,
    metric_type benchmark_metric_type NOT NULL,
    current_value DECIMAL(15, 4) NOT NULL,
    benchmark_value DECIMAL(15, 4) NOT NULL,
    deviation_percentage DECIMAL(10, 2) NOT NULL,
    severity benchmark_alert_severity NOT NULL DEFAULT 'info',
    message TEXT,
    is_resolved BOOLEAN NOT NULL DEFAULT FALSE,
    resolved_at TIMESTAMPTZ,
    resolved_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_benchmark_alerts_org ON benchmark_alerts (organization_id);
CREATE INDEX IF NOT EXISTS idx_benchmark_alerts_building
    ON benchmark_alerts (building_id, created_at DESC);

-- ---------------------------------------------------------------------------
-- Row-level security
-- ---------------------------------------------------------------------------
DO $$
DECLARE
    t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY[
        'energy_performance_certificates',
        'carbon_emissions',
        'carbon_targets',
        'sustainability_scores',
        'building_benchmarks',
        'benchmark_alerts'
    ] LOOP
        EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY', t);
        EXECUTE format('DROP POLICY IF EXISTS %I ON %I', t || '_tenant_isolation', t);
        EXECUTE format(
            'CREATE POLICY %I ON %I FOR ALL '
            'USING ((organization_id = get_current_org_id() AND get_current_org_not_deleted()) OR is_super_admin()) '
            'WITH CHECK ((organization_id = get_current_org_id() AND get_current_org_not_deleted()) OR is_super_admin())',
            t || '_tenant_isolation', t
        );
    END LOOP;
END $$;
