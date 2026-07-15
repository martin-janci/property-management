-- Migration: 00217_pgvector_backfill_assumed_marker.sql
-- Follow-up to #2321 (post-merge review of PR #2312 / #2300).
--
-- Problem
-- -------
-- 00215 made `migrate_jsonb_to_vector()` stamp the assumed embedding model
-- (`text-embedding-3-small`) into `metadata.embedding_model` for untagged
-- legacy rows. But 1536-dim covers BOTH `text-embedding-3-small` and the older
-- `text-embedding-ada-002` (00081's own DDL comment sized the column for
-- "OpenAI ada-002") — two incompatible embedding spaces. The stamp is written
-- in a form indistinguishable from genuinely recorded provenance, so if any
-- legacy rows were actually ada-002 the back-fill turns a *detectable unknown*
-- (no provenance) into *confidently wrong provenance*, with no way to tell
-- assumed rows apart after a production run.
--
-- Fix
-- ---
-- Redefine the function (forward-only; 00081/00215 are merged and never edited)
-- so that, alongside the assumed `embedding_model`, untagged rows also get an
-- `embedding_model_assumed: true` marker. Rows that already carry an
-- `embedding_model` are still left untouched. A future re-embedding job (the
-- real fix for legacy rows) can then target `metadata ? 'embedding_model_assumed'`
-- to find exactly the rows whose provenance was inferred rather than recorded.
--
-- Land this BEFORE operators run `/rag/migrate` in production — once the assumed
-- rows are stamped without the marker they are no longer identifiable.

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'vector') THEN
        EXECUTE $func$
            CREATE OR REPLACE FUNCTION migrate_jsonb_to_vector()
            RETURNS void AS $inner$
            BEGIN
                -- Single set-based UPDATE (see 00215). Only untouched legacy
                -- rows are converted: a 1536-dim JSONB array whose vector
                -- column is still NULL.
                UPDATE document_embeddings
                SET
                    embedding_vector = (
                        SELECT ARRAY(
                            SELECT (e)::float8
                            FROM jsonb_array_elements_text(embedding) AS e
                        )
                    )::vector,
                    -- Preserve any pre-existing `embedding_model`. For untagged
                    -- rows, stamp the assumed model AND an `embedding_model_assumed`
                    -- marker so the inference stays auditable and a later
                    -- re-embedding pass can find exactly these rows.
                    metadata = CASE
                        WHEN COALESCE(metadata, '{}'::jsonb) ? 'embedding_model'
                            THEN metadata
                        ELSE jsonb_set(
                            jsonb_set(
                                COALESCE(metadata, '{}'::jsonb),
                                '{embedding_model}',
                                to_jsonb('text-embedding-3-small'::text),
                                true
                            ),
                            '{embedding_model_assumed}',
                            'true'::jsonb,
                            true
                        )
                    END,
                    updated_at = NOW()
                WHERE embedding IS NOT NULL
                  AND embedding_vector IS NULL
                  AND jsonb_typeof(embedding) = 'array'
                  AND jsonb_array_length(embedding) = 1536;
            END;
            $inner$ LANGUAGE plpgsql
        $func$;

        COMMENT ON FUNCTION migrate_jsonb_to_vector IS
            '#2321: set-based back-fill of legacy JSONB embeddings into pgvector; stamps assumed embedding_model (text-embedding-3-small) plus an embedding_model_assumed=true marker on untagged rows so inferred provenance stays auditable.';

        RAISE NOTICE 'Redefined migrate_jsonb_to_vector() to add embedding_model_assumed marker';
    ELSE
        RAISE NOTICE 'pgvector extension not present - skipping migrate_jsonb_to_vector() redefinition';
    END IF;
END $$;
