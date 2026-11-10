-- Stable slugs for the built-in workflow automation templates.
--
-- The four system templates seeded by 00064 only carry English `name` and
-- `description` text, which ppt-web rendered verbatim in every locale
-- (round-3 gap audit #32). A stable, locale-independent `slug` lets the
-- client look up translated copy (`automation.templates.<slug>.name` /
-- `.description`) and fall back to `name` for custom templates (slug NULL).

ALTER TABLE workflow_automation_templates
    ADD COLUMN IF NOT EXISTS slug VARCHAR(100);

UPDATE workflow_automation_templates SET slug = 'weekly_maintenance_report'
 WHERE slug IS NULL AND is_system AND name = 'Weekly Maintenance Report';
UPDATE workflow_automation_templates SET slug = 'overdue_payment_reminder'
 WHERE slug IS NULL AND is_system AND name = 'Overdue Payment Reminder';
UPDATE workflow_automation_templates SET slug = 'high_priority_fault_escalation'
 WHERE slug IS NULL AND is_system AND name = 'High Priority Fault Escalation';
UPDATE workflow_automation_templates SET slug = 'monthly_statement_generation'
 WHERE slug IS NULL AND is_system AND name = 'Monthly Statement Generation';

CREATE UNIQUE INDEX IF NOT EXISTS uq_workflow_automation_templates_slug
    ON workflow_automation_templates (slug)
    WHERE slug IS NOT NULL;
