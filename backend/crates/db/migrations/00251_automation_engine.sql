-- Gap audit 3.1: automation rules get a real runner.
--
-- 1. Global-read SELECT leg for the automation worker.
--    `workflow_automation_rules` carries FORCE RLS (00114) and the api-server
--    connects as the table owner without BYPASSRLS (00179). The background
--    automation worker must find due schedule rules and active condition
--    rules across every org, so it reads under `set_global_read_context(TRUE)`
--    exactly like the report-schedule worker (00218). Writes stay gated by the
--    org-scoped tenant-isolation policy: the worker performs each rule's writes
--    under that rule's tenant context.
DROP POLICY IF EXISTS workflow_automation_rules_global_read ON workflow_automation_rules;

CREATE POLICY workflow_automation_rules_global_read ON workflow_automation_rules
    FOR SELECT
    USING (is_global_read_context());

COMMENT ON POLICY workflow_automation_rules_global_read ON workflow_automation_rules IS
    'Gap audit 3.1: cross-org SELECT for the automation worker '
    '(app.global_read = on). Read-only; writes remain org-scoped.';

-- Due-work lookup for condition rules (schedule rules use idx_automation_rules_next_run).
CREATE INDEX IF NOT EXISTS idx_automation_rules_trigger_active
    ON workflow_automation_rules (trigger_type)
    WHERE is_active = TRUE;

-- Dedupe lookup for condition rules: one run per (rule, entity).
CREATE INDEX IF NOT EXISTS idx_automation_logs_rule_entity
    ON workflow_automation_logs (rule_id, (trigger_data->>'entity_key'));

-- 2. Rewrite the built-in templates to the actions the engine implements.
--    The 00064 seeds referenced actions nothing could execute
--    (`assign_fault` to "on_call", report names that do not exist, per-unit
--    statements). Slugs stay stable so translated copy keeps working.
UPDATE workflow_automation_templates SET
    trigger_type = 'schedule',
    trigger_config_template = '{"cron": "0 9 * * 1", "timezone": "Europe/Bratislava"}',
    actions_template = '[{"type": "generate_report", "name": "Fault report", "config": {"report": "faults", "format": "pdf", "period_days": 7, "to": "managers"}}]'
WHERE slug = 'weekly_maintenance_report';

UPDATE workflow_automation_templates SET
    trigger_type = 'condition',
    trigger_config_template = '{"check": "payment.daysOverdue", "operator": ">=", "value": 7, "conditions": [{"field": "payment.daysOverdue", "operator": "greater_than_or_equals", "value": 7}]}',
    actions_template = '[{"type": "send_notification", "name": "Notify residents", "config": {"title": "Payment overdue", "message": "Invoice {{payment.invoiceNumber}} ({{payment.balanceDue}} {{payment.currency}}) is {{payment.daysOverdue}} days overdue.", "recipients": ["unit_residents"]}}, {"type": "send_email", "name": "E-mail residents", "config": {"to": "unit_residents", "subject": "Payment overdue: invoice {{payment.invoiceNumber}}", "message": "Invoice {{payment.invoiceNumber}} with balance {{payment.balanceDue}} {{payment.currency}} is {{payment.daysOverdue}} days overdue. Please arrange payment."}}]'
WHERE slug = 'overdue_payment_reminder';

UPDATE workflow_automation_templates SET
    trigger_type = 'event',
    trigger_config_template = '{"event_type": "fault_created", "filters": {"priority": "high"}}',
    actions_template = '[{"type": "send_notification", "name": "Notify managers", "config": {"title": "High priority fault", "message": "{{fault.title}} ({{fault.category}}) was reported.", "recipients": ["managers"]}}, {"type": "send_email", "name": "E-mail managers", "config": {"to": "managers", "subject": "High priority fault: {{fault.title}}", "message": "A high priority fault was reported: {{fault.title}}\n\n{{fault.description}}"}}]'
WHERE slug = 'high_priority_fault_escalation';

-- Per-unit statements do not exist; the template becomes the monthly
-- consumption report e-mailed to managers (ppt-web copy updated in step).
UPDATE workflow_automation_templates SET
    name = 'Monthly Consumption Report',
    description = 'E-mail the monthly consumption report to managers on the 1st',
    trigger_type = 'schedule',
    trigger_config_template = '{"cron": "0 8 1 * *", "timezone": "Europe/Bratislava"}',
    actions_template = '[{"type": "generate_report", "name": "Monthly consumption report", "config": {"report": "consumption", "format": "excel", "period_days": 31, "to": "managers"}}]'
WHERE slug = 'monthly_statement_generation';
