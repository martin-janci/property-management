-- Migration: 00210_fix_czech_three_quarters_statute
-- Align the Czech three-quarters-majority legal reference between the seed and
-- the code (#1906 finding-2).
--
-- Migration 00197 seeded czechia/three_quarters_majority with
-- 'SS 1210 zakona 89/2012 Sb.', but
-- CzechDecisionType::ThreeQuartersMajority::legal_reference() returns
-- 'SS 1209 zakona 89/2012 Sb.'. For a legal-compliance feature the emitted
-- statute citation must be authoritative and consistent across the two code
-- paths (the seeded jurisdiction_rules row vs. the in-code fallback). Forward-fix
-- the seed to match the code value (a §1209/§1210 swap), rather than editing the
-- already-applied 00197 in place (which would change its checksum).
--
-- NOTE: this aligns the DB to the code's value (§1209). The exact § of zákon
-- 89/2012 Sb. for the three-quarters threshold should still be confirmed by
-- legal review; this migration only removes the code/DB divergence.
UPDATE jurisdiction_rules
SET legal_reference = 'SS 1209 zakona 89/2012 Sb.'
WHERE jurisdiction = 'czechia'
  AND decision_type = 'three_quarters_majority'
  AND legal_reference = 'SS 1210 zakona 89/2012 Sb.';
