//! Embedding provider abstraction for the RAG pipeline (Story 84.5).
//!
//! The RAG retrieval path (`routes/ai/sessions.rs` +
//! `LlmDocumentRepository::search_documents_*`) previously depended directly
//! on `LlmClient::generate_embedding`, which requires a live OpenAI API key.
//! This module introduces a provider trait so callers (indexing pipelines,
//! tests, air-gapped deployments) can swap the embedding backend:
//!
//! * [`EmbeddingProvider`] — the trait (single + batch embedding),
//! * [`LlmClient`] — implements it by delegating to the OpenAI embedding API,
//! * [`StubEmbeddingProvider`] — deterministic, offline, no-network provider
//!   for tests and local development. It is NOT semantically meaningful; it
//!   only guarantees determinism, unit norm, and dimension stability.

use async_trait::async_trait;
use sha2::{Digest, Sha256};

use crate::llm::{EmbeddingResult, LlmClient, LlmError};

/// Default embedding dimension (OpenAI `text-embedding-3-small`, matches the
/// `vector(1536)` column added by migration 00081).
pub const DEFAULT_EMBEDDING_DIM: usize = 1536;

/// Backend-agnostic embedding generation for RAG indexing and retrieval.
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Generate an embedding for a single text.
    async fn embed(&self, text: &str) -> Result<EmbeddingResult, LlmError>;

    /// Generate embeddings for multiple texts. Default: sequential `embed`.
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<EmbeddingResult>, LlmError> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.embed(text).await?);
        }
        Ok(results)
    }

    /// Dimension of the vectors this provider produces.
    fn dimension(&self) -> usize {
        DEFAULT_EMBEDDING_DIM
    }
}

/// The production provider: OpenAI embeddings via [`LlmClient`].
#[async_trait]
impl EmbeddingProvider for LlmClient {
    async fn embed(&self, text: &str) -> Result<EmbeddingResult, LlmError> {
        self.generate_embedding(text, None).await
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<EmbeddingResult>, LlmError> {
        self.generate_embeddings_batch(texts, None).await
    }
}

/// Deterministic offline embedding provider.
///
/// Produces a unit-norm vector derived from a SHA-256 hash chain over the
/// input text. The same text always yields the same vector; different texts
/// yield (with overwhelming probability) different vectors. Cosine similarity
/// between stub vectors carries no semantic meaning — use it only where
/// determinism matters more than relevance (tests, CI, local dev without an
/// OpenAI key).
#[derive(Debug, Clone)]
pub struct StubEmbeddingProvider {
    dim: usize,
}

impl StubEmbeddingProvider {
    /// Provider with the default (1536) dimension.
    pub fn new() -> Self {
        Self::with_dimension(DEFAULT_EMBEDDING_DIM)
    }

    /// Provider with a custom dimension (e.g. 3 for cheap tests).
    pub fn with_dimension(dim: usize) -> Self {
        assert!(dim > 0, "embedding dimension must be > 0");
        Self { dim }
    }

    /// Deterministically expand `text` into a unit-norm vector of `self.dim`.
    fn embed_sync(&self, text: &str) -> Vec<f32> {
        let mut values = Vec::with_capacity(self.dim);
        let mut counter: u64 = 0;
        let mut block = [0u8; 32];

        while values.len() < self.dim {
            let mut hasher = Sha256::new();
            hasher.update(text.as_bytes());
            hasher.update(counter.to_le_bytes());
            block.copy_from_slice(&hasher.finalize());
            counter += 1;

            for chunk in block.chunks_exact(4) {
                if values.len() == self.dim {
                    break;
                }
                let raw = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                // Map to [-1.0, 1.0)
                let v = (raw as f64 / u32::MAX as f64) * 2.0 - 1.0;
                values.push(v as f32);
            }
        }

        // L2-normalize so cosine similarity behaves like a dot product.
        let norm = values
            .iter()
            .map(|v| (*v as f64).powi(2))
            .sum::<f64>()
            .sqrt();
        if norm > 0.0 {
            for v in &mut values {
                *v = (*v as f64 / norm) as f32;
            }
        }
        values
    }
}

impl Default for StubEmbeddingProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EmbeddingProvider for StubEmbeddingProvider {
    async fn embed(&self, text: &str) -> Result<EmbeddingResult, LlmError> {
        Ok(EmbeddingResult {
            embedding: self.embed_sync(text),
            model: format!("stub-deterministic-{}", self.dim),
            tokens_used: 0,
        })
    }

    fn dimension(&self) -> usize {
        self.dim
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block_on<F: std::future::Future>(fut: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("build runtime")
            .block_on(fut)
    }

    #[test]
    fn stub_is_deterministic() {
        let provider = StubEmbeddingProvider::with_dimension(64);
        let a = block_on(provider.embed("lease agreement clause")).unwrap();
        let b = block_on(provider.embed("lease agreement clause")).unwrap();
        assert_eq!(a.embedding, b.embedding, "same text must embed identically");
    }

    #[test]
    fn stub_differs_for_different_texts() {
        let provider = StubEmbeddingProvider::with_dimension(64);
        let a = block_on(provider.embed("water damage in unit 4")).unwrap();
        let b = block_on(provider.embed("annual assembly minutes")).unwrap();
        assert_ne!(
            a.embedding, b.embedding,
            "different texts must produce different vectors"
        );
    }

    #[test]
    fn stub_produces_unit_norm_at_requested_dimension() {
        for dim in [3usize, 64, DEFAULT_EMBEDDING_DIM] {
            let provider = StubEmbeddingProvider::with_dimension(dim);
            assert_eq!(provider.dimension(), dim);
            let result = block_on(provider.embed("dimension check")).unwrap();
            assert_eq!(result.embedding.len(), dim);
            let norm: f64 = result
                .embedding
                .iter()
                .map(|v| (*v as f64).powi(2))
                .sum::<f64>()
                .sqrt();
            assert!(
                (norm - 1.0).abs() < 1e-3,
                "vector must be unit-norm, got {norm} at dim {dim}"
            );
        }
    }

    #[test]
    fn stub_batch_matches_single() {
        let provider = StubEmbeddingProvider::with_dimension(16);
        let texts = vec!["one".to_string(), "two".to_string()];
        let batch = block_on(provider.embed_batch(&texts)).unwrap();
        let single = block_on(provider.embed("one")).unwrap();
        assert_eq!(batch.len(), 2);
        assert_eq!(batch[0].embedding, single.embedding);
    }
}
