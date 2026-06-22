//! Document repository (Epic 7A: Basic Document Management, Epic 7B: Document Versioning, Epic 28: Document Intelligence).
//!
//! # RLS Integration
//!
//! This repository supports two usage patterns:
//!
//! 1. **RLS-aware** (recommended): Use methods with `_rls` suffix that accept an executor
//!    with RLS context already set (e.g., from `RlsConnection`).
//!
//! 2. **Legacy**: Use methods without suffix that use the internal pool. These do NOT
//!    enforce RLS and should be migrated to the RLS-aware pattern.
//!
//! ## Example
//!
//! ```rust,ignore
//! async fn create_document(
//!     mut rls: RlsConnection,
//!     State(state): State<AppState>,
//!     Json(data): Json<CreateDocument>,
//! ) -> Result<Json<Document>> {
//!     let document = state.document_repo.create_rls(rls.conn(), data).await?;
//!     rls.release().await;
//!     Ok(Json(document))
//! }
//! ```
//!
//! # Module layout
//!
//! The repository surface is large (folders, documents, shares, versions and
//! document-intelligence). To keep each area readable and to reduce the churn
//! surface of any single file, the `impl DocumentRepository` block is split
//! across cohesive sub-modules. Every sub-module adds methods to the *same*
//! [`DocumentRepository`] struct defined here:
//!
//! - [`folders`]      — folder CRUD + tree (Story 7A.2)
//! - [`documents`]    — document CRUD, listing, access checks & share-token lookups (Stories 7A.1, 7A.3)
//! - [`shares`]       — share links, tokens & access log (Story 7A.5)
//! - [`versions`]     — document versioning & restore (Story 7B.1)
//! - [`intelligence`] — OCR, search, classification, summarization & stats (Epic 28)
//!
//! Private row-mapping / token / password helpers live in [`internal`].

use sqlx::PgPool;

mod documents;
mod folders;
mod intelligence;
mod internal;
mod shares;
mod versions;

/// Repository for document operations.
#[derive(Debug, Clone)]
pub struct DocumentRepository {
    pub(super) pool: PgPool,
}

impl DocumentRepository {
    /// Create a new document repository.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
