//! Reality-server utility modules.
//!
//! Crate-internal helpers shared across route handlers. Keep this module
//! lean — anything reusable across servers belongs in `api-core` or `common`.

pub mod errors;
pub mod url_validator;

/// Process-wide mutex for tests that mutate global env vars (e.g. `RUST_ENV`).
///
/// Rust's test harness runs tests in parallel by default, so any test that
/// touches a shared env var has to serialize against every other such test
/// in the same process. Use [`env_lock`] in any test that reads or writes
/// `RUST_ENV` (and friends) to avoid flaky cross-test interference.
#[cfg(test)]
pub(crate) fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    use std::sync::{Mutex, OnceLock};
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    // Tests may panic while holding the guard; recovering from a poisoned
    // mutex is fine because the protected data is `()`.
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}
