-- Migration: 00194_fix_rag_statistics_view.sql
-- Epic 103 / Story 103.5: pgvector migration for RAG
--
-- Fixes a latent column-name mismatch between the `v_rag_statistics` view
-- (defined in 00081_create_pgvector.sql) and the repository code that reads
-- it (`LlmDocumentRepository::get_rag_statistics`, the `RagStatistics` model).
--
-- The original view exposed:
--     indexed_documents, total_chunks, chunks_with_embedding,
--     avg_chunk_length, last_updated
-- but the repository SELECTs:
--     indexed_documents, total_chunks, chunks_with_vector,
--     chunks_pending_migration, avg_chunk_length, last_updated
-- so any call to `get_rag_statistics` on a DB where the view exists fails at
-- runtime with `column "chunks_with_vector" does not exist`.
--
-- This migration re-creates the view with the columns the code expects, while
-- keeping the JSONB-fallback semantics intact:
--   * chunks_with_vector       — chunks that have an embedding stored
--                                (the pgvector `embedding_vector` column when
--                                 the extension is enabled, else the JSONB
--                                 `embedding` column on JSONB-only deployments)
--   * chunks_pending_migration — chunks that still carry a JSONB embedding but
--                                have not been migrated to `embedding_vector`
--                                (always 0 on JSONB-only deployments, since
--                                 there is no vector column to migrate into)
--
-- Because `embedding_vector` only exists when pgvector is installed, the view
-- body is built dynamically so this migration succeeds on both kinds of DB.
--
-- The view already exists from 00081_create_pgvector.sql with a different column
-- set (`chunks_with_embedding` in the slot now used by `chunks_with_vector`).
-- `CREATE OR REPLACE VIEW` cannot rename/reorder an existing view's columns, so
-- drop the old view first to allow the new column set to be installed cleanly.
DROP VIEW IF EXISTS v_rag_statistics;

DO $$
DECLARE
    v_has_vector_col BOOLEAN;
BEGIN
    SELECT EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_name = 'document_embeddings'
          AND column_name = 'embedding_vector'
    ) INTO v_has_vector_col;

    IF v_has_vector_col THEN
        -- pgvector deployment: distinguish vector-backed chunks from chunks
        -- still pending JSONB->vector migration.
        EXECUTE $view$
            CREATE OR REPLACE VIEW v_rag_statistics AS
            SELECT
                organization_id,
                COUNT(DISTINCT document_id) AS indexed_documents,
                COUNT(*) AS total_chunks,
                COUNT(*) FILTER (WHERE embedding_vector IS NOT NULL)
                    AS chunks_with_vector,
                COUNT(*) FILTER (
                    WHERE embedding IS NOT NULL AND embedding_vector IS NULL
                ) AS chunks_pending_migration,
                AVG(char_length(chunk_text))::INT AS avg_chunk_length,
                MAX(updated_at) AS last_updated
            FROM document_embeddings
            GROUP BY organization_id
        $view$;
    ELSE
        -- JSONB-only deployment: there is no vector column, so a chunk with a
        -- JSONB embedding counts as "with vector" for stats purposes, and
        -- nothing is pending vector migration.
        EXECUTE $view$
            CREATE OR REPLACE VIEW v_rag_statistics AS
            SELECT
                organization_id,
                COUNT(DISTINCT document_id) AS indexed_documents,
                COUNT(*) AS total_chunks,
                COUNT(*) FILTER (WHERE embedding IS NOT NULL)
                    AS chunks_with_vector,
                0::BIGINT AS chunks_pending_migration,
                AVG(char_length(chunk_text))::INT AS avg_chunk_length,
                MAX(updated_at) AS last_updated
            FROM document_embeddings
            GROUP BY organization_id
        $view$;
    END IF;
END $$;

GRANT SELECT ON v_rag_statistics TO PUBLIC;

COMMENT ON VIEW v_rag_statistics IS
    'Story 103.5: RAG system statistics per organization (columns aligned with RagStatistics model in 00194)';
