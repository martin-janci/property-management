-- Story 3.1 AC3 (BIT-200): Building geocoding / coordinates
-- Adds latitude/longitude so buildings can be geocoded on write and shown on a map.
-- Mirrors the precision used by listings/emergency (DECIMAL(10,8) / DECIMAL(11,8)).

ALTER TABLE buildings
    ADD COLUMN IF NOT EXISTS latitude DECIMAL(10, 8),
    ADD COLUMN IF NOT EXISTS longitude DECIMAL(11, 8);

COMMENT ON COLUMN buildings.latitude IS 'Geocoded latitude (WGS84); NULL when geocoding is unconfigured or unresolved';
COMMENT ON COLUMN buildings.longitude IS 'Geocoded longitude (WGS84); NULL when geocoding is unconfigured or unresolved';

-- Supports map/proximity queries that filter on a populated coordinate pair.
CREATE INDEX IF NOT EXISTS idx_buildings_coordinates ON buildings (latitude, longitude)
    WHERE latitude IS NOT NULL AND longitude IS NOT NULL;
