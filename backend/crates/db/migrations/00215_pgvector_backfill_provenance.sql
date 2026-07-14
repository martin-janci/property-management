-- Migration: 00215_pgvector_backfill_provenance.sql
-- Epic 103 / Story 103.5: RAG pgvector back-fill correctness + performance (#2300).
--
-- Background
-- ----------
-- Migration 00081 defined `migrate_jsonb_to_vector()`, the back-fill that copies
-- a legacy 1536-dim JSONB `embedding` into the pgvector `embedding_vector`
-- column. PR #2291 wired it to `POST /api/v1/ai/llm/rag/migrate`. Post-merge
-- review (#2300) found two problems with the original definition:
--
--   1. correctness — it copied the vector but did NOT stamp
--      `metadata.embedding_model` provenance. Legacy rows predate provenance
--      tagging, so after a run they still carried no `embedding_model`. The
--      retrieval filter `filter_by_embedding_model` keeps null-provenance rows
--      (`None => true`) for backward compatibility, so a provenance-filtered
--      similarity search still mixed embedding spaces. Worse: before the
--      back-fill these rows had `embedding_vector IS NULL` and were invisible to
--      the pgvector retrieval path (`search_similar_documents` requires
--      `embedding_vector IS NOT NULL`); populating the column pulled the exact
--      untagged rows the task set out to isolate INTO the filtered result set.
--
--   2. performance — it was a row-by-row plpgsql loop with a per-row UPDATE,
--      run synchronously across every organization in one super-admin request.
--
-- This migration replaces the function (forward-only: 00081 is merged and is
-- never edited) with a single set-based UPDATE that also records provenance.
--
-- Provenance assumption
-- ---------------------
-- Legacy 1536-dim embeddings are assumed to be OpenAI `text-embedding-3-small`
-- — the default model used everywhere else (`integrations::llm`, the embed-write
-- path in `LlmDocumentRepository::upsert_embedding`, and `DEFAULT_EMBEDDING_DIM
-- = 1536`). Rows that ALREADY carry an `embedding_model` key keep their own
-- value; only untagged rows are stamped. Once stamped, a search filtered on a
-- DIFFERENT model correctly excludes them instead of silently mixing spaces.
--
-- Note: after a large back-fill the IVFFlat index (`idx_document_embeddings_vector`,
-- lists = 100) should be rebuilt (`REINDEX INDEX idx_document_embeddings_vector`)
-- to restore recall — that is an operational step, not part of this idempotent
-- data migration.

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'vector') THEN
        EXECUTE $func$
            CREATE OR REPLACE FUNCTION migrate_jsonb_to_vector()
            RETURNS void AS $inner$
            BEGIN
                -- Single set-based UPDATE (replaces the 00081 row-by-row loop).
                -- Only untouched legacy rows are converted: a 1536-dim JSONB
                -- array whose vector column is still NULL.
                UPDATE document_embeddings
                SET
                    embedding_vector = (
                        SELECT ARRAY(
                            SELECT (e)::float8
                            FROM jsonb_array_elements_text(embedding) AS e
                        )
                    )::vector,
                    -- Stamp assumed provenance so the retrieval filter can reason
                    -- about the row's embedding space. Preserve any pre-existing
                    -- `embedding_model` value; only fill it in when absent.
                    metadata = CASE
                        WHEN COALESCE(metadata, '{}'::jsonb) ? 'embedding_model'
                            THEN metadata
                        ELSE jsonb_set(
                            COALESCE(metadata, '{}'::jsonb),
                            '{embedding_model}',
                            to_jsonb('text-embedding-3-small'::text),
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
            'Story 103.5 / #2300: set-based back-fill of legacy JSONB embeddings into pgvector, stamping assumed embedding_model provenance (text-embedding-3-small) on untagged rows.';

        RAISE NOTICE 'Replaced migrate_jsonb_to_vector() with set-based, provenance-stamping definition';
    ELSE
        RAISE NOTICE 'pgvector extension not present - skipping migrate_jsonb_to_vector() redefinition';
    END IF;
END $$;
