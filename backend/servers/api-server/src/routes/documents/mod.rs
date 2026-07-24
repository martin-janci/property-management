//! Document routes (Epic 7A, 7B, 28, 92) — split by surface.
//!
//! | Surface      | Module         | Handlers |
//! |--------------|----------------|----------|
//! | core         | `core`         | CRUD, move, access, download, preview |
//! | versions     | `versions`     | Version history, create, get, restore |
//! | folders      | `folders`      | Folder tree, CRUD |
//! | shares       | `shares`       | Share CRUD, public access |
//! | intelligence | `intelligence` | OCR, classification, search, summarization |

pub mod core;
pub mod folders;
pub mod intelligence;
pub mod shares;
pub mod versions;

use axum::Router;

use crate::state::AppState;

/// Assemble the documents router from all surface sub-modules.
pub fn router() -> Router<AppState> {
    Router::new()
        .merge(core::router())
        .merge(versions::router())
        .merge(folders::router())
        .merge(shares::authenticated_router())
        .merge(intelligence::router())
        // Signature requests are a document sub-resource; see BIT-313 —
        // the handlers extract `Path(document_id)`, so they are mounted here
        // as `/api/v1/documents/{id}/signature-requests` rather than under the
        // bare `/api/v1/signature-requests` root where extraction had no
        // segment to bind.
        .merge(crate::routes::signatures::document_signature_router())
}

/// Public (no-auth) document routes — shared document access.
///
/// Kept separate so `lib.rs` can mount these outside the authenticated
/// `/api/v1/documents` prefix (they are already mounted at the correct path
/// by the share handlers themselves).
pub fn public_router() -> Router<AppState> {
    shares::public_router()
}
