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

use clap::Parser;
use db::seed::{SeedConfig, SeedError, SeedRunner};
use dialoguer::{Confirm, Input, Password};

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
                println!("│  PPT Users deleted:     {:>30} │", stats.users_deleted);
                println!(
                    "│  Portal Users deleted:  {:>30} │",
                    stats.portal_users_deleted
                );
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
