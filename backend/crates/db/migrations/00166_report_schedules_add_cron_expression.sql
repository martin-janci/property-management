-- Issue #616: add dedicated cron_expression column to report_schedules.
--
-- Background: migration 00162 created `time TEXT NOT NULL DEFAULT '08:00'`
-- intended for HH:MM schedule times. PR #531 (gap-81-1) repurposed the same
-- column to store 5-field UNIX cron expressions (e.g. `*/5 * * * *`). The
-- two usages are mutually exclusive — anything reading `time` as HH:MM will
-- silently mis-interpret cron data.
--
-- This migration adds a proper `cron_expression TEXT` column and backfills
-- it from any rows where `time` clearly contains a cron expression (i.e.
-- the value contains a space or `*` character — neither is valid in HH:MM).

ALTER TABLE report_schedules
    ADD COLUMN IF NOT EXISTS cron_expression TEXT;

-- Backfill: copy any cron-shaped strings out of `time` into the new column.
UPDATE report_schedules
   SET cron_expression = time
 WHERE cron_expression IS NULL
   AND time ~ '[\* ]';

COMMENT ON COLUMN report_schedules.cron_expression IS
    '5-field UNIX cron expression (gap-81-1). Preferred over the legacy `time` HH:MM column.';

COMMENT ON COLUMN report_schedules.time IS
    'Legacy HH:MM time-of-day. Superseded by cron_expression; retained for back-compat.';
