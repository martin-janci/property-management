//! Boot-time preflight for required production environment variables.
//!
//! Recurrence guard for issue #951: the deploy that shipped the e-signature
//! feature didn't inject `ESIGN_TOKEN_SECRET` / `ESIGN_WEBHOOK_SECRET`, so
//! `api-server` panicked deep inside startup (`LightweightProvider::from_env()`)
//! and the blue/green deploy aborted with a half-broken container. PR #949
//! fixed the missing injection in the deploy-server; this preflight is the
//! belt-and-braces guard so that *any* future missing required secret is caught
//! up front with ONE clear error listing exactly what's missing — before the
//! server does any heavy init or starts accepting connections.
//!
//! The check is gated to non-development environments (matching how `main.rs`
//! derives `is_development` from `RUST_ENV == "development"`) so it never breaks
//! local `cargo run`.

/// Environment variables that are hard-required in production / staging and
/// have **no safe fallback** outside development. Keep this list conservative:
/// only vars that genuinely make the server boot half-broken (or panic later)
/// when absent belong here.
///
/// - `DATABASE_URL` — no prod default; `main.rs` panics without it outside dev.
/// - `JWT_SECRET` — no prod default; auth tokens cannot be issued/verified.
/// - `ESIGN_TOKEN_SECRET` — required by `LightweightProvider::from_env()`
///   (issue #951).
/// - `ESIGN_WEBHOOK_SECRET` — required for the e-signature webhook receiver
///   (issue #951); empty/missing makes it reject all events.
pub const REQUIRED_PROD_ENV_VARS: &[&str] = &[
    "DATABASE_URL",
    "JWT_SECRET",
    "ESIGN_TOKEN_SECRET",
    "ESIGN_WEBHOOK_SECRET",
];

/// Pure core of the preflight check, kept free of process-global state so it is
/// trivially testable.
///
/// `is_development` mirrors `main.rs`'s `RUST_ENV == "development"` gate. `lookup`
/// resolves a variable name to its value (returning `None` when unset); a value
/// that is present but empty/whitespace-only is treated as missing, because an
/// empty secret is as broken as an absent one.
///
/// Returns `Ok(())` when development is true (the per-var dev fallbacks in
/// `main.rs` handle those cases) or when every required var is present and
/// non-empty. Otherwise returns `Err` with a single human-readable message
/// listing **all** missing vars.
pub fn check_required_env<F>(is_development: bool, lookup: F) -> Result<(), String>
where
    F: Fn(&str) -> Option<String>,
{
    if is_development {
        return Ok(());
    }

    let missing: Vec<&str> = REQUIRED_PROD_ENV_VARS
        .iter()
        .copied()
        .filter(|name| match lookup(name) {
            Some(value) => value.trim().is_empty(),
            None => true,
        })
        .collect();

    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Missing required environment variables: {}. \
             Set RUST_ENV=development to use dev defaults.",
            missing.join(", ")
        ))
    }
}

/// Run the preflight against the real process environment.
///
/// Call this early in `main()` — after `dotenvy::dotenv()` so `.env` files are
/// honoured, but before any heavy init (DB pool, migrations, server bind). In
/// development it is a no-op (the per-var fallbacks in `main.rs` apply); outside
/// development a missing required var produces an `Err` that `main` propagates,
/// so the process exits non-zero before partial initialization.
pub fn run() -> Result<(), String> {
    let is_development = std::env::var("RUST_ENV").unwrap_or_default() == "development";
    check_required_env(is_development, |name| std::env::var(name).ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Build a lookup closure backed by a fixed map, so tests never touch the
    /// real process environment.
    fn map_lookup(map: HashMap<&'static str, &'static str>) -> impl Fn(&str) -> Option<String> {
        move |name: &str| map.get(name).map(|v| v.to_string())
    }

    fn all_present() -> HashMap<&'static str, &'static str> {
        REQUIRED_PROD_ENV_VARS
            .iter()
            .map(|name| (*name, "set-value"))
            .collect()
    }

    #[test]
    fn all_present_in_prod_is_ok() {
        let result = check_required_env(false, map_lookup(all_present()));
        assert!(result.is_ok(), "expected Ok, got {result:?}");
    }

    #[test]
    fn missing_esign_secrets_in_prod_lists_them() {
        let mut map = all_present();
        map.remove("ESIGN_TOKEN_SECRET");
        map.remove("ESIGN_WEBHOOK_SECRET");

        let err = check_required_env(false, map_lookup(map))
            .expect_err("expected Err when ESIGN secrets are missing");

        assert!(err.contains("ESIGN_TOKEN_SECRET"), "msg: {err}");
        assert!(err.contains("ESIGN_WEBHOOK_SECRET"), "msg: {err}");
        // Vars that ARE present must not be reported as missing.
        assert!(!err.contains("DATABASE_URL"), "msg: {err}");
        assert!(!err.contains("JWT_SECRET"), "msg: {err}");
    }

    #[test]
    fn empty_value_counts_as_missing() {
        let mut map = all_present();
        map.insert("ESIGN_WEBHOOK_SECRET", "   "); // whitespace-only

        let err = check_required_env(false, map_lookup(map))
            .expect_err("expected Err when a required var is empty");
        assert!(err.contains("ESIGN_WEBHOOK_SECRET"), "msg: {err}");
    }

    #[test]
    fn development_skips_check_even_when_all_missing() {
        let empty: HashMap<&'static str, &'static str> = HashMap::new();
        let result = check_required_env(true, map_lookup(empty));
        assert!(
            result.is_ok(),
            "development must never fail preflight, got {result:?}"
        );
    }

    #[test]
    fn missing_all_lists_every_required_var() {
        let empty: HashMap<&'static str, &'static str> = HashMap::new();
        let err = check_required_env(false, map_lookup(empty))
            .expect_err("expected Err when everything is missing");
        for name in REQUIRED_PROD_ENV_VARS {
            assert!(err.contains(name), "missing {name} in: {err}");
        }
    }
}
