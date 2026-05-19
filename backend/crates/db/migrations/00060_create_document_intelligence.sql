-- Epic 28: Document Intelligence
-- Story 28.1: Document OCR Text Extraction
-- Story 28.2: Document Full-Text Search
-- Story 28.3: Document Auto-Classification
-- Story 28.4: Document Summarization (extends existing)

-- ============================================================================
-- Story 28.1: OCR Text Extraction
-- ============================================================================

-- OCR status enum
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'ocr_status') THEN
        CREATE TYPE ocr_status AS ENUM (
            'pending',        -- Queued for OCR processing
            'processing',     -- Currently being processed
            'completed',      -- Successfully extracted
            'failed',         -- Extraction failed
            'not_applicable', -- File type doesn't support OCR
            'skipped'         -- Manually skipped
        );
    END IF;
END
$$;

-- Add OCR columns to documents table
ALTER TABLE documents ADD COLUMN IF NOT EXISTS extracted_text TEXT;
ALTER TABLE documents ADD COLUMN IF NOT EXISTS ocr_status ocr_status DEFAULT 'pending';
ALTER TABLE documents ADD COLUMN IF NOT EXISTS ocr_processed_at TIMESTAMPTZ;
ALTER TABLE documents ADD COLUMN IF NOT EXISTS ocr_error TEXT;
ALTER TABLE documents ADD COLUMN IF NOT EXISTS ocr_page_count INTEGER;
ALTER TABLE documents ADD COLUMN IF NOT EXISTS ocr_confidence DECIMAL(5,4);  -- 0.0000 to 1.0000

-- OCR processing queue for async processing
CREATE TABLE IF NOT EXISTS document_ocr_queue (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    document_id UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    priority INTEGER NOT NULL DEFAULT 5,  -- 1 (highest) to 10 (lowest)
    attempts INTEGER NOT NULL DEFAULT 0,
    max_attempts INTEGER NOT NULL DEFAULT 3,
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (document_id)
);

-- Index for efficient queue processing
CREATE INDEX IF NOT EXISTS idx_ocr_queue_pending ON document_ocr_queue(next_attempt_at)
    WHERE attempts < max_attempts;
CREATE INDEX IF NOT EXISTS idx_ocr_queue_priority ON document_ocr_queue(priority, next_attempt_at);

-- ============================================================================
-- Story 28.2: Full-Text Search
-- ============================================================================

-- Add full-text search vector column
ALTER TABLE documents ADD COLUMN IF NOT EXISTS search_vector tsvector;

-- Create GIN index for full-text search
CREATE INDEX IF NOT EXISTS idx_documents_search_vector ON documents USING GIN (search_vector);

-- Function to update search vector
CREATE OR REPLACE FUNCTION update_document_search_vector() RETURNS TRIGGER AS $$
BEGIN
    NEW.search_vector :=
        setweight(to_tsvector('english', COALESCE(NEW.title, '')), 'A') ||
        setweight(to_tsvector('english', COALESCE(NEW.description, '')), 'B') ||
        setweight(to_tsvector('english', COALESCE(NEW.file_name, '')), 'B') ||
        setweight(to_tsvector('english', COALESCE(NEW.extracted_text, '')), 'C') ||
        setweight(to_tsvector('english', COALESCE(NEW.ai_summary, '')), 'C');
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Trigger to auto-update search vector
DROP TRIGGER IF EXISTS trigger_update_document_search_vector ON documents;
CREATE TRIGGER trigger_update_document_search_vector
    BEFORE INSERT OR UPDATE OF title, description, file_name, extracted_text, ai_summary
    ON documents
    FOR EACH ROW
    EXECUTE FUNCTION update_document_search_vector();

-- Update existing documents' search vectors
-- Idempotency hardening: WHERE search_vector IS NULL makes replay cheap and
-- avoids a multi-minute table rewrite on dev DB resets / backup restores.
-- No semantic change on first run: at that point all rows already had NULL.
UPDATE documents SET search_vector =
    setweight(to_tsvector('english', COALESCE(title, '')), 'A') ||
    setweight(to_tsvector('english', COALESCE(description, '')), 'B') ||
    setweight(to_tsvector('english', COALESCE(file_name, '')), 'B') ||
    setweight(to_tsvector('english', COALESCE(extracted_text, '')), 'C') ||
    setweight(to_tsvector('english', COALESCE(ai_summary, '')), 'C')
WHERE search_vector IS NULL;

-- ============================================================================
-- Story 28.3: Document Auto-Classification
-- ============================================================================

-- Add classification columns to documents
ALTER TABLE documents ADD COLUMN IF NOT EXISTS ai_predicted_category TEXT;
ALTER TABLE documents ADD COLUMN IF NOT EXISTS ai_classification_confidence DECIMAL(5,4);  -- 0.0000 to 1.0000
ALTER TABLE documents ADD COLUMN IF NOT EXISTS ai_classification_at TIMESTAMPTZ;
ALTER TABLE documents ADD COLUMN IF NOT EXISTS ai_classification_accepted BOOLEAN;

-- Document classification history for model training
CREATE TABLE IF NOT EXISTS document_classification_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    document_id UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    predicted_category TEXT NOT NULL,
    confidence DECIMAL(5,4) NOT NULL,
    actual_category TEXT,  -- User-corrected category if different
    was_accepted BOOLEAN,
    feedback_by UUID REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_classification_history_doc ON document_classification_history(document_id);
CREATE INDEX IF NOT EXISTS idx_classification_history_accepted ON document_classification_history(was_accepted, created_at);

-- ============================================================================
-- Story 28.4: Enhanced Document Summarization
-- ============================================================================

-- Document topics for categorization
ALTER TABLE documents ADD COLUMN IF NOT EXISTS ai_topics JSONB DEFAULT '[]';

-- Summarization requests queue
CREATE TABLE IF NOT EXISTS document_summarization_queue (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    document_id UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    priority INTEGER NOT NULL DEFAULT 5,
    requested_by UUID REFERENCES users(id),
    attempts INTEGER NOT NULL DEFAULT 0,
    max_attempts INTEGER NOT NULL DEFAULT 3,
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (document_id)
);

CREATE INDEX IF NOT EXISTS idx_summarization_queue_pending ON document_summarization_queue(next_attempt_at)
    WHERE attempts < max_attempts;

-- ============================================================================
-- Processing Statistics
-- ============================================================================

-- Document intelligence processing statistics
CREATE TABLE IF NOT EXISTS document_intelligence_stats (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    date DATE NOT NULL DEFAULT CURRENT_DATE,
    documents_processed INTEGER NOT NULL DEFAULT 0,
    ocr_completed INTEGER NOT NULL DEFAULT 0,
    ocr_failed INTEGER NOT NULL DEFAULT 0,
    classifications_completed INTEGER NOT NULL DEFAULT 0,
    classifications_accepted INTEGER NOT NULL DEFAULT 0,
    summaries_generated INTEGER NOT NULL DEFAULT 0,
    total_pages_processed INTEGER NOT NULL DEFAULT 0,
    avg_ocr_confidence DECIMAL(5,4),
    avg_classification_confidence DECIMAL(5,4),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (organization_id, date)
);

CREATE INDEX IF NOT EXISTS idx_di_stats_org_date ON document_intelligence_stats(organization_id, date DESC);

-- RLS policies
ALTER TABLE document_ocr_queue ENABLE ROW LEVEL SECURITY;
ALTER TABLE document_classification_history ENABLE ROW LEVEL SECURITY;
ALTER TABLE document_summarization_queue ENABLE ROW LEVEL SECURITY;
ALTER TABLE document_intelligence_stats ENABLE ROW LEVEL SECURITY;

-- RLS policies based on document's organization
CREATE POLICY document_ocr_queue_tenant_isolation ON document_ocr_queue
    FOR ALL
    USING (EXISTS (
        SELECT 1 FROM documents d
        WHERE d.id = document_id
        AND d.organization_id = current_setting('app.current_organization_id', true)::uuid
    ));

CREATE POLICY document_classification_history_tenant_isolation ON document_classification_history
    FOR ALL
    USING (EXISTS (
        SELECT 1 FROM documents d
        WHERE d.id = document_id
        AND d.organization_id = current_setting('app.current_organization_id', true)::uuid
    ));

CREATE POLICY document_summarization_queue_tenant_isolation ON document_summarization_queue
    FOR ALL
    USING (EXISTS (
        SELECT 1 FROM documents d
        WHERE d.id = document_id
        AND d.organization_id = current_setting('app.current_organization_id', true)::uuid
    ));

CREATE POLICY document_intelligence_stats_tenant_isolation ON document_intelligence_stats
    FOR ALL
    USING (organization_id = current_setting('app.current_organization_id', true)::uuid);

-- Triggers for updated_at
CREATE TRIGGER update_document_ocr_queue_updated_at
    BEFORE UPDATE ON document_ocr_queue
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_document_intelligence_stats_updated_at
    BEFORE UPDATE ON document_intelligence_stats
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- Helper Functions
-- ============================================================================

-- Function to get full-text search rank
CREATE OR REPLACE FUNCTION document_search_rank(
    doc_search_vector tsvector,
    search_query tsquery
) RETURNS float4 AS $$
BEGIN
    RETURN ts_rank_cd(doc_search_vector, search_query);
END;
$$ LANGUAGE plpgsql IMMUTABLE;

-- Function to get search headline with highlighted matches
CREATE OR REPLACE FUNCTION document_search_headline(
    doc_text TEXT,
    search_query tsquery
) RETURNS TEXT AS $$
BEGIN
    RETURN ts_headline('english', COALESCE(doc_text, ''), search_query,
        'StartSel=<mark>, StopSel=</mark>, MaxWords=50, MinWords=20');
END;
$$ LANGUAGE plpgsql IMMUTABLE;

-- Function to queue document for OCR processing
CREATE OR REPLACE FUNCTION queue_document_for_ocr(
    p_document_id UUID,
    p_priority INTEGER DEFAULT 5
) RETURNS UUID AS $$
DECLARE
    queue_id UUID;
BEGIN
    -- Check if document supports OCR (PDF or image)
    IF NOT EXISTS (
        SELECT 1 FROM documents
        WHERE id = p_document_id
        AND (mime_type = 'application/pdf' OR mime_type LIKE 'image/%')
    ) THEN
        -- Mark as not applicable
        UPDATE documents SET ocr_status = 'not_applicable' WHERE id = p_document_id;
        RETURN NULL;
    END IF;

    INSERT INTO document_ocr_queue (document_id, priority)
    VALUES (p_document_id, p_priority)
    ON CONFLICT (document_id) DO UPDATE SET
        priority = LEAST(document_ocr_queue.priority, EXCLUDED.priority),
        next_attempt_at = NOW(),
        updated_at = NOW()
    RETURNING id INTO queue_id;

    UPDATE documents SET ocr_status = 'pending' WHERE id = p_document_id;

    RETURN queue_id;
END;
$$ LANGUAGE plpgsql;

-- Function to auto-queue new documents for processing
CREATE OR REPLACE FUNCTION auto_queue_document_intelligence() RETURNS TRIGGER AS $$
BEGIN
    -- Queue for OCR if applicable
    PERFORM queue_document_for_ocr(NEW.id, 5);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Trigger to auto-queue new documents
DROP TRIGGER IF EXISTS trigger_auto_queue_document ON documents;
CREATE TRIGGER trigger_auto_queue_document
    AFTER INSERT ON documents
    FOR EACH ROW
    EXECUTE FUNCTION auto_queue_document_intelligence();
