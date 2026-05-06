// backend/servers/deploy-server/src/infra/worktree_lock.rs
//! Per-worktree mutex registry. Serializes concurrent open/close/GC on the same worktree name.
//!
//! Pattern: same as `GitFetcher::locks` but at the worktree level.
//! Locks live for the process lifetime; map grows with cumulative worktree names.
//! For Phase 1 scale (<100 unique names ever) this is acceptable; bounded LRU is a Phase 6 polish.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, OwnedMutexGuard};

#[derive(Clone, Default)]
pub struct WorktreeLockRegistry {
    locks: Arc<Mutex<HashMap<String, Arc<Mutex<()>>>>>,
}

impl WorktreeLockRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Block until the per-worktree lock is acquired.
    /// Returned guard releases on drop.
    pub async fn acquire(&self, name: &str) -> OwnedMutexGuard<()> {
        let mutex = {
            let mut map = self.locks.lock().await;
            map.entry(name.to_string())
                .or_insert_with(|| Arc::new(Mutex::new(())))
                .clone()
        };
        mutex.lock_owned().await
    }

    /// Try to acquire without blocking. Returns None if held by another task.
    /// Used by GC to skip worktrees currently being mutated by an open/close.
    pub async fn try_acquire(&self, name: &str) -> Option<OwnedMutexGuard<()>> {
        let mutex = {
            let mut map = self.locks.lock().await;
            map.entry(name.to_string())
                .or_insert_with(|| Arc::new(Mutex::new(())))
                .clone()
        };
        mutex.try_lock_owned().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn acquire_blocks_concurrent_holder() {
        let reg = WorktreeLockRegistry::new();
        let g1 = reg.acquire("foo").await;
        // try_acquire while held → None
        assert!(reg.try_acquire("foo").await.is_none());
        // try_acquire on different name → Some
        assert!(reg.try_acquire("bar").await.is_some());
        drop(g1);
        // After drop → can acquire again
        assert!(reg.try_acquire("foo").await.is_some());
    }
}
