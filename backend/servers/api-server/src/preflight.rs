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

/// A required production environment variable plus its minimum acceptable
/// length. The length floor encodes the constraint the *downstream* init
/// enforces, so a present-but-too-short value is rejected up front here instead
/// of panicking ~100 lines later (the partial recurrence of #951 that mere
/// presence-checking left open).
pub struct RequiredVar {
    pub name: &'static str,
    /// Minimum byte length of the trimmed value. `1` means "non-empty"; larger
    /// values mirror a hard floor enforced later in startup.
    pub min_len: usize,
}

/// Environment variables that are hard-required in production / staging and
/// have **no safe fallback** outside development. Keep this list conservative:
/// only vars that genuinely make the server boot half-broken (or panic later)
/// when absent — or present but below a known length floor — belong here.
///
/// - `DATABASE_URL` — no prod default; `main.rs` panics without it outside dev.
/// - `JWT_SECRET` — no prod default; `main.rs` panics if `< 32` chars
///   (`main.rs:519-521`), so the same `32` floor is enforced here.
/// - `ESIGN_TOKEN_SECRET` — required by `LightweightProvider::from_env()`
///   (issue #951); `LightweightConfig::from_env` rejects `< MIN_TOKEN_SECRET_LEN`
///   bytes (`esignature.rs:295`), referenced here to stay in sync.
/// - `ESIGN_WEBHOOK_SECRET` — required for the e-signature webhook receiver
///   (issue #951); empty/missing makes it reject all events.
pub const REQUIRED_PROD_ENV_VARS: &[RequiredVar] = &[
    RequiredVar {
        name: "DATABASE_URL",
        min_len: 1,
    },
    RequiredVar {
        name: "JWT_SECRET",
        min_len: 32, // matches the floor in main.rs
    },
    RequiredVar {
        name: "ESIGN_TOKEN_SECRET",
        min_len: integrations::MIN_TOKEN_SECRET_LEN,
    },
    RequiredVar {
        name: "ESIGN_WEBHOOK_SECRET",
        min_len: 1,
    },
];

/// Pure core of the preflight check, kept free of process-global state so it is
/// trivially testable.
///
/// `is_development` mirrors `main.rs`'s `RUST_ENV == "development"` gate. `lookup`
/// resolves a variable name to its value (returning `None` when unset); a value
/// that is present but empty/whitespace-only is treated as missing, because an
/// empty secret is as broken as an absent one. A value present and non-empty but
/// shorter than its `min_len` floor is reported separately as **too short**, so
/// a deploy injecting e.g. `JWT_SECRET=short` fails preflight rather than
/// panicking later in init.
///
/// Returns `Ok(())` when development is true (the per-var dev fallbacks in
/// `main.rs` handle those cases) or when every required var is present and meets
/// its length floor. Otherwise returns `Err` with a single human-readable
/// message listing **all** missing and **all** too-short vars.
pub fn check_required_env<F>(is_development: bool, lookup: F) -> Result<(), String>
where
    F: Fn(&str) -> Option<String>,
{
    if is_development {
        return Ok(());
    }

    let mut missing: Vec<&str> = Vec::new();
    let mut too_short: Vec<String> = Vec::new();

    for var in REQUIRED_PROD_ENV_VARS {
        match lookup(var.name) {
            Some(value) if value.trim().is_empty() => missing.push(var.name),
            Some(value) if value.trim().len() < var.min_len => {
                too_short.push(format!("{} (need >= {})", var.name, var.min_len));
            }
            Some(_) => {}
            None => missing.push(var.name),
        }
    }

    if missing.is_empty() && too_short.is_empty() {
        return Ok(());
    }

    let mut parts: Vec<String> = Vec::new();
    if !missing.is_empty() {
        parts.push(format!(
            "Missing required environment variables: {}",
            missing.join(", ")
        ));
    }
    if !too_short.is_empty() {
        parts.push(format!("Too short: {}", too_short.join(", ")));
    }
    Err(format!(
        "{}. Set RUST_ENV=development to use dev defaults.",
        parts.join(". ")
    ))
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

    /// A value long enough to clear every floor in `REQUIRED_PROD_ENV_VARS`
    /// (the largest is 32), so "present" rows in tests never trip the length
    /// check unless a test deliberately overrides them with a short value.
    const LONG_VALUE: &str = "set-value-long-enough-to-clear-all-floors";

    fn all_present() -> HashMap<&'static str, &'static str> {
        REQUIRED_PROD_ENV_VARS
            .iter()
            .map(|var| (var.name, LONG_VALUE))
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
        for var in REQUIRED_PROD_ENV_VARS {
            assert!(err.contains(var.name), "missing {} in: {err}", var.name);
        }
    }

    #[test]
    fn short_jwt_secret_in_prod_is_too_short() {
        let mut map = all_present();
        map.insert("JWT_SECRET", "short"); // present, non-empty, < 32

        let err = check_required_env(false, map_lookup(map))
            .expect_err("expected Err when JWT_SECRET is present but too short");
        assert!(err.contains("Too short"), "msg: {err}");
        assert!(err.contains("JWT_SECRET"), "msg: {err}");
        // A too-short var must not be double-reported as missing.
        assert!(!err.contains("Missing"), "msg: {err}");
    }

    #[test]
    fn short_esign_token_secret_in_prod_is_too_short() {
        let mut map = all_present();
        map.insert("ESIGN_TOKEN_SECRET", "too-short-secret"); // < MIN_TOKEN_SECRET_LEN (32)

        let err = check_required_env(false, map_lookup(map))
            .expect_err("expected Err when ESIGN_TOKEN_SECRET is present but too short");
        assert!(err.contains("Too short"), "msg: {err}");
        assert!(err.contains("ESIGN_TOKEN_SECRET"), "msg: {err}");
    }

    #[test]
    fn missing_and_too_short_are_both_reported() {
        let mut map = all_present();
        map.remove("DATABASE_URL"); // missing
        map.insert("JWT_SECRET", "short"); // too short

        let err = check_required_env(false, map_lookup(map))
            .expect_err("expected Err for combined missing + too-short");
        assert!(err.contains("Missing"), "msg: {err}");
        assert!(err.contains("DATABASE_URL"), "msg: {err}");
        assert!(err.contains("Too short"), "msg: {err}");
        assert!(err.contains("JWT_SECRET"), "msg: {err}");
    }
}
