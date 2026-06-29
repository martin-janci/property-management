-- Migration: 00194_audit_action_rental_guest_id_document.sql
-- GH #1760 / #1783: the guest ID-document upload + OCR-extract seam (PR #1750)
-- handles a guest's identity document (high-sensitivity, GDPR-relevant PII) but
-- wrote no `audit_log` record of who uploaded or read it and when.
--
-- Add two dedicated denial-agnostic action variants so the upload and the OCR
-- access are action-filterable in the audit log, following the
-- `mfa_recovery_rate_limited` precedent (00185). `audit_action` is a Postgres
-- ENUM, so the new values are added via ALTER TYPE (idempotent).
ALTER TYPE audit_action ADD VALUE IF NOT EXISTS 'rental_guest_id_document_upload';
ALTER TYPE audit_action ADD VALUE IF NOT EXISTS 'rental_guest_id_document_extract';
