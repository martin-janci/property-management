-- Fix rental_bookings external-id uniqueness so manual bookings are not capped
-- at one per platform across the whole system.
--
-- Migration 00051 declared
--   CONSTRAINT unique_external_booking UNIQUE NULLS NOT DISTINCT (platform, external_booking_id)
-- NULLS NOT DISTINCT treats every NULL external_booking_id as equal, so only ONE
-- booking per platform without an external id could exist across ALL
-- organisations: a second manual/'direct' booking anywhere failed with a unique
-- violation (23505). It also made external ids globally unique per platform,
-- although rental_bookings is tenant-scoped (organization_id + FORCE RLS) and
-- two organisations may legitimately import the same platform reservation code.
--
-- Intended rule: an external booking id is unique per organisation and
-- platform; bookings without an external id are unconstrained. No code path
-- uses this constraint as an ON CONFLICT arbiter (bookings are created with a
-- plain INSERT in BookingRepository::create_booking), so no statement needs a
-- matching WHERE predicate.
--
-- Safe on existing data: the old constraint was strictly stronger on the rows
-- the new index covers (every non-NULL (platform, external_booking_id) pair was
-- already globally unique), so no existing row can violate the new index.
-- RLS policies on rental_bookings are untouched.

ALTER TABLE rental_bookings DROP CONSTRAINT IF EXISTS unique_external_booking;

CREATE UNIQUE INDEX IF NOT EXISTS uq_rental_bookings_org_platform_external_id
    ON rental_bookings (organization_id, platform, external_booking_id)
    WHERE external_booking_id IS NOT NULL;
