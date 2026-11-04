-- Durable dead-letter store for Stripe paid-webhook deliveries that cannot be
-- settled server-side (#3284, follow-up to PR #3273).
--
-- Background
-- ----------
-- PR #3273 stopped the silent-drop bug: a verified `checkout.session.completed`
-- (`payment_status = "paid"`) whose pending session points at an invoice that
-- no longer exists now returns 5xx instead of acking, so Stripe's at-least-once
-- retry keeps redelivering the collected payment. That fix leaned on Stripe's
-- retry queue as the *only* safety net. Stripe eventually gives up retrying
-- (its backoff window is finite), after which a collected payment we never
-- reconciled would be lost with no in-house record.
--
-- This table is that in-house record: on the missing-invoice branch (and the
-- invoice-load DB-error branch) in
-- `api-server/src/routes/integrations/webhook/payment.rs`, the handler persists
-- the raw signed event + provider session id + the (now-dangling) invoice id
-- here BEFORE still returning 5xx. Operators reconcile from this table and set
-- `resolved_at` / `resolution` once handled.
--
-- RLS
-- ---
-- `ENABLE ROW LEVEL SECURITY` with org-scoped read/manage policies mirroring
-- `online_payment_sessions` (00040). Like every financial table, RLS here is
-- ENABLE (not FORCE): the webhook receiver writes through the service/owner
-- role on a plain pool with no `app.current_user_id` GUC set, and the owner
-- bypasses non-FORCE RLS, so the dead-letter INSERT succeeds on the
-- unauthenticated webhook path. Authenticated operator reads go through
-- `organization_members`, so a row is only visible to members of its owning
-- organization (or a super admin).

CREATE TABLE IF NOT EXISTS payment_webhook_dead_letters (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    provider VARCHAR(50) NOT NULL DEFAULT 'stripe',
    -- The gateway session id from the event (Stripe Checkout Session id).
    provider_session_id VARCHAR(255) NOT NULL,
    -- The invoice the session pointed at. NO foreign key on purpose: the whole
    -- reason a row lands here is that this invoice may no longer exist.
    invoice_id UUID,
    -- Gateway event type, e.g. 'checkout.session.completed'.
    event_type VARCHAR(100),
    -- Why the event could not be settled: 'invoice_missing' | 'invoice_load_failed'.
    failure_reason VARCHAR(50) NOT NULL,
    -- The raw, signature-verified request body exactly as received, so the
    -- event can be replayed / audited after reconciliation.
    raw_event BYTEA NOT NULL,
    -- The Stripe-Signature header that was verified against the raw body.
    signature TEXT,
    -- How many times this (provider, session) dead-letter has been seen —
    -- bumped on each Stripe redelivery via the ON CONFLICT upsert.
    attempts INTEGER NOT NULL DEFAULT 1,
    -- Operator reconciliation outcome.
    resolved_at TIMESTAMPTZ,
    resolution TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Idempotency / retry-collapse: Stripe redelivers the same event many times
-- while operators reconcile. A redelivery must bump the existing dead-letter
-- row rather than flood the table with duplicates. Matched by the
-- `ON CONFLICT (provider, provider_session_id)` upsert in
-- `record_payment_webhook_dead_letter`.
CREATE UNIQUE INDEX idx_payment_webhook_dead_letters_dedup
    ON payment_webhook_dead_letters (provider, provider_session_id);

-- Operator dashboard: list unresolved dead letters per org, newest first.
CREATE INDEX idx_payment_webhook_dead_letters_unresolved
    ON payment_webhook_dead_letters (organization_id, created_at)
    WHERE resolved_at IS NULL;

CREATE INDEX idx_payment_webhook_dead_letters_invoice
    ON payment_webhook_dead_letters (invoice_id);

-- Keep `updated_at` current (same trigger fn used across the financial schema).
CREATE TRIGGER payment_webhook_dead_letters_updated_at
    BEFORE UPDATE ON payment_webhook_dead_letters
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

ALTER TABLE payment_webhook_dead_letters ENABLE ROW LEVEL SECURITY;

-- Org members can view their org's dead letters; super admins see all.
CREATE POLICY payment_webhook_dead_letters_select ON payment_webhook_dead_letters
    FOR SELECT
    USING (
        EXISTS (
            SELECT 1 FROM organization_members om
            WHERE om.organization_id = payment_webhook_dead_letters.organization_id
            AND om.user_id = NULLIF(current_setting('app.current_user_id', TRUE), '')::UUID
        )
        OR is_super_admin()
    );

-- Admins/managers of the org (and super admins) can manage (resolve) them.
CREATE POLICY payment_webhook_dead_letters_manage ON payment_webhook_dead_letters
    FOR ALL
    USING (
        EXISTS (
            SELECT 1 FROM organization_members om
            WHERE om.organization_id = payment_webhook_dead_letters.organization_id
            AND om.user_id = NULLIF(current_setting('app.current_user_id', TRUE), '')::UUID
        )
        OR is_super_admin()
    )
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM organization_members om
            WHERE om.organization_id = payment_webhook_dead_letters.organization_id
            AND om.user_id = NULLIF(current_setting('app.current_user_id', TRUE), '')::UUID
        )
        OR is_super_admin()
    );
