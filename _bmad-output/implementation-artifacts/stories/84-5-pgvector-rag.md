# Story 84.5: pgvector RAG Migration

Status: done

## Story

As a **system user**,
I want to **search documents using semantic similarity**,
So that **I can find relevant information even with different wording**.

## Acceptance Criteria

1. **AC-1: Vector Storage Migration**
   - Given the system uses document storage
   - When migrating to pgvector
   - Then vector embeddings are stored in PostgreSQL
   - And existing documents are re-indexed

2. **AC-2: Document Embedding**
   - Given a new document is uploaded
   - When processing is complete
   - Then text is extracted and chunked
   - And embeddings are generated and stored
   - And the document is searchable

3. **AC-3: Semantic Search**
   - Given I have indexed documents
   - When I search with a natural language query
   - Then semantically similar chunks are returned
   - And results are ranked by relevance
   - And source documents are identified

4. **AC-4: Hybrid Search**
   - Given I perform a search
   - When using hybrid mode
   - Then keyword and semantic results are combined
   - And the best matches from both are shown
   - And relevance is properly weighted

5. **AC-5: RAG Query Enhancement**
   - Given I ask a question about documents
   - When using AI-enhanced search
   - Then relevant context is retrieved
   - And the AI generates an answer using the context
   - And sources are cited

## Tasks / Subtasks

- [ ] Task 1: Set Up pgvector Extension (AC: 1)
  - [ ] 1.1 Enable pgvector extension in PostgreSQL
  - [ ] 1.2 Create vector column migration
  - [ ] 1.3 Configure vector index (IVFFlat or HNSW)
  - [ ] 1.4 Set up similarity search functions

- [ ] Task 2: Update Document Repository (AC: 1, 2)
  - [ ] 2.1 Update `/backend/crates/db/src/repositories/llm_document.rs:434`
  - [ ] 2.2 Add vector storage methods
  - [ ] 2.3 Implement chunk storage with embeddings
  - [ ] 2.4 Add batch upsert for efficiency

- [ ] Task 3: Implement Embedding Pipeline (AC: 2)
  - [ ] 3.1 Create text extraction service
  - [ ] 3.2 Implement chunking strategy
  - [ ] 3.3 Integrate embedding model (OpenAI/local)
  - [ ] 3.4 Create async processing queue

- [ ] Task 4: Implement Semantic Search (AC: 3)
  - [ ] 4.1 Update `/backend/crates/db/src/repositories/llm_document.rs:471`
  - [ ] 4.2 Create semantic search query
  - [ ] 4.3 Implement k-NN search with pgvector
  - [ ] 4.4 Add relevance scoring

- [ ] Task 5: Implement Hybrid Search (AC: 4)
  - [ ] 5.1 Create keyword search component
  - [ ] 5.2 Combine with semantic results
  - [ ] 5.3 Implement RRF (Reciprocal Rank Fusion)
  - [ ] 5.4 Expose hybrid mode option

- [ ] Task 6: Implement RAG API (AC: 5)
  - [ ] 6.1 Create RAG query endpoint
  - [ ] 6.2 Retrieve relevant context
  - [ ] 6.3 Send context to LLM
  - [ ] 6.4 Format response with citations
  - [ ] 6.5 Handle long context windows

## Dev Notes

### Architecture Requirements
- pgvector for vector storage
- Async embedding pipeline
- Hybrid keyword + semantic search
- LLM integration for RAG queries

### Technical Specifications
- Embedding model: text-embedding-3-small or local model
- Vector dimension: 1536 (OpenAI) or 384 (MiniLM)
- Chunk size: 512 tokens with 50 token overlap
- Index type: HNSW for better recall

### Existing TODO References
```rust
// backend/crates/db/src/repositories/llm_document.rs:434
// TODO: Migrate to pgvector
// - Use PostgreSQL vector extension
// - Store embeddings with document chunks

// backend/crates/db/src/repositories/llm_document.rs:471
// TODO: Implement semantic search with pgvector
// - k-NN search for similar chunks
// - Combine with keyword search
```

### Database Schema
```sql
-- Enable pgvector extension
CREATE EXTENSION IF NOT EXISTS vector;

-- Document chunks with embeddings
CREATE TABLE document_chunks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    document_id UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    chunk_index INTEGER NOT NULL,
    content TEXT NOT NULL,
    token_count INTEGER NOT NULL,
    embedding vector(1536) NOT NULL, -- 1536 for OpenAI, 384 for MiniLM
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(document_id, chunk_index)
);

-- HNSW index for fast similarity search
CREATE INDEX ON document_chunks USING hnsw (embedding vector_cosine_ops);

-- Full-text search index for hybrid search
CREATE INDEX idx_chunks_content_fts ON document_chunks USING gin(to_tsvector('english', content));
```

### Embedding Pipeline
```rust
pub struct EmbeddingPipeline {
    extractor: Box<dyn TextExtractor>,
    chunker: TextChunker,
    embedding_service: Box<dyn EmbeddingService>,
    chunk_repo: Arc<ChunkRepository>,
}

impl EmbeddingPipeline {
    pub async fn process_document(&self, document: &Document) -> Result<(), PipelineError> {
        // 1. Extract text
        let text = self.extractor.extract(&document.content, &document.content_type)?;

        // 2. Chunk text
        let chunks = self.chunker.chunk(&text, ChunkConfig {
            max_tokens: 512,
            overlap_tokens: 50,
        });

        // 3. Generate embeddings in batches
        let embeddings = self.embedding_service
            .embed_batch(&chunks.iter().map(|c| c.text.as_str()).collect::<Vec<_>>())
            .await?;

        // 4. Store chunks with embeddings
        let chunk_records: Vec<_> = chunks.iter()
            .zip(embeddings.iter())
            .enumerate()
            .map(|(i, (chunk, embedding))| ChunkRecord {
                document_id: document.id,
                chunk_index: i as i32,
                content: chunk.text.clone(),
                token_count: chunk.token_count as i32,
                embedding: embedding.clone(),
                metadata: chunk.metadata.clone(),
            })
            .collect();

        self.chunk_repo.upsert_batch(&chunk_records).await?;

        Ok(())
    }
}
```

### Semantic Search
```rust
impl ChunkRepository {
    pub async fn semantic_search(
        &self,
        query_embedding: &[f32],
        limit: i32,
        threshold: f32,
    ) -> Result<Vec<SearchResult>, DbError> {
        sqlx::query_as!(
            SearchResult,
            r#"
            SELECT
                dc.id,
                dc.document_id,
                dc.content,
                d.title as document_title,
                1 - (dc.embedding <=> $1::vector) as score
            FROM document_chunks dc
            JOIN documents d ON d.id = dc.document_id
            WHERE 1 - (dc.embedding <=> $1::vector) > $3
            ORDER BY dc.embedding <=> $1::vector
            LIMIT $2
            "#,
            query_embedding as _,
            limit,
            threshold,
        )
        .fetch_all(&self.pool)
        .await
    }
}
```

### Hybrid Search with RRF
```rust
impl SearchService {
    pub async fn hybrid_search(
        &self,
        query: &str,
        limit: i32,
    ) -> Result<Vec<SearchResult>, SearchError> {
        // 1. Get semantic results
        let query_embedding = self.embedding_service.embed(query).await?;
        let semantic_results = self.chunk_repo
            .semantic_search(&query_embedding, limit * 2, 0.5)
            .await?;

        // 2. Get keyword results
        let keyword_results = self.chunk_repo
            .keyword_search(query, limit * 2)
            .await?;

        // 3. Combine with RRF
        let combined = self.reciprocal_rank_fusion(
            &semantic_results,
            &keyword_results,
            60, // k parameter
        );

        Ok(combined.into_iter().take(limit as usize).collect())
    }

    fn reciprocal_rank_fusion(
        &self,
        list1: &[SearchResult],
        list2: &[SearchResult],
        k: i32,
    ) -> Vec<SearchResult> {
        let mut scores: HashMap<Uuid, f32> = HashMap::new();

        for (rank, result) in list1.iter().enumerate() {
            *scores.entry(result.id).or_default() += 1.0 / (k as f32 + rank as f32 + 1.0);
        }

        for (rank, result) in list2.iter().enumerate() {
            *scores.entry(result.id).or_default() += 1.0 / (k as f32 + rank as f32 + 1.0);
        }

        let mut combined: Vec<_> = list1.iter()
            .chain(list2.iter())
            .collect::<HashSet<_>>()
            .into_iter()
            .map(|r| (r.clone(), scores[&r.id]))
            .collect();

        combined.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        combined.into_iter().map(|(r, _)| r).collect()
    }
}
```

### RAG Query
```rust
pub struct RAGService {
    search_service: Arc<SearchService>,
    llm_client: Arc<LLMClient>,
}

impl RAGService {
    pub async fn query(&self, question: &str) -> Result<RAGResponse, RAGError> {
        // 1. Retrieve relevant chunks
        let chunks = self.search_service
            .hybrid_search(question, 5)
            .await?;

        // 2. Build context
        let context = chunks.iter()
            .map(|c| format!("[Source: {}]\n{}", c.document_title, c.content))
            .collect::<Vec<_>>()
            .join("\n\n");

        // 3. Query LLM with context
        let prompt = format!(
            "Answer the question based on the following context. Cite sources.\n\n\
            Context:\n{}\n\n\
            Question: {}\n\n\
            Answer:",
            context,
            question
        );

        let answer = self.llm_client.complete(&prompt).await?;

        Ok(RAGResponse {
            answer,
            sources: chunks.into_iter().map(|c| Source {
                document_id: c.document_id,
                document_title: c.document_title,
                excerpt: c.content,
            }).collect(),
        })
    }
}
```

### File List (to create/modify)

**Create:**
- `/backend/crates/db/migrations/NNNN_add_pgvector.sql`
- `/backend/crates/db/src/repositories/document_chunk.rs`
- `/backend/crates/common/src/embedding/mod.rs`
- `/backend/crates/common/src/embedding/pipeline.rs`
- `/backend/crates/common/src/embedding/chunker.rs`
- `/backend/servers/api-server/src/services/rag.rs`
- `/backend/servers/api-server/src/routes/rag.rs`

**Modify:**
- `/backend/crates/db/src/repositories/llm_document.rs` - Vector storage
- `/backend/crates/db/src/repositories/mod.rs` - Export modules
- `/backend/servers/api-server/src/routes/mod.rs` - Add RAG routes

### Dependencies
- PostgreSQL 15+ with pgvector extension
- OpenAI API or local embedding model

### References
- [Source: backend/crates/db/src/repositories/llm_document.rs:434,471]
- [pgvector Documentation]
- [UC-10: Document Management]
