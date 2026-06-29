-- Persistent payment-reminder dedup (#1790).
--
-- The payment-reminder scheduler runs on a ~60s tick and re-selected every
-- 'sent' invoice in the due-date window on every tick, notifying all residents
-- each time. The only guard was an in-memory 5-minute dedup map that is cleared
-- on restart, so residents were spammed for the whole reminder window.
--
-- Add a persistent per-invoice stamp (mirroring signature reminders'
-- last_reminder_at) so the scheduler can skip invoices already reminded within
-- the dedup window.
ALTER TABLE invoices
    ADD COLUMN IF NOT EXISTS last_payment_reminder_at TIMESTAMPTZ;
