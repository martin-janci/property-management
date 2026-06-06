//! Database Seed CLI.
//!
//! Seeds the database with sample data for development and testing.
//!
//! # Usage
//!
//! ```bash
//! # Interactive mode (prompts for admin credentials)
//! cargo run -p api-server --bin ppt-seed
//!
//! # Non-interactive mode
//! cargo run -p api-server --bin ppt-seed -- \
//!   --admin-email admin@example.com \
//!   --admin-password SecurePass123
//!
//! # Force re-seed (drops existing seed data)
//! cargo run -p api-server --bin ppt-seed -- --force
//!
//! # Minimal seed (admin only, no sample data)
//! cargo run -p api-server --bin ppt-seed -- --minimal
//! ```

use argon2::password_hash::PasswordHasher;
use argon2::Argon2;
use clap::Parser;
use db::seed::{SeedConfig, SeedError, SeedRunner};
use dialoguer::{Confirm, Input, Password};

/// Default reality-portal OAuth redirect URIs covering the standard prod +
/// staging Reality Portal apexes. Operators with custom apex hostnames can
/// override via repeated `--reality-portal-redirect-uri` flags.
///
/// Uses the `api.<apex>` subdomain because the bare apex (`rlt.sk`,
/// `staging.rlt.sk`) reverse-proxies to reality-web (Next.js) which has no
/// `/api/v1/sso/*` handler — the SSO callback 404s if the redirect_uri points
/// at the apex. `api.<apex>` is proxied by Caddy to reality-server :8081,
/// which serves the `/api/v1/sso/callback` handler (fix for #952).
const DEFAULT_REALITY_PORTAL_REDIRECT_URIS: &[&str] = &[
    "https://api.rlt.sk/api/v1/sso/callback",
    "https://api.staging.rlt.sk/api/v1/sso/callback",
];

/// UPSERT the `reality-portal` row in `oauth_clients` with the given secret.
/// Idempotent: subsequent runs with the same secret rewrite the hash (Argon2
/// salts are random so the column changes byte-for-byte but verifies against
/// the same plaintext). Used on first prod/staging deploy to bootstrap the
/// SSO handshake between reality-server and api-server.
async fn upsert_reality_portal_client(
    pool: &sqlx::PgPool,
    secret: &str,
    redirect_uris: &[String],
) -> anyhow::Result<()> {
    if secret.len() < 32 {
        return Err(anyhow::anyhow!(
            "--reality-portal-secret must be at least 32 characters \
             (matches the validation in deploy-server's `build_service_envs`)"
        ));
    }
    let argon2 = Argon2::default();
    // password-hash 0.6: `hash_password` generates a random salt internally.
    let hash = argon2
        .hash_password(secret.as_bytes())
        .map_err(|e| anyhow::anyhow!("argon2 hash failed: {e}"))?
        .to_string();

    // Store as JSONB array. The schema's `redirect_uris` column is JSONB,
    // and the OAuth handler validates the requested redirect_uri against
    // exact entries in this array on every authorize/token call.
    let redirect_uris_json = serde_json::to_value(redirect_uris)?;
    let scopes_json = serde_json::json!(["profile", "openid"]);

    sqlx::query(
        r#"
        INSERT INTO oauth_clients (
            client_id, client_secret_hash, name, description,
            redirect_uris, scopes, is_confidential, rotate_refresh_tokens, is_active
        ) VALUES (
            'reality-portal', $1, 'Reality Portal',
            'SSO bridge from reality-server to api-server (seeded by ppt-seed --reality-portal-secret)',
            $2::jsonb, $3::jsonb, true, true, true
        )
        ON CONFLICT (client_id) DO UPDATE SET
            client_secret_hash = EXCLUDED.client_secret_hash,
            redirect_uris      = EXCLUDED.redirect_uris,
            scopes             = EXCLUDED.scopes,
            is_active          = true,
            updated_at         = NOW();
        "#,
    )
    .bind(&hash)
    .bind(&redirect_uris_json)
    .bind(&scopes_json)
    .execute(pool)
    .await?;
    Ok(())
}

#[derive(Parser)]
#[command(name = "ppt-seed")]
#[command(about = "Seed database with sample data for development")]
#[command(long_about = r#"
Seeds the database with comprehensive sample data including:
  - 1 organization (Demo Property Management)
  - Users for all 12 role types
  - 3 buildings with 19 units total
  - Unit resident assignments

All sample users use the email domain @demo-property.test for easy identification.
"#)]
struct Cli {
    /// Admin email address
    #[arg(long)]
    admin_email: Option<String>,

    /// Admin password (will prompt if not provided)
    #[arg(long)]
    admin_password: Option<String>,

    /// Force re-seed (drops existing seed data)
    #[arg(long, short)]
    force: bool,

    /// Minimal seed (admin only, no sample buildings/users)
    #[arg(long)]
    minimal: bool,

    /// Skip confirmation prompts
    #[arg(long, short = 'y')]
    yes: bool,

    /// Plaintext OAuth client secret for the `reality-portal` client. When
    /// provided, the seeder UPSERTs a row into `oauth_clients` with
    /// `client_id="reality-portal"` and `client_secret_hash` = Argon2id of
    /// this value. Reality-server reads its matching plaintext from
    /// `PM_CLIENT_SECRET` (set by the deploy-server from
    /// `/etc/ppt-deploy/secrets.env::PPT_PM_CLIENT_SECRET`) — both sides
    /// must agree, so on first prod deploy run this with the same value:
    ///
    ///     ppt-seed --reality-portal-secret "$PPT_PM_CLIENT_SECRET"
    #[arg(long, env = "PPT_PM_CLIENT_SECRET")]
    reality_portal_secret: Option<String>,

    /// Allowed OAuth redirect URI for the reality-portal client (repeatable).
    /// If none specified, defaults to the prod + staging Reality Portal SSO
    /// callback URLs.
    #[arg(long = "reality-portal-redirect-uri")]
    reality_portal_redirect_uris: Vec<String>,
}

fn validate_password(password: &str) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    if password.len() < 8 {
        errors.push("Password must be at least 8 characters".to_string());
    }
    if !password.chars().any(|c| c.is_uppercase()) {
        errors.push("Password must contain at least one uppercase letter".to_string());
    }
    if !password.chars().any(|c| c.is_numeric()) {
        errors.push("Password must contain at least one number".to_string());
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("sqlx::query=warn".parse()?)
                .add_directive("ppt_seed=info".parse()?),
        )
        .init();

    let cli = Cli::parse();

    // Get database URL
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL environment variable must be set");

    println!();
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║         Property Management Database Seeder              ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();

    // Interactive prompts for missing credentials
    let admin_email = match cli.admin_email {
        Some(email) => email,
        None => Input::new()
            .with_prompt("Admin email")
            .default("admin@ppt.local".to_string())
            .interact_text()?,
    };

    let admin_password = match cli.admin_password {
        Some(pass) => {
            // Validate provided password
            if let Err(errors) = validate_password(&pass) {
                eprintln!("Password validation failed:");
                for e in errors {
                    eprintln!("  ✗ {}", e);
                }
                return Err(anyhow::anyhow!("Invalid password"));
            }
            pass
        }
        None => loop {
            let pass = Password::new()
                .with_prompt("Admin password")
                .with_confirmation("Confirm password", "Passwords don't match")
                .interact()?;

            // Validate password requirements
            match validate_password(&pass) {
                Ok(()) => break pass,
                Err(errors) => {
                    eprintln!("Password validation failed:");
                    for e in errors {
                        eprintln!("  ✗ {}", e);
                    }
                    eprintln!("Please try again.\n");
                }
            }
        },
    };

    // Show configuration summary
    println!();
    println!("┌──────────────────────────────────────────────────────────┐");
    println!("│ Seed Configuration                                       │");
    println!("├──────────────────────────────────────────────────────────┤");
    println!("│  Admin Email:    {:<40} │", admin_email);
    println!(
        "│  Sample Data:    {:<40} │",
        if cli.minimal {
            "No (admin only)"
        } else {
            "Yes (full dataset)"
        }
    );
    println!(
        "│  Force Re-seed:  {:<40} │",
        if cli.force { "Yes" } else { "No" }
    );
    println!("└──────────────────────────────────────────────────────────┘");

    if !cli.minimal {
        println!();
        println!("Sample data includes:");
        println!("  • 1 organization (Demo správa nehnuteľností)");
        println!("  • 15 PPT users covering all 12 role types");
        println!("  • 3 buildings with 19 units total");
        println!("  • Unit resident assignments");
        println!("  • 6 faults in various states (new, triaged, in_progress, resolved, closed)");
        println!("  • 2 votes (1 closed with results, 1 active)");
        println!("  • 4 announcements (published, pinned, archived)");
        println!("  • 6 listings (sale & rent, SK locale)");
        println!("  • 4 Reality Portal users (3 buyers/renters, 1 realtor)");
        println!("  • 3 inquiries with message threads");
        println!("  • 3 portal favorites");
        println!();
        println!("PPT sample users use password: DemoHeslo123");
        println!("PPT sample emails end with:   @demo-property.test");
        println!("Portal users use password:    PortalHeslo123");
        println!("Portal emails end with:        @demo-reality.test");
    }

    // Confirmation
    if !cli.yes {
        println!();
        if !Confirm::new()
            .with_prompt("Proceed with seeding?")
            .default(true)
            .interact()?
        {
            println!("Aborted.");
            return Ok(());
        }
    }

    println!();
    println!("Connecting to database...");

    // Connect to database
    let pool = db::create_rls_safe_pool(&database_url).await?;

    println!("Connected. Starting seed process...");
    println!();

    // Create seed configuration
    let config = SeedConfig {
        admin_email: admin_email.clone(),
        admin_password,
        include_sample_data: !cli.minimal,
        force: cli.force,
    };

    // Run seeder
    let runner = SeedRunner::new(pool, config);

    match runner.run().await {
        Ok(result) => {
            // Print cleanup stats if force was used
            if let Some(stats) = &result.cleanup_stats {
                println!("┌──────────────────────────────────────────────────────────┐");
                println!("│ Cleanup (existing seed data removed)                     │");
                println!("├──────────────────────────────────────────────────────────┤");
                println!("│  Users deleted:         {:>30} │", stats.users_deleted);
                println!(
                    "│  Organizations deleted: {:>30} │",
                    stats.organizations_deleted
                );
                println!(
                    "│  Buildings deleted:     {:>30} │",
                    stats.buildings_deleted
                );
                println!("│  Units deleted:         {:>30} │", stats.units_deleted);
                println!("└──────────────────────────────────────────────────────────┘");
                println!();
            }

            println!("╔══════════════════════════════════════════════════════════╗");
            println!("║                    Seed Complete ✓                       ║");
            println!("╠══════════════════════════════════════════════════════════╣");
            println!("║  Organizations:  {:>38} ║", result.organizations_created);
            println!("║  PPT Users:      {:>38} ║", result.users_created);
            println!("║  Buildings:      {:>38} ║", result.buildings_created);
            println!("║  Units:          {:>38} ║", result.units_created);
            println!("║  Residents:      {:>38} ║", result.residents_assigned);
            println!("║  Faults:         {:>38} ║", result.faults_created);
            println!("║  Votes:          {:>38} ║", result.votes_created);
            println!("║  Announcements:  {:>38} ║", result.announcements_created);
            println!("╠══════════════════════════════════════════════════════════╣");
            println!("║  Listings:       {:>38} ║", result.listings_created);
            println!("║  Portal Users:   {:>38} ║", result.portal_users_created);
            println!("║  Inquiries:      {:>38} ║", result.inquiries_created);
            println!("║  Favorites:      {:>38} ║", result.favorites_created);
            println!("╠══════════════════════════════════════════════════════════╣");
            println!("║  Admin User ID:  {} ║", result.admin_user_id);
            println!("║  Organization:   {} ║", result.organization_id);
            println!("╚══════════════════════════════════════════════════════════╝");
            println!();
            println!("You can now login with:");
            println!("  Email: {}", admin_email);
            println!("  Password: <the password you provided>");
            println!();

            // Optional: bootstrap the reality-portal OAuth client so the
            // reality-server ↔ api-server SSO/OAuth handshake works on
            // first deploy. Without this, every login attempt through
            // reality-server hits "invalid_client" because no row exists in
            // `oauth_clients` for `client_id="reality-portal"`. Idempotent
            // — re-runs UPSERT.
            if let Some(secret) = &cli.reality_portal_secret {
                let redirect_uris: Vec<String> = if cli.reality_portal_redirect_uris.is_empty() {
                    DEFAULT_REALITY_PORTAL_REDIRECT_URIS
                        .iter()
                        .map(|s| s.to_string())
                        .collect()
                } else {
                    cli.reality_portal_redirect_uris.clone()
                };
                println!("Seeding `reality-portal` OAuth client...");
                println!("  redirect_uris: {:?}", redirect_uris);
                upsert_reality_portal_client(runner.pool(), secret, &redirect_uris).await?;
                println!("  ✓ reality-portal client UPSERT complete");
                println!();
            }

            Ok(())
        }
        Err(SeedError::AlreadySeeded) => {
            eprintln!();
            eprintln!("╔══════════════════════════════════════════════════════════╗");
            eprintln!("║                    Seed Skipped                          ║");
            eprintln!("╠══════════════════════════════════════════════════════════╣");
            eprintln!("║  Seed data already exists in the database.               ║");
            eprintln!("║                                                          ║");
            eprintln!("║  Use --force to drop existing seed data and re-seed.     ║");
            eprintln!("╚══════════════════════════════════════════════════════════╝");
            eprintln!();
            Err(anyhow::anyhow!("Seed data already exists"))
        }
        Err(e) => {
            eprintln!();
            eprintln!("╔══════════════════════════════════════════════════════════╗");
            eprintln!("║                    Seed Failed ✗                         ║");
            eprintln!("╠══════════════════════════════════════════════════════════╣");
            eprintln!(
                "║  Error: {:<47} ║",
                e.to_string().chars().take(47).collect::<String>()
            );
            eprintln!("╚══════════════════════════════════════════════════════════╝");
            eprintln!();
            Err(anyhow::anyhow!("Seed failed: {}", e))
        }
    }
}
