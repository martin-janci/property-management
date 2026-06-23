//! Private helpers shared across the `document` sub-modules: folder-tree
//! assembly, share-token generation and password hashing/verification.

use crate::models::FolderTreeNode;
use sqlx::Error as SqlxError;
use uuid::Uuid;

// ============================================================================
// Helper Functions
// ============================================================================

/// Build folder tree from flat list.
pub(super) fn build_folder_tree(nodes: Vec<FolderTreeNode>) -> Vec<FolderTreeNode> {
    use std::collections::HashMap;

    let mut node_map: HashMap<Uuid, FolderTreeNode> = HashMap::new();
    let mut root_ids: Vec<Uuid> = Vec::new();

    // First pass: create all nodes
    for node in nodes {
        if node.parent_id.is_none() {
            root_ids.push(node.id);
        }
        node_map.insert(node.id, node);
    }

    // Second pass: build parent-child relationships
    let mut children_map: HashMap<Uuid, Vec<FolderTreeNode>> = HashMap::new();
    for node in node_map.values() {
        if let Some(parent_id) = node.parent_id {
            children_map
                .entry(parent_id)
                .or_default()
                .push(node.clone());
        }
    }

    // Third pass: attach children to parents
    fn attach_children(
        node: &mut FolderTreeNode,
        children_map: &HashMap<Uuid, Vec<FolderTreeNode>>,
    ) {
        if let Some(children) = children_map.get(&node.id) {
            let mut child_nodes: Vec<FolderTreeNode> = children.clone();
            for child in &mut child_nodes {
                attach_children(child, children_map);
            }
            node.children = Some(child_nodes);
        }
    }

    root_ids
        .iter()
        .filter_map(|id| {
            node_map.get(id).cloned().map(|mut node| {
                attach_children(&mut node, &children_map);
                node
            })
        })
        .collect()
}

/// Generate a secure random share token.
///
/// Uses OS CSPRNG directly rather than `thread_rng`.
pub(super) fn generate_share_token() -> String {
    use rand::RngExt;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::rand_core::UnwrapErr(rand::rngs::SysRng);
    (0..32)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Hash a password using Argon2.
pub(super) fn hash_password(password: &str) -> Result<String, SqlxError> {
    use argon2::{password_hash::PasswordHasher, Argon2};
    let argon2 = Argon2::default();
    // password-hash 0.6: `hash_password` generates a random salt internally
    // (via the `getrandom` feature) — no explicit `SaltString` needed.
    argon2
        .hash_password(password.as_bytes())
        .map(|h| h.to_string())
        .map_err(|e| {
            tracing::error!("Failed to hash password: {}", e);
            SqlxError::Protocol("Password hashing failed".to_string())
        })
}

/// Verify a password against a hash.
pub(super) fn verify_password(password: &str, hash: &str) -> bool {
    use argon2::{
        password_hash::{phc::PasswordHash, PasswordVerifier},
        Argon2,
    };
    let parsed_hash = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}
