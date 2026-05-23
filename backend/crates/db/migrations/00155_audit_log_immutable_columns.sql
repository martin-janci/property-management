-- Migration: 00155_audit_log_immutable_columns.sql
-- Dev-team review P1-04: enforce immutability of audit_logs hash chain.
--
-- The old `AuditLogRepository::create` path was INSERT then UPDATE — first
-- inserting the row, then writing the integrity_hash. Any code path that
-- could run an UPDATE against this table could rewrite the hash and
-- forge the chain, undermining the immutability promise the schema
-- comments make.
--
-- The repository is now single-INSERT (hash + previous_hash both
-- supplied at insert time). This trigger backs that contract at the
-- database level: any UPDATE that attempts to change integrity_hash or
-- previous_hash is refused, regardless of which role issues it.
--
-- Other columns remain mutable so legitimate maintenance (e.g.
-- masking PII out of `details` for GDPR erasure) still works.

CREATE OR REPLACE FUNCTION audit_logs_block_hash_mutation()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.integrity_hash IS DISTINCT FROM OLD.integrity_hash THEN
        RAISE EXCEPTION 'audit_logs.integrity_hash is immutable (P1-04)';
    END IF;
    IF NEW.previous_hash IS DISTINCT FROM OLD.previous_hash THEN
        RAISE EXCEPTION 'audit_logs.previous_hash is immutable (P1-04)';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS audit_logs_immutable_hash ON audit_logs;
CREATE TRIGGER audit_logs_immutable_hash
    BEFORE UPDATE ON audit_logs
    FOR EACH ROW
    EXECUTE FUNCTION audit_logs_block_hash_mutation();
