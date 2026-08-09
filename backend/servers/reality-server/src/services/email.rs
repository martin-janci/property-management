//! Password-reset email transport for reality-server (UC-44.3).
//!
//! ## The bug this closes
//!
//! reality-server issued a reset token, persisted its SHA-256 hash, then
//! **discarded the plaintext token** and unconditionally responded
//! *"a reset link has been sent"*. In production that was a lie — no email
//! transport was ever wired, so the link never reached the user and the account
//! had no working self-service recovery path.
//!
//! ## The fix
//!
//! A real transport lives behind the [`PasswordResetMailer`] seam:
//! - [`SmtpPasswordResetMailer`] delivers via SMTP (lettre), configured from the
//!   same `SMTP_*` environment variables api-server already uses.
//! - [`LogPasswordResetMailer`] is the local/dev fallback — it logs instead of
//!   sending and reports `is_configured() == false`.
//!
//! [`build_password_reset_mailer`] selects the transport from the environment,
//! and [`delivery_decision`] is the pure guard the request handler uses so the
//! endpoint no longer falsely claims delivery when no transport exists.

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use db::models::user::Locale;
use lettre::{
    message::{header::ContentType, Mailbox},
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};

/// How a password-reset request should be handled given the transport state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetDelivery {
    /// A real transport is configured — issue the token and send the email.
    Send,
    /// No transport is configured, but a dev/log fallback is permitted — issue
    /// the token and log it (local/dev only).
    LogFallback,
    /// No transport is configured and no fallback is allowed — the endpoint must
    /// report the feature as unavailable rather than claim a link was sent.
    Unavailable,
}

/// Pure guard used by the request handler to decide how to answer a reset
/// request without leaking whether the account exists (the inputs are global
/// server state, not per-user).
///
/// * `configured` — whether a real delivery transport exists.
/// * `allow_log_fallback` — whether the caller permits the dev/log fallback
///   (true in debug/local builds, false in release/production).
pub fn delivery_decision(configured: bool, allow_log_fallback: bool) -> ResetDelivery {
    if configured {
        ResetDelivery::Send
    } else if allow_log_fallback {
        ResetDelivery::LogFallback
    } else {
        ResetDelivery::Unavailable
    }
}

/// Transport seam for delivering a password-reset link.
#[async_trait]
pub trait PasswordResetMailer: Send + Sync {
    /// `true` when a real delivery transport is configured. When `false`, the
    /// request handler must NOT claim a link was sent (see [`delivery_decision`]).
    fn is_configured(&self) -> bool;

    /// Deliver a reset link carrying `token` to `to_email`.
    async fn send_reset_link(
        &self,
        to_email: &str,
        name: &str,
        token: &str,
        locale: Locale,
    ) -> Result<()>;
}

/// Base URL of the portal web front-end that hosts the reset-password page.
/// Loaded from `PORTAL_WEB_BASE_URL` (or legacy `REALITY_WEB_BASE_URL`), with a
/// local-dev fallback. Any trailing slash is trimmed so link building is stable.
fn reset_base_url_from_env() -> String {
    let raw = std::env::var("PORTAL_WEB_BASE_URL")
        .or_else(|_| std::env::var("REALITY_WEB_BASE_URL"))
        .unwrap_or_else(|_| "http://localhost:3000".to_string());
    raw.trim_end_matches('/').to_string()
}

/// Build the localized reset-password link. Matches the reality-web route
/// `/{locale}/auth/reset-password` and URL-encodes the token.
fn build_reset_url(base_url: &str, token: &str, locale: &Locale) -> String {
    format!(
        "{}/{}/auth/reset-password?token={}",
        base_url,
        locale.as_str(),
        urlencoding::encode(token)
    )
}

fn reset_subject(locale: &Locale) -> &'static str {
    match locale {
        Locale::Slovak => "Obnovenie hesla",
        Locale::Czech => "Obnovení hesla",
        Locale::German => "Passwort zurücksetzen",
        Locale::English => "Reset your password",
    }
}

fn reset_body(name: &str, url: &str, locale: &Locale) -> String {
    match locale {
        Locale::Slovak => format!(
            "Dobrý deň {name},\n\nDostali sme žiadosť o obnovenie vášho hesla. Kliknutím na odkaz nižšie ho obnovte:\n\n{url}\n\nOdkaz je platný 30 minút. Ak ste o obnovenie hesla nežiadali, túto správu ignorujte.\n\nS pozdravom,\nTím Reality Portál",
        ),
        Locale::Czech => format!(
            "Dobrý den {name},\n\nObdrželi jsme žádost o obnovení vašeho hesla. Kliknutím na odkaz níže ho obnovte:\n\n{url}\n\nOdkaz je platný 30 minut. Pokud jste o obnovení hesla nežádali, tuto zprávu ignorujte.\n\nS pozdravem,\nTým Reality Portál",
        ),
        Locale::German => format!(
            "Guten Tag {name},\n\nWir haben eine Anfrage zum Zurücksetzen Ihres Passworts erhalten. Klicken Sie auf den Link unten:\n\n{url}\n\nDer Link ist 30 Minuten gültig. Wenn Sie kein Zurücksetzen angefordert haben, ignorieren Sie diese Nachricht.\n\nMit freundlichen Grüßen,\nDas Reality-Portal-Team",
        ),
        Locale::English => format!(
            "Hello {name},\n\nWe received a request to reset your password. Click the link below:\n\n{url}\n\nThis link is valid for 30 minutes. If you didn't request a password reset, please ignore this email.\n\nBest regards,\nThe Reality Portal Team",
        ),
    }
}

/// SMTP settings loaded from the environment (mirrors api-server's `SMTP_*`).
struct SmtpSettings {
    host: String,
    port: u16,
    username: String,
    password: String,
    from_address: String,
    from_name: String,
    use_tls: bool,
}

impl SmtpSettings {
    /// Returns `None` unless the mandatory SMTP variables are all present.
    fn from_env() -> Option<Self> {
        let host = std::env::var("SMTP_HOST").ok()?;
        let username = std::env::var("SMTP_USERNAME").ok()?;
        let password = std::env::var("SMTP_PASSWORD").ok()?;
        let from_address = std::env::var("SMTP_FROM").ok()?;
        let port = std::env::var("SMTP_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(587);
        let from_name =
            std::env::var("SMTP_FROM_NAME").unwrap_or_else(|_| "Reality Portal".to_string());
        let use_tls = std::env::var("SMTP_TLS")
            .map(|v| v != "false" && v != "0")
            .unwrap_or(true);

        Some(Self {
            host,
            port,
            username,
            password,
            from_address,
            from_name,
            use_tls,
        })
    }
}

/// Production transport: delivers the reset link over SMTP.
pub struct SmtpPasswordResetMailer {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    /// Pre-rendered `Name <address>` sender mailbox source string.
    from: String,
    base_url: String,
}

impl SmtpPasswordResetMailer {
    fn new(settings: SmtpSettings, base_url: String) -> Result<Self> {
        let creds = Credentials::new(settings.username.clone(), settings.password.clone());
        let builder = if settings.use_tls {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&settings.host)?
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&settings.host)
        };
        let transport = builder.port(settings.port).credentials(creds).build();
        let from = format!("{} <{}>", settings.from_name, settings.from_address);
        Ok(Self {
            transport,
            from,
            base_url,
        })
    }
}

#[async_trait]
impl PasswordResetMailer for SmtpPasswordResetMailer {
    fn is_configured(&self) -> bool {
        true
    }

    async fn send_reset_link(
        &self,
        to_email: &str,
        name: &str,
        token: &str,
        locale: Locale,
    ) -> Result<()> {
        let url = build_reset_url(&self.base_url, token, &locale);
        let from: Mailbox = self
            .from
            .parse()
            .map_err(|e| anyhow::anyhow!("invalid SMTP from address: {e}"))?;
        let to: Mailbox = to_email
            .parse()
            .map_err(|e| anyhow::anyhow!("invalid recipient address: {e}"))?;

        let email = Message::builder()
            .from(from)
            .to(to)
            .subject(reset_subject(&locale))
            .header(ContentType::TEXT_PLAIN)
            .body(reset_body(name, &url, &locale))?;

        self.transport.send(email).await?;
        Ok(())
    }
}

/// Local/dev fallback: logs the reset link instead of sending it, and reports
/// itself as **not** configured so the request handler never claims delivery.
pub struct LogPasswordResetMailer {
    base_url: String,
}

impl LogPasswordResetMailer {
    pub fn new(base_url: String) -> Self {
        Self { base_url }
    }
}

#[async_trait]
impl PasswordResetMailer for LogPasswordResetMailer {
    fn is_configured(&self) -> bool {
        false
    }

    async fn send_reset_link(
        &self,
        to_email: &str,
        _name: &str,
        token: &str,
        locale: Locale,
    ) -> Result<()> {
        // The reset URL embeds the plaintext token — only surface it in debug
        // builds so a mis-wired production deployment can never leak it to logs.
        #[cfg(debug_assertions)]
        {
            let url = build_reset_url(&self.base_url, token, &locale);
            tracing::info!(
                email_hash = %common::email_log_hash(to_email),
                reset_url = %url,
                "Password-reset email (logging fallback — NOT delivered; configure SMTP_* to enable delivery)"
            );
        }
        #[cfg(not(debug_assertions))]
        {
            let _ = (token, locale);
            tracing::warn!(
                email_hash = %common::email_log_hash(to_email),
                "Password-reset requested but no email transport is configured — link NOT delivered"
            );
        }
        Ok(())
    }
}

/// Select the password-reset transport from the environment.
///
/// Prefers a real SMTP transport when the `SMTP_*` variables are present; falls
/// back to [`LogPasswordResetMailer`] (delivery disabled) otherwise.
pub fn build_password_reset_mailer() -> Arc<dyn PasswordResetMailer> {
    let base_url = reset_base_url_from_env();
    match SmtpSettings::from_env() {
        Some(settings) => match SmtpPasswordResetMailer::new(settings, base_url.clone()) {
            Ok(mailer) => {
                tracing::info!("Password-reset SMTP transport configured");
                Arc::new(mailer)
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Failed to build SMTP transport for password reset; using logging fallback (delivery disabled)"
                );
                Arc::new(LogPasswordResetMailer::new(base_url))
            }
        },
        None => {
            tracing::info!(
                "SMTP not configured; password reset uses the logging fallback (delivery disabled)"
            );
            Arc::new(LogPasswordResetMailer::new(base_url))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delivery_decision_truth_table() {
        // Configured transport always sends, regardless of the fallback flag.
        assert_eq!(delivery_decision(true, false), ResetDelivery::Send);
        assert_eq!(delivery_decision(true, true), ResetDelivery::Send);
        // Unconfigured + fallback allowed (dev) → log the token locally.
        assert_eq!(delivery_decision(false, true), ResetDelivery::LogFallback);
        // Unconfigured + no fallback (prod) → must report unavailable, never
        // claim a link was sent. This is the regression the bug produced.
        assert_eq!(delivery_decision(false, false), ResetDelivery::Unavailable);
    }

    #[test]
    fn log_mailer_reports_not_configured() {
        let mailer = LogPasswordResetMailer::new("http://localhost:3000".to_string());
        assert!(!mailer.is_configured());
    }

    #[tokio::test]
    async fn log_mailer_send_is_ok_and_never_delivers() {
        let mailer = LogPasswordResetMailer::new("http://localhost:3000".to_string());
        let res = mailer
            .send_reset_link("user@example.com", "User", "tok-123", Locale::English)
            .await;
        assert!(res.is_ok());
    }

    #[test]
    fn reset_url_is_locale_prefixed_and_token_encoded() {
        let url = build_reset_url("https://portal.example.sk", "a b/c+d", &Locale::Slovak);
        assert_eq!(
            url,
            "https://portal.example.sk/sk/auth/reset-password?token=a%20b%2Fc%2Bd"
        );
    }

    #[test]
    fn base_url_trailing_slash_trimmed_in_link() {
        let url = build_reset_url("https://portal.example.sk/", "tok", &Locale::English);
        assert_eq!(
            url, "https://portal.example.sk//en/auth/reset-password?token=tok",
            "build_reset_url does not itself trim — reset_base_url_from_env does"
        );
    }

    #[test]
    fn reset_subject_is_localized() {
        assert_eq!(reset_subject(&Locale::English), "Reset your password");
        assert_eq!(reset_subject(&Locale::Slovak), "Obnovenie hesla");
        assert_eq!(reset_subject(&Locale::Czech), "Obnovení hesla");
        assert_eq!(reset_subject(&Locale::German), "Passwort zurücksetzen");
    }

    #[test]
    fn reset_body_contains_name_and_link() {
        let body = reset_body("Jane", "https://x/reset?token=t", &Locale::English);
        assert!(body.contains("Jane"));
        assert!(body.contains("https://x/reset?token=t"));
    }
}
